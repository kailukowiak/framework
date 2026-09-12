//! The serialized document projection shared by the desktop and MCP.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DocumentView {
    #[serde(flatten)]
    pub document: Document,
    pub computed_frames: HashMap<Id, ComputedFrame>,
    pub computed_results: HashMap<Id, ComputedResult>,
    pub computed_blocks: HashMap<Id, ComputedBlock>,
    pub computed_texts: HashMap<Id, ComputedText>,
    pub computed_calculation_matrices: HashMap<Id, ComputedCalculationMatrix>,
    /// What each value object holds right now, once the active scenario has
    /// had its say. Keyed by value id.
    ///
    /// A separate map rather than a rewritten `raw` on the object itself:
    /// the card has to be able to show the effective number *and* say that
    /// it is not the one stored, and an interface handed only the override
    /// could not tell an assumption apart from an edit.
    pub computed_values: HashMap<Id, ComputedValue>,
    pub computed_models: HashMap<Id, ComputedModel>,
    /// UI projections of named constructor formulas, never independent state.
    #[serde(default)]
    #[ts(optional, as = "Option<Vec<ParameterInput>>")]
    pub parameter_inputs: Vec<ParameterInput>,
    pub formula_functions: Vec<FormulaFunction>,
    pub can_undo: bool,
    pub can_redo: bool,
    /// True when the document was opened without evaluation, so the interface
    /// can say so and offer to turn it back on. The computed maps above are
    /// empty in that state, not merely not-yet-filled.
    pub safe_mode: bool,
}
