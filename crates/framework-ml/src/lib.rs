//! FrameWork's reusable numerical model runtime. No document IDs or UI state.
mod error;
mod types;
pub use error::MlError;
pub use types::*;
pub mod xgboost;
pub type Result<T> = std::result::Result<T, MlError>;
mod exact;
mod fit;
mod logistic_backend;
mod logistic_error;
mod metrics;
mod ols_summary;
mod prediction;
mod separation;
pub use fit::fit;
pub use prediction::{evaluate, output_names, predict, predict_columns, validate_model};

pub fn import_xgboost(bytes: &[u8]) -> Result<FittedModel> {
    let booster = xgboost::Model::from_json(bytes).map_err(MlError::InvalidModel)?;
    let model = FittedModel {
        version: 1,
        feature_names: booster.feature_names().to_vec(),
        target_name: None,
        payload: ModelPayload::Xgboost {
            model: Box::new(booster),
        },
        summary: ModelSummary {
            coefficients: vec![],
            training_metrics: Default::default(),
            observations: None,
            covariance: None,
            confidence_level: None,
            residual_degrees_of_freedom: None,
            warnings: vec!["Imported model: training statistics are unavailable".into()],
        },
        training_baseline: None,
        training_class_probabilities: None,
    };
    validate_model(&model)?;
    Ok(model)
}

pub use prediction::validate_model as validate_fitted;
pub use xgboost::IterationRange;
pub use xgboost::Settings as XgboostSettings;

pub fn import_xgboost_with_range(
    bytes: &[u8],
    range: Option<IterationRange>,
) -> Result<FittedModel> {
    let mut fitted = import_xgboost(bytes)?;
    if let (ModelPayload::Xgboost { model }, Some(range)) = (&mut fitted.payload, range) {
        model
            .set_prediction_range(range)
            .map_err(MlError::InvalidModel)?;
    }
    Ok(fitted)
}
mod statistics;
pub use statistics::{StatsMethod, StatsRequest, StatsResult, statistics};
pub mod onnx;
pub mod pipeline_types;

pub mod forest;
pub use forest::{ForestModel, ForestSettings};

mod typed_prediction;
pub use typed_prediction::{
    import_onnx, input_types, output_types, predict_scalars, predict_scalars as predict_values,
};
