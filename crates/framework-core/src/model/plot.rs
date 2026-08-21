use crate::Id;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlotObject {
    pub id: Id,
    pub name: String,
    pub source_frame_id: Id,
    #[ts(type = "Record<string, unknown>")]
    pub spec: serde_json::Value,
}
