//! Older baked inputs can carry their former recipe separately from the
//! visible saved result. It has no active dependency until a person supplies
//! a replacement source.
use crate::{Column, ConnectorRecipe, FrameStep};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisconnectedRead {
    pub source: String,
    pub connector: Option<ConnectorRecipe>,
    pub columns: Vec<Column>,
    pub base_columns: Vec<Column>,
    pub steps: Vec<FrameStep>,
}

impl crate::FrameObject {
    pub(crate) fn render_disconnected_steps(
        &self,
        document: &crate::Document,
    ) -> Vec<crate::RenderedFrameStep> {
        let Some(recipe) = &self.disconnected_read else {
            return Vec::new();
        };
        let mut original = self.clone();
        original.columns = recipe.columns.clone();
        original.base_columns = recipe.base_columns.clone();
        original.render_steps(document, original.input_columns(), &recipe.steps)
    }
}
