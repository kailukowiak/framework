//! Import semantics that must survive lowering from an external model format.
//! Numeric labels are not strings, missing sentinels are not interchangeable,
//! and boosting iteration ranges count rounds rather than individual trees.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NumericPrecision {
    Float32,
    Float64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ClassLabel {
    Integer(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum MissingValue {
    #[serde(rename = "nan")]
    NaN,
    Null,
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IterationRange {
    pub start: u32,
    pub end: u32,
}

impl IterationRange {
    /// End is exclusive. No magical `(0, 0)` spelling for all trees travels
    /// with the fitted model: an explicit range has one meaning on every engine.
    pub fn validate(&self, total_rounds: u32) -> Result<(), String> {
        if self.start >= self.end || self.end > total_rounds {
            return Err("iteration range must be nonempty and within saved rounds".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PredictionPolicy {
    pub precision: NumericPrecision,
    pub iteration_range: Option<IterationRange>,
    pub threshold: Option<f64>,
}

impl PredictionPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(threshold) = self.threshold
            && (!threshold.is_finite() || !(0.0..=1.0).contains(&threshold))
        {
            return Err("classification threshold must be finite and between zero and one".into());
        }
        if let Some(range) = self.iteration_range {
            // Only the parsed booster can supply the real upper bound.
            range.validate(u32::MAX)?;
        }
        Ok(())
    }
}
