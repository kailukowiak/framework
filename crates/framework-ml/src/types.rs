use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    Ols,
    Logistic,
    Xgboost,
    RandomForestRegressor,
    RandomForestClassifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum CovarianceMethod {
    Classical,
    Hc3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FitRequest {
    pub method: Method,
    pub feature_names: Vec<String>,
    pub target_name: String,
    pub confidence_level: f64,
    pub covariance: CovarianceMethod,
    pub max_iterations: usize,
    #[serde(default)]
    pub forest: crate::forest::ForestSettings,
    #[serde(default)]
    pub xgboost: crate::xgboost::Settings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NumericDataset {
    pub rows: Vec<Vec<f64>>,
    pub targets: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StatisticType {
    T,
    Z,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Coefficient {
    pub term: String,
    pub estimate: Option<f64>,
    pub standard_error: Option<f64>,
    pub statistic: Option<f64>,
    pub statistic_type: StatisticType,
    pub p_value: Option<f64>,
    pub confidence_lower: Option<f64>,
    pub confidence_upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub rows: usize,
    pub rmse: Option<f64>,
    pub mae: Option<f64>,
    pub r_squared: Option<f64>,
    pub accuracy: Option<f64>,
    pub log_loss: Option<f64>,
    pub baseline_rmse: Option<f64>,
    pub baseline_accuracy: Option<f64>,
    pub baseline_log_loss: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ModelSummary {
    pub coefficients: Vec<Coefficient>,
    pub training_metrics: Metrics,
    pub observations: Option<usize>,
    pub covariance: Option<CovarianceMethod>,
    pub confidence_level: Option<f64>,
    pub residual_degrees_of_freedom: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModelPayload {
    Linear {
        intercept: f64,
        coefficients: Vec<f64>,
    },
    BinaryLogistic {
        intercept: f64,
        coefficients: Vec<f64>,
    },
    Onnx {
        model: Box<crate::onnx::ImportedLinear>,
    },
    RandomForest {
        model: Box<crate::forest::ForestModel>,
    },
    Xgboost {
        model: Box<crate::xgboost::Model>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FittedModel {
    pub version: u32,
    pub feature_names: Vec<String>,
    pub target_name: Option<String>,
    pub payload: ModelPayload,
    pub summary: ModelSummary,
    /// Learned from training targets; absent for imported models. Never learned
    /// from the labels subsequently supplied to evaluate().
    pub training_baseline: Option<f64>,
    #[serde(default)]
    pub training_class_probabilities: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PredictionOutput {
    /// Numerical response for regression; class index for classification.
    pub values: Vec<f64>,
    /// Class order is 0..K. Binary models return [P(0), P(1)].
    pub probabilities: Option<Vec<Vec<f64>>>,
}
