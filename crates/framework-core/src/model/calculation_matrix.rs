use crate::formula::ast::Formula;
use crate::{DataType, Id};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A compact two-axis authoring surface.  It is deliberately not a frame:
/// the axes and shared body are the authored facts; the long result is a
/// live projection rebuilt from them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalculationMatrixObject {
    pub id: Id,
    pub name: String,
    pub rows: Vec<CalculationMatrixAxisFormula>,
    pub columns: Vec<CalculationMatrixAxisFormula>,
    pub body: CalculationMatrixBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalculationMatrixAxisFormula {
    pub id: Id,
    /// Explicit model input varied by this axis. Names remain local aliases;
    /// matching a name must never silently change a model's dependencies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub target_id: Option<Id>,
    pub name: String,
    pub source: String,
    pub formula: Option<Formula>,
    pub data_type: DataType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalculationMatrixBody {
    pub source: String,
    pub formula: Option<Formula>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}
