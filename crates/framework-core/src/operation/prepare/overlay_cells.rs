//! Bulk cell edits on a frame whose rows live in a base it reads.
//!
//! The range gestures — clearing a selection, a fill, a cut, a pasted block —
//! arrive as `SetCells` and `PasteCells`, the same operations they are on a
//! table the document holds. On a file-backed frame they become patches
//! against the base read instead, for the reason `prepare_set_cell` gives for
//! the single cell: the interface should not have to know where a frame keeps
//! its values in order to set them. Before this, the bulk forms looked each row
//! up among the document's own rows, which such a frame has none of, so a
//! table accepted one typed value and refused the same edit made twenty at a
//! time.

use crate::*;
use std::collections::BTreeSet;

impl Document {
    /// The ordinal a page row id names, checked against the rows the page
    /// could actually have shown.
    ///
    /// The id is minted from the same index the patches are keyed by — one
    /// ordinal space, before any sort or filter moved a row — so parsing it is
    /// all the addressing there is. The checks are what keep a stale id from
    /// recording a patch nobody can see: past the end of the base and its
    /// added rows there is no row, and a struck-out row is not on the page.
    pub(super) fn overlay_row_ordinal(frame: &FrameObject, row_id: &str) -> Result<u32, CoreError> {
        let ordinal = Self::artifact_row_ordinal(&frame.id, row_id)
            .and_then(|ordinal| u32::try_from(ordinal).ok())
            .ok_or(CoreError::RowNotFound)?;
        if ordinal >= Self::overlay_row_count(frame)? || frame.deleted_rows.contains(&ordinal) {
            return Err(CoreError::RowNotFound);
        }
        Ok(ordinal)
    }

    /// How many ordinals the base read and its added rows span together,
    /// struck-out rows included — they keep their ordinals, which is what
    /// lets every other patch keep its address.
    fn overlay_row_count(frame: &FrameObject) -> Result<u32, CoreError> {
        let base = frame
            .artifact
            .as_ref()
            .ok_or(CoreError::RowNotFound)?
            .row_count;
        u32::try_from(base)
            .ok()
            .and_then(|base| base.checked_add(frame.appended_rows))
            .ok_or_else(|| {
                CoreError::InvalidOperation(
                    "This table has more rows than a cell edit can address".into(),
                )
            })
    }

    /// `SetCells` on a frame whose rows are read rather than held.
    ///
    /// Every cell is checked before any is recorded, the same as on a table
    /// the document holds: a range edit that wrote the cells it could and
    /// dropped the rest would look like it had worked.
    pub(super) fn prepare_set_overlay_cells(
        &self,
        frame_id: Id,
        cells: Vec<CellUpdate>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let mut patches = Vec::with_capacity(cells.len());
        for update in cells {
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == update.column_id)
                .ok_or(CoreError::ColumnNotFound)?;
            self.ensure_column_is_editable(&frame_id, &update.column_id)?;
            validate_cell_raw(column, &update.raw)?;
            patches.push(OverlayCellPatch {
                row_ordinal: Self::overlay_row_ordinal(frame, &update.row_id)?,
                column_id: update.column_id,
                raw: Some(update.raw),
            });
        }
        Ok(ReplicatedOperation::SetOverlayCells {
            frame_id,
            appended_rows: None,
            cells: patches,
        })
    }

    /// `PasteCells` on a frame whose rows are read rather than held.
    ///
    /// The block runs down from the anchor through the rows the frame holds,
    /// in the order it holds them, as it does on a literal table — stepping
    /// over struck-out rows, which are no longer part of it. Rows it runs past
    /// the end are added the way `prepare_add_row` adds one here: by raising
    /// the count past the base's end, where every value is already a patch.
    /// That keeps the paste, growth included, one edit and one undo.
    ///
    /// The caller has already checked the rectangle fits the columns and
    /// touches no calculated one; `first_column` is where it starts.
    pub(super) fn prepare_paste_overlay_cells(
        &self,
        frame_id: Id,
        row_id: &str,
        first_column: usize,
        grid: Vec<Vec<String>>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let anchor = Self::overlay_row_ordinal(frame, row_id)?;
        let row_count = Self::overlay_row_count(frame)?;
        let deleted: BTreeSet<u32> = frame.deleted_rows.iter().copied().collect();
        let mut ordinal = anchor;
        let mut patches = Vec::new();
        for (offset, values) in grid.into_iter().enumerate() {
            if offset > 0 {
                ordinal = ordinal.checked_add(1).ok_or(CoreError::RowNotFound)?;
            }
            // Struck-out rows sit only inside the existing span, so this
            // never steps over a row the paste itself is adding.
            while deleted.contains(&ordinal) {
                ordinal += 1;
            }
            for (column_offset, raw) in values.into_iter().enumerate() {
                let column = &frame.columns[first_column + column_offset];
                validate_cell_raw(column, &raw)?;
                patches.push(OverlayCellPatch {
                    row_ordinal: ordinal,
                    column_id: column.id.clone(),
                    raw: Some(raw),
                });
            }
        }
        let grown = ordinal
            .checked_add(1)
            .filter(|end| *end > row_count)
            .map(|end| frame.appended_rows + (end - row_count));
        Ok(ReplicatedOperation::SetOverlayCells {
            frame_id,
            appended_rows: grown,
            cells: patches,
        })
    }
}
