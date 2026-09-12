use crate::{FittedModel, MlError, ModelPayload, NumericDataset, PredictionOutput, Result};

pub(crate) fn classification(model: &FittedModel) -> Option<usize> {
    match &model.payload {
        ModelPayload::Linear { .. } => None,
        ModelPayload::Onnx { model } => (!model.labels().is_empty()).then_some(2),
        ModelPayload::RandomForest { model } => model.classes(),
        ModelPayload::BinaryLogistic { .. } => Some(2),
        ModelPayload::Xgboost { model } => match model.objective() {
            crate::xgboost::Objective::Regression => None,
            crate::xgboost::Objective::Binary => Some(2),
            crate::xgboost::Objective::Multiclass => Some(model.groups()),
        },
    }
}

pub fn output_names(model: &FittedModel) -> Vec<String> {
    if let ModelPayload::Onnx { model } = &model.payload
        && !model.labels().is_empty()
    {
        return std::iter::once("Predicted class".into())
            .chain(model.labels().iter().map(|label| {
                format!(
                    "P({})",
                    match label {
                        crate::pipeline_types::ClassLabel::Integer(v) => v.to_string(),
                        crate::pipeline_types::ClassLabel::String(v) => v.clone(),
                    }
                )
            }))
            .collect();
    }
    match classification(model) {
        None => vec!["Prediction".into()],
        Some(classes) => std::iter::once("Predicted class".into())
            .chain((0..classes).map(|i| format!("P({i})")))
            .collect(),
    }
}

pub fn predict_columns(model: &FittedModel, rows: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let result = predict(model, rows)?;
    Ok(result
        .values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            std::iter::once(value)
                .chain(
                    result
                        .probabilities
                        .as_ref()
                        .map_or(&[][..], |p| &p[i])
                        .iter()
                        .copied(),
                )
                .collect()
        })
        .collect())
}

pub fn validate_model(model: &FittedModel) -> Result<()> {
    if model.version != 1 || model.feature_names.is_empty() {
        return Err(MlError::InvalidModel(
            "Unsupported model version or empty feature schema".into(),
        ));
    }
    match &model.payload {
        ModelPayload::Linear {
            intercept,
            coefficients,
        }
        | ModelPayload::BinaryLogistic {
            intercept,
            coefficients,
        } => {
            if !intercept.is_finite()
                || coefficients.len() != model.feature_names.len()
                || coefficients.iter().any(|v| !v.is_finite())
            {
                return Err(MlError::InvalidModel(
                    "Invalid stored linear coefficients".into(),
                ));
            }
        }
        ModelPayload::Onnx { model: pipeline } => {
            pipeline
                .validate()
                .map_err(|e| MlError::InvalidModel(e.to_string()))?;
            if pipeline.inputs().len() != model.feature_names.len() {
                return Err(MlError::InvalidModel(
                    "Stored pipeline input schema mismatch".into(),
                ));
            }
        }
        ModelPayload::RandomForest { model: forest } => {
            forest.validate(model.feature_names.len())?
        }
        ModelPayload::Xgboost { model: booster } => {
            booster.validate().map_err(MlError::InvalidModel)?;
            if booster.feature_names().len() != model.feature_names.len() {
                return Err(MlError::InvalidModel(
                    "Imported feature schema mismatch".into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn predict(model: &FittedModel, rows: &[Vec<f64>]) -> Result<PredictionOutput> {
    validate_model(model)?;
    let classes = classification(model);
    let mut result = PredictionOutput {
        values: Vec::with_capacity(rows.len()),
        probabilities: classes.map(|_| Vec::with_capacity(rows.len())),
    };
    for row in rows {
        if row.len() != model.feature_names.len() {
            return Err(MlError::InvalidInput(
                "Prediction feature count does not match the fitted model".into(),
            ));
        }
        let scores = score_row(model, row)?;
        if let Some(probabilities) = &mut result.probabilities {
            let class = scores
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(i, _)| i)
                .unwrap_or(0);
            result.values.push(class as f64);
            probabilities.push(scores);
        } else {
            result.values.push(scores[0]);
        }
    }
    Ok(result)
}

fn score_row(model: &FittedModel, row: &[f64]) -> Result<Vec<f64>> {
    match &model.payload {
        ModelPayload::Linear {
            intercept,
            coefficients,
        }
        | ModelPayload::BinaryLogistic {
            intercept,
            coefficients,
        } => {
            if row.iter().any(|v| !v.is_finite()) {
                return Err(MlError::InvalidInput("This model requires finite predictors; fill or remove missing values before scoring".into()));
            }
            let value = intercept
                + row
                    .iter()
                    .zip(coefficients)
                    .map(|(x, b)| x * b)
                    .sum::<f64>();
            if !value.is_finite() {
                return Err(MlError::Numerical("Prediction overflowed".into()));
            }
            if matches!(model.payload, ModelPayload::BinaryLogistic { .. }) {
                let p = if value >= 0. {
                    1. / (1. + (-value).exp())
                } else {
                    let e = value.exp();
                    e / (1. + e)
                };
                Ok(vec![1. - p, p])
            } else {
                Ok(vec![value])
            }
        }
        ModelPayload::Onnx { .. } => Err(MlError::InvalidInput(
            "Use typed predictor values for an imported ONNX pipeline".into(),
        )),
        ModelPayload::RandomForest { model: forest } => forest.score(row),
        ModelPayload::Xgboost { model: booster } => {
            let mut scores: Vec<f64> = booster
                .predict(
                    &row.iter()
                        .map(|v| if v.is_nan() { None } else { Some(*v) })
                        .collect::<Vec<_>>(),
                    None,
                )
                .map_err(MlError::InvalidInput)?
                .into_iter()
                .map(f64::from)
                .collect();
            if matches!(booster.objective(), crate::xgboost::Objective::Binary) {
                scores.insert(0, 1. - scores[0]);
            }
            Ok(scores)
        }
    }
}

pub fn evaluate(model: &FittedModel, data: &NumericDataset) -> Result<crate::Metrics> {
    if data.rows.is_empty()
        || data.rows.len() != data.targets.len()
        || data.targets.iter().any(|v| !v.is_finite())
    {
        return Err(MlError::InvalidInput(
            "Evaluation requires aligned rows and finite targets".into(),
        ));
    }
    let predictions = predict(model, &data.rows)?;
    crate::metrics::calculate(
        &predictions,
        &data.targets,
        model.training_baseline,
        model.training_class_probabilities.as_deref(),
    )
}
