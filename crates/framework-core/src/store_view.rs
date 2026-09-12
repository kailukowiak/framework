//! Evaluated and safe-mode projections of the same stored document.
use super::*;

impl Store {
    pub fn view(&self) -> DocumentView {
        if self.safe_mode {
            return self.view_skeleton();
        }
        let document = self.document.materialized_for_view();
        DocumentView {
            computed_frames: document.compute_frames(),
            computed_results: document.compute_results(),
            computed_blocks: document.compute_blocks(),
            computed_texts: document.compute_texts(),
            computed_calculation_matrices: document.compute_calculation_matrices(),
            computed_values: document.compute_values(),
            computed_models: document.compute_models(),
            parameter_inputs: document.parameter_inputs(),
            document,
            formula_functions: formula_function_catalog(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            safe_mode: false,
        }
    }

    /// The document structure with nothing evaluated: no derivation is
    /// materialized, no formula resolved, no source read. Every card renders
    /// from its stored shape and its computed body is simply absent, which is
    /// what lets a poisoned document open far enough to be repaired. The
    /// counterpart guards on ingest keep the empty maps from being refilled
    /// behind the interface's back.
    fn view_skeleton(&self) -> DocumentView {
        DocumentView {
            document: self.document.clone(),
            computed_frames: HashMap::new(),
            computed_results: HashMap::new(),
            computed_blocks: HashMap::new(),
            computed_texts: HashMap::new(),
            computed_calculation_matrices: HashMap::new(),
            computed_values: HashMap::new(),
            computed_models: HashMap::new(),
            parameter_inputs: Vec::new(),
            formula_functions: formula_function_catalog(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            safe_mode: true,
        }
    }
}
