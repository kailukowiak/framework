use crate::pipeline_types::{ClassLabel, Dtype, Scalar};
use crate::{Coefficient, FittedModel, MlError, ModelPayload, ModelSummary, Result, StatisticType};

pub fn input_types(model: &FittedModel) -> Vec<Dtype> {
    match &model.payload {
        ModelPayload::Onnx { model } => model.inputs().iter().map(|v| v.dtype).collect(),
        _ => vec![Dtype::Number; model.feature_names.len()],
    }
}
pub fn output_types(model: &FittedModel) -> Vec<Dtype> {
    let mut result = vec![Dtype::Number; crate::output_names(model).len()];
    if let ModelPayload::Onnx { model } = &model.payload
        && matches!(model.labels().first(), Some(ClassLabel::String(_)))
    {
        result[0] = Dtype::String;
    }
    result
}

pub fn predict_scalars(model: &FittedModel, rows: &[Vec<Scalar>]) -> Result<Vec<Vec<Scalar>>> {
    crate::validate_model(model)?;
    if let ModelPayload::Onnx { model: pipeline } = &model.payload {
        if rows.len() > 10_000 {
            return Err(MlError::ResourceLimit(
                "This ONNX pipeline currently supports at most 10000 scoring rows per call".into(),
            ));
        }
        return pipeline
            .predict(rows)
            .map_err(|e| MlError::InvalidInput(e.to_string()))?
            .into_iter()
            .map(|prediction| {
                Ok(match prediction {
                    crate::onnx::Prediction::Regression(value) => {
                        vec![Scalar::Number(f64::from(value))]
                    }
                    crate::onnx::Prediction::Classification {
                        label,
                        probabilities,
                    } => vec![
                        scalar_label(label)?,
                        Scalar::Number(f64::from(probabilities[0])),
                        Scalar::Number(f64::from(probabilities[1])),
                    ],
                })
            })
            .collect();
    }
    let numeric: Vec<Vec<f64>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| match v {
                    Scalar::Number(value) => Ok(*value),
                    _ => Err(MlError::InvalidInput(
                        "This model requires numeric predictors".into(),
                    )),
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<_>>()?;
    Ok(crate::predict_columns(model, &numeric)?
        .into_iter()
        .map(|row| row.into_iter().map(Scalar::Number).collect())
        .collect())
}

fn scalar_label(label: ClassLabel) -> Result<Scalar> {
    match label {
        ClassLabel::String(label) => Ok(Scalar::String(label)),
        ClassLabel::Integer(label) if label.unsigned_abs() <= 9_007_199_254_740_991 => {
            Ok(Scalar::Number(label as f64))
        }
        ClassLabel::Integer(_) => Err(MlError::Unsupported(
            "This integer class label cannot be represented exactly in FrameWork's numeric cells"
                .into(),
        )),
    }
}

pub fn import_onnx(bytes: &[u8]) -> Result<FittedModel> {
    let pipeline = crate::onnx::import(bytes).map_err(|e| MlError::InvalidModel(e.to_string()))?;
    for label in pipeline.labels() {
        scalar_label(label.clone())?;
    }
    let classification = !pipeline.labels().is_empty();
    let width = pipeline.recipe().features.len();
    let index = usize::from(classification);
    let mut coefficients = Vec::new();
    for term in 0..=width {
        let estimate = if term == 0 {
            pipeline.intercepts()[index]
        } else {
            pipeline.coefficients()[index * width + term - 1]
        };
        coefficients.push(Coefficient {
            term: if term == 0 {
                "Intercept".into()
            } else {
                pipeline.recipe().features[term - 1].clone()
            },
            estimate: Some(f64::from(estimate)),
            standard_error: None,
            statistic: None,
            statistic_type: if classification {
                StatisticType::Z
            } else {
                StatisticType::T
            },
            p_value: None,
            confidence_lower: None,
            confidence_upper: None,
        });
    }
    let mut warnings = vec![
        "Imported fitted preprocessing and coefficients; training uncertainty is unavailable"
            .into(),
    ];
    if classification {
        warnings.push(format!(
            "Coefficients describe log odds for class {}",
            match &pipeline.labels()[1] {
                ClassLabel::String(v) => v.clone(),
                ClassLabel::Integer(v) => v.to_string(),
            }
        ));
    }
    let model = FittedModel {
        version: 1,
        feature_names: pipeline.inputs().iter().map(|v| v.name.clone()).collect(),
        target_name: None,
        payload: ModelPayload::Onnx {
            model: Box::new(pipeline),
        },
        summary: ModelSummary {
            coefficients,
            training_metrics: Default::default(),
            observations: None,
            covariance: None,
            confidence_level: None,
            residual_degrees_of_freedom: None,
            warnings,
        },
        training_baseline: None,
        training_class_probabilities: None,
    };
    crate::validate_model(&model)?;
    Ok(model)
}
