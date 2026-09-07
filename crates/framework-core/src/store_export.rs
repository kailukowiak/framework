//! Delivery queries over the same data and display layers used by the grid.
use crate::*;
use std::collections::HashSet;
use std::path::Path;

impl Store {
    /// Write a frame's materialized values to `path` as CSV.
    ///
    /// Derived frames evaluate through the same recursive Polars plan the
    /// canvas uses, and values stay raw — ISO dates and plain numbers rather
    /// than display formatting.
    pub fn export_frame_csv(&self, frame_id: &str, path: &Path) -> Result<(), CoreError> {
        self.document().export_frame_csv(frame_id, path)
    }

    /// Write selected frames and every named scalar answer to an Excel
    /// workbook. This is a values-only handoff: FrameWork remains the place
    /// where formulas live, while the workbook receives their current answers.
    pub fn export_excel(
        &self,
        frame_ids: &[Id],
        path: &Path,
        include_lineage: bool,
    ) -> Result<(), CoreError> {
        self.document()
            .export_excel(frame_ids, path, include_lineage)
    }

    /// Export the current filtered/sorted rows when requested. This does not
    /// change downstream calculations or pretend to export canvas layout.
    pub fn export_excel_scoped(
        &self,
        frame_ids: &[Id],
        path: &Path,
        include_lineage: bool,
        current_view: bool,
    ) -> Result<(), CoreError> {
        self.document()
            .export_excel_scoped(frame_ids, path, include_lineage, current_view)
    }

    /// Exact row counts for the export scope, computed only when the export
    /// dialog asks. Metadata alone may describe the unfiltered artifact.
    pub fn export_row_count(&self, frame_id: &str, current_view: bool) -> Result<usize, CoreError> {
        let layer = if current_view {
            Layer::Display
        } else {
            Layer::Data
        };
        let plan = self
            .document()
            .materialize_frame_lazy(frame_id, layer, &mut HashSet::new())
            .map_err(CoreError::Export)?;
        let count = plan
            .select([polars::prelude::len()])
            .collect()
            .map_err(|error| CoreError::Export(error.to_string()))?;
        Ok(count
            .column("len")
            .map_err(|error| CoreError::Export(error.to_string()))?
            .u32()
            .map_err(|error| CoreError::Export(error.to_string()))?
            .get(0)
            .unwrap_or(0) as usize)
    }
}
