use crate::{
    Coefficient, CovarianceMethod, FitRequest, FittedModel, Method, MlError, ModelPayload,
    ModelSummary, NumericDataset, Result, StatisticType,
};
use anofox_regression::solvers::{FittedRegressor, OlsRegressor, Regressor};
use faer::{Col, Mat};

pub fn fit(request: &FitRequest, data: &NumericDataset) -> Result<FittedModel> {
    validate(request, data)?;
    let (payload, mut summary) = match request.method {
        Method::Ols => ols(request, data)?,
        Method::Logistic => logistic(request, data)?,
        Method::RandomForestRegressor | Method::RandomForestClassifier => {
            let forest = crate::forest::train(
                data,
                &request.forest,
                request.method == Method::RandomForestClassifier,
            )?;
            let mut summary = empty_summary(request);
            summary.covariance = None;
            summary.confidence_level = None;
            if request.method == Method::RandomForestClassifier {
                summary.warnings.push(
                    "Class probabilities are tree-vote fractions, not calibrated estimates".into(),
                );
            }
            (
                ModelPayload::RandomForest {
                    model: Box::new(forest),
                },
                summary,
            )
        }
        Method::Xgboost => {
            let booster = crate::xgboost::train(data, &request.feature_names, &request.xgboost)?;
            let mut summary = empty_summary(request);
            summary.covariance = None;
            summary.confidence_level = None;
            if booster.objective() != crate::xgboost::Objective::Regression {
                summary.warnings.push(
                    "Class probabilities are native XGBoost estimates and are not calibrated"
                        .into(),
                );
            }
            (
                ModelPayload::Xgboost {
                    model: Box::new(booster),
                },
                summary,
            )
        }
    };
    summary.observations = Some(data.rows.len());
    let mut model = FittedModel {
        version: 1,
        feature_names: request.feature_names.clone(),
        target_name: Some(request.target_name.clone()),
        payload,
        summary,
        training_baseline: Some(crate::metrics::mean(&data.targets)),
        training_class_probabilities: None,
    };
    if let Some(classes) = crate::prediction::classification(&model) {
        let mut frequencies = vec![0.; classes];
        for &label in &data.targets {
            frequencies[label as usize] += 1.;
        }
        for value in &mut frequencies {
            *value /= data.targets.len() as f64;
        }
        model.training_class_probabilities = Some(frequencies);
        if classes > 2 {
            model.training_baseline = None;
        }
    }
    model.summary.training_metrics = crate::evaluate(&model, data)?;
    Ok(model)
}

fn validate(request: &FitRequest, data: &NumericDataset) -> Result<()> {
    let p = request.feature_names.len();
    if p == 0
        || data.rows.len() != data.targets.len()
        || data.rows.len() < 2
        || (matches!(request.method, Method::Ols | Method::Logistic) && data.rows.len() <= p + 1)
        || data.rows.iter().any(|r| r.len() != p)
    {
        return Err(MlError::InvalidInput("Choose predictors and a target with matching rows and more observations than fitted parameters".into()));
    }
    if data.rows.len().saturating_mul(p) > 5_000_000 {
        return Err(MlError::ResourceLimit(
            "The training matrix exceeds the current five-million-cell fitting budget".into(),
        ));
    }
    if data
        .rows
        .iter()
        .flatten()
        .chain(&data.targets)
        .any(|v| !v.is_finite())
    {
        return Err(MlError::InvalidInput(
            "Training needs finite numbers; fill or remove missing values upstream before fitting"
                .into(),
        ));
    }
    if !request.confidence_level.is_finite()
        || request.confidence_level <= 0.
        || request.confidence_level >= 1.
        || request.max_iterations == 0
        || request.max_iterations > 10000
    {
        return Err(MlError::InvalidInput(
            "Choose a confidence level between 0 and 1 and an iteration limit from 1 to 10000"
                .into(),
        ));
    }
    Ok(())
}

fn empty_summary(request: &FitRequest) -> ModelSummary {
    ModelSummary {
        coefficients: vec![],
        training_metrics: Default::default(),
        observations: None,
        covariance: Some(request.covariance),
        confidence_level: Some(request.confidence_level),
        residual_degrees_of_freedom: None,
        warnings: vec![],
    }
}

fn ols(request: &FitRequest, data: &NumericDataset) -> Result<(ModelPayload, ModelSummary)> {
    let x = Mat::from_fn(data.rows.len(), request.feature_names.len(), |i, j| {
        data.rows[i][j]
    });
    let y = Col::from_fn(data.targets.len(), |i| data.targets[i]);
    let fitted = OlsRegressor::builder()
        .with_intercept(true)
        .confidence_level(request.confidence_level)
        .build()
        .fit(&x, &y)
        .map_err(|e| MlError::Numerical(e.to_string()))?;
    let result = fitted.result();
    let mut summary = empty_summary(request);
    summary.residual_degrees_of_freedom = Some(result.residual_df());
    let robust = if request.covariance == CovarianceMethod::Hc3 {
        Some(
            anofox_regression::inference::compute_hc_inference(
                &x,
                &result.coefficients,
                result.intercept,
                &result.residuals,
                &result.aliased,
                true,
                anofox_regression::inference::HcType::HC3,
                request.confidence_level,
            )
            .map_err(|e| MlError::Numerical(e.into()))?,
        )
    } else {
        None
    };
    for index in 0..=request.feature_names.len() {
        summary.coefficients.push(crate::ols_summary::coefficient(
            request,
            result,
            robust.as_ref(),
            index,
        ));
    }
    if result.aliased.iter().any(|a| *a) {
        summary.warnings.push("Linearly dependent predictors have unavailable coefficients; predictions use the identifiable fit".into());
    }
    let intercept = result
        .intercept
        .ok_or_else(|| MlError::Numerical("The fit did not produce an intercept".into()))?;
    let coefficients = result
        .coefficients
        .iter()
        .enumerate()
        .map(|(i, v)| if result.aliased[i] { 0. } else { *v })
        .collect();
    Ok((
        ModelPayload::Linear {
            intercept,
            coefficients,
        },
        summary,
    ))
}

fn logistic(request: &FitRequest, data: &NumericDataset) -> Result<(ModelPayload, ModelSummary)> {
    if request.covariance != CovarianceMethod::Classical {
        return Err(MlError::Unsupported(
            "Logistic regression currently supports model-based standard errors only".into(),
        ));
    }
    let fitted = crate::logistic_backend::fit(
        &data.rows,
        &data.targets,
        crate::logistic_backend::Settings {
            confidence_level: request.confidence_level,
            max_iterations: request.max_iterations,
            ..Default::default()
        },
    )
    .map_err(crate::logistic_error::convert)?;
    let mut summary = empty_summary(request);
    for (i, term) in fitted.coefficients.iter().enumerate() {
        summary.coefficients.push(Coefficient {
            term: if i == 0 {
                "Intercept".into()
            } else {
                request.feature_names[i - 1].clone()
            },
            estimate: Some(term.estimate),
            standard_error: Some(term.standard_error),
            statistic: Some(term.z_statistic),
            statistic_type: StatisticType::Z,
            p_value: Some(term.p_value),
            confidence_lower: Some(term.confidence_interval[0]),
            confidence_upper: Some(term.confidence_interval[1]),
        });
    }
    Ok((
        ModelPayload::BinaryLogistic {
            intercept: fitted.coefficients[0].estimate,
            coefficients: fitted
                .coefficients
                .iter()
                .skip(1)
                .map(|c| c.estimate)
                .collect(),
        },
        summary,
    ))
}
