use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub slot: String,
    pub name: String,
    pub dtype: Dtype,
    pub role: Role,
    pub binding: Option<ColumnBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColumnBinding {
    pub frame_id: String,
    pub column_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Dtype {
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Predictor,
    Outcome,
    Id,
    Weight,
    Group,
    Time,
}

/// Steps are in topological order, while named slots permit independent
/// branches. `features` is the final matrix order, not the order inputs happened
/// to arrive in. This is sufficient to describe the first ColumnTransformer
/// examples without inventing a separate mutable frame for each fitted step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recipe {
    pub precision: NumericPrecision,
    pub steps: Vec<Step>,
    pub features: Vec<String>,
}

/// `deny_unknown_fields` is deliberately absent here even though the structs
/// in this file carry it. ts-rs parses that serde attribute on structs but
/// not on enums, so keeping it warns on every build while changing nothing
/// in the generated TypeScript. Each variant still requires its own fields,
/// so a malformed step is refused all the same; only an extra unknown key
/// inside a known step is now ignored rather than rejected, which is the
/// forgiving direction for recipes persisted in documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Step {
    Impute {
        input: String,
        output: String,
        strategy: Imputation,
        missing: MissingValue,
        learned: Option<Scalar>,
    },
    Scale {
        input: String,
        output: String,
        learned: Option<ScaleParameters>,
    },
    OneHot {
        input: String,
        outputs: Vec<String>,
        unknown: UnknownCategory,
        learned: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Imputation {
    Mean,
    MostFrequent,
    /// Imported formats may retain the replacement value without recording
    /// how it was estimated. Applying that constant must not invent a claim
    /// that it was a training mean or most-frequent category.
    Constant,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum UnknownCategory {
    Error,
    AllZero,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Scalar {
    Number(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaleParameters {
    pub offset: f64,
    /// Retain the original operator as well as its parameter. Replacing an
    /// ONNX f32 multiplier by its reciprocal divisor can change rounding; the
    /// model should not change arithmetic merely to normalize a field name.
    pub scale: f64,
    pub operation: ScaleOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ScaleOperation {
    Divide,
    Multiply,
}

// Import semantics that must survive lowering from an external model format.
// Numeric labels are not strings, missing sentinels are not interchangeable,
// and boosting iteration ranges count rounds rather than individual trees.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum NumericPrecision {
    Float32,
    Float64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ClassLabel {
    Integer(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum MissingValue {
    #[serde(rename = "nan")]
    NaN,
    Null,
    String(String),
    Number(f64),
}
