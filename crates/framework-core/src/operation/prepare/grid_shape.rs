//! Resolving the edits that change a grid's shape: a row placed between two
//! others, several rows deleted at once, and a paste wide enough to need
//! columns the frame does not have yet.
//!
//! Each one has two answers depending on where the frame keeps its rows. A
//! frame that holds them can be reshaped like a spreadsheet; a frame read from
//! a file is edited through patches against that file, which can strike rows
//! out and add them past its end but cannot put one between two rows the file
//! already orders, nor give the file a column it does not have.

use crate::*;
use std::collections::HashSet;

impl Document {
    /// Refuses a positioned insert where a position cannot be honoured.
    ///
    /// Rows added to a frame read from a file are patches past the file's
    /// end — ordinals continue where the file stops — so "above row 12" has
    /// nowhere to go. Saying so is better than appending and leaving the
    /// person to find their row at the bottom.
    pub(super) fn ensure_row_position_is_addressable(
        &self,
        frame: &FrameObject,
        before_row_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let Some(before_row_id) = before_row_id else {
            return Ok(());
        };
        if !frame.preserves_own_row_identity() {
            if self.frame_rows_are_editable(&frame.id) {
                return Err(CoreError::InvalidOperation(
                    "Rows in a table read from a file are added at the end, not between \
                     existing rows."
                        .into(),
                ));
            }
            // Not editable at all: the ordinary refusal below names why.
            return Ok(());
        }
        if !frame.rows.iter().any(|row| row.id == before_row_id) {
            return Err(CoreError::RowNotFound);
        }
        Ok(())
    }

    /// Resolves a multi-row delete.
    ///
    /// The ids arrive the way a selection gathers them — in click order,
    /// possibly repeated — and leave unique and in the frame's own order, so
    /// every replica removes the same set and the inverse can be read off it.
    /// On a frame read from a file every row is struck out in one patch-list
    /// write, exactly as `DeleteRow` strikes out one.
    pub(crate) fn prepare_delete_rows(
        &self,
        frame_id: Id,
        row_ids: Vec<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        if row_ids.is_empty() {
            return Err(CoreError::InvalidOperation(
                "No rows were chosen to delete".into(),
            ));
        }
        let frame = self.frame(&frame_id)?;
        if !frame.preserves_own_row_identity() {
            if !self.frame_rows_are_editable(&frame_id) {
                // The single-row refusal says why in the frame's own terms.
                return self.prepare_delete_row(frame_id, row_ids[0].clone());
            }
            let mut deleted_rows = frame.deleted_rows.clone();
            for row_id in &row_ids {
                let ordinal = Self::artifact_row_ordinal(&frame_id, row_id)
                    .and_then(|ordinal| u32::try_from(ordinal).ok())
                    .ok_or(CoreError::RowNotFound)?;
                if let Err(insert_at) = deleted_rows.binary_search(&ordinal) {
                    deleted_rows.insert(insert_at, ordinal);
                }
            }
            return Ok(ReplicatedOperation::SetRowPatches {
                frame_id,
                deleted_rows,
                appended_rows: frame.appended_rows,
            });
        }
        let wanted: HashSet<&str> = row_ids.iter().map(String::as_str).collect();
        let row_ids: Vec<Id> = frame
            .rows
            .iter()
            .filter(|row| wanted.contains(row.id.as_str()))
            .map(|row| row.id.clone())
            .collect();
        if row_ids.len() != wanted.len() {
            return Err(CoreError::RowNotFound);
        }
        Ok(ReplicatedOperation::DeleteRows { frame_id, row_ids })
    }

    /// Refuses a column-lifecycle edit aimed at a calculated column of a
    /// frame's own chain.
    ///
    /// Once a frame that owns its columns may carry a chain, its visible
    /// columns are of two kinds: literal ones the chain reads, and the outputs
    /// of its calculations. Only the first kind is this edit's to delete or
    /// retype — an output belongs to its formula, and removing it from the
    /// visible list would leave the step still producing it.
    pub(super) fn ensure_not_a_chain_output(
        &self,
        frame_id: &str,
        column_id: &str,
    ) -> Result<(), CoreError> {
        let frame = self.frame(frame_id)?;
        let is_output = !frame.steps.is_empty()
            && frame.owns_its_columns()
            && frame.columns.iter().any(|column| column.id == column_id)
            && !frame.column_is_stored_input(column_id);
        if is_output {
            return Err(CoreError::InvalidOperation(
                "This column is calculated by Wrangle. Change or remove it in the chain instead."
                    .into(),
            ));
        }
        Ok(())
    }

    /// The columns a paste needs beyond the frame's right-hand edge, or the
    /// whole-paste refusal when this frame cannot grow them.
    ///
    /// A frame the document owns, whose chain only adds calculations, grows
    /// the way a spreadsheet does: the missing columns are named the way the
    /// app names a new column and typed from the pasted values by the reader
    /// a paste into an empty frame uses. Anything else keeps the refusal, for
    /// the reason the paste has always given — a frame whose schema comes
    /// from a file or a reshape cannot be handed a column by a clipboard.
    pub(super) fn columns_a_paste_adds(
        &self,
        frame: &FrameObject,
        first_column: usize,
        grid: &[Vec<String>],
    ) -> Result<Vec<Column>, CoreError> {
        let width = grid.iter().map(Vec::len).max().unwrap_or(0);
        let fit = frame.columns.len() - first_column;
        if width <= fit {
            return Ok(Vec::new());
        }
        if !(frame.owns_its_columns() && frame.preserves_own_row_identity()) {
            return Err(CoreError::InvalidOperation(format!(
                "Nothing pasted: the clipboard needs {width} columns, but only {fit} fit here. Add columns first, or paste onto empty canvas to create a new table."
            )));
        }
        let mut taken: HashSet<String> = frame
            .columns
            .iter()
            .chain(frame.base_columns.iter())
            .map(|column| column.name.to_lowercase())
            .collect();
        Ok((fit..width)
            .map(|offset| {
                let name = next_new_column_name(&taken);
                taken.insert(name.to_lowercase());
                let values: Vec<&str> = grid
                    .iter()
                    .map(|row| row.get(offset).map_or("", String::as_str))
                    .collect();
                let (data_type, scale) = pasted_values_type(&values);
                Column {
                    id: column_id(&name),
                    name,
                    source_name: None,
                    data_type,
                    scale,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                }
            })
            .collect())
    }
}

impl FrameObject {
    /// This frame as it will stand once a paste's new columns are on, for
    /// resolving the paste's cells against. Rows are left as they are; the
    /// apply gives each existing row its blank cell for every new column.
    pub(super) fn with_appended_columns(&self, added: &[Column]) -> FrameObject {
        let mut widened = self.clone();
        for column in added {
            if !widened.base_columns.is_empty() {
                widened.base_columns.push(column.clone());
            }
            widened.columns.push(column.clone());
        }
        widened
    }
}

/// "New column", then "New column 2", "New column 3" — the name the grid's
/// own *Add column* gives (`nextColumnName` in `src/FrameGrid.tsx`), compared
/// without regard to case the same way, so a pasted column and a clicked one
/// are named by one rule.
fn next_new_column_name(taken: &HashSet<String>) -> String {
    if !taken.contains("new column") {
        return "New column".into();
    }
    let mut suffix = 2;
    while taken.contains(&format!("new column {suffix}")) {
        suffix += 1;
    }
    format!("New column {suffix}")
}

/// The type one pasted column's values read as, by the same reader a paste
/// into an empty frame uses. Text when that reader cannot say, or when any
/// value would not be accepted by the type it proposed — a column whose own
/// paste it refuses is worse than a text column.
fn pasted_values_type(values: &[&str]) -> (DataType, Option<u8>) {
    let fallback = (DataType::String, None);
    if values.iter().all(|value| value.trim().is_empty()) {
        return fallback;
    }
    let text = std::iter::once("value")
        .chain(values.iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(read) = read_pasted_frame(&text) else {
        return fallback;
    };
    let (columns, _) = Document::frame_content_from_frame(&read);
    let [column] = columns.as_slice() else {
        return fallback;
    };
    if values
        .iter()
        .any(|value| validate_cell_raw(column, value).is_err())
    {
        return fallback;
    }
    (column.data_type, column.scale)
}
