//! A model is an explicitly fitted object. Data edits change its predictions,
//! never its learned parameters; another fit is a recorded document operation.
use crate::{CoreError, DataObject, Document, Id};
use framework_ml::{CovarianceMethod, FittedModel, Method};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelInput {
    pub source_frame_id: Id,
    pub feature_column_ids: Vec<Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelSpec {
    pub source_frame_id: Id,
    pub target_column_id: Option<Id>,
    pub feature_column_ids: Vec<Id>,
    pub method: Method,
    pub covariance: CovarianceMethod,
    pub confidence_level: f64,
    pub holdout_fraction: f64,
    pub seed: u32,
    #[serde(default)]
    #[ts(optional, as = "Option<framework_ml::ForestSettings>")]
    pub forest: framework_ml::ForestSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelFit {
    pub id: Id,
    #[ts(as = "u32")]
    pub training_revision: u64,
    pub training_fingerprint: Option<String>,
    /// The recipe that produced this fit, which may differ from the draft
    /// specification after editing. Failed retraining never replaces this.
    pub spec: Option<ModelSpec>,
    pub result: FittedModel,
    pub evaluation_metrics: Option<framework_ml::Metrics>,
    pub split: Option<ModelSplit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelObject {
    pub id: Id,
    pub name: String,
    pub spec: Option<ModelSpec>,
    pub fitted: Option<ModelFit>,
    /// A default scoring binding, not training provenance. Changing scoring
    /// rows cannot make an imported fitted model stale.
    pub imported_input: Option<ModelInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelPrediction {
    pub model_id: Id,
    pub feature_column_ids: Vec<Id>,
    /// Stable output addresses survive retraining and source renaming.
    pub output_column_ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ComputedModel {
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}

impl Document {
    pub fn model(&self, model_id: &str) -> Result<&ModelObject, CoreError> {
        match self.object(model_id)? {
            DataObject::Model(model) => Ok(model),
            _ => Err(CoreError::InvalidOperation("That is not a model".into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelSplit {
    pub seed: u32,
    pub training_rows: Vec<u32>,
    pub evaluation_rows: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModelSummaryKind {
    Coefficients,
    TrainingMetrics,
    EvaluationMetrics,
}
