//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to column IDs, and
//! every precondition checked before anything is applied.
//!
//! Cell and row edits.

use crate::*;
use std::collections::{BTreeMap, HashSet};

impl Document {
    /// Refuses an edit to a frame whose values come from somewhere else.
    ///
    /// Not a formality. The read path takes an imported frame's values from
    /// its artifact and a derived frame's from its chain, so a raw value
    /// written into either is kept in the document and never shown again —
    /// an edit that appears to work, does nothing, and leaves the document
    /// holding a number nobody will ever see. Saying no is the smaller
    /// surprise, and it names the way through.
    ///
    /// Applies to the `prepare` side only, which is where a person's edit
    /// enters. Replicated operations from a peer or from an undo inverse are
    /// already-decided facts and are applied as given.
    fn ensure_rows_are_editable(&self, frame_id: &str) -> Result<(), CoreError> {
        if self.frame_cells_are_editable(frame_id) {
            return Ok(());
        }
        let frame = self.frame(frame_id)?;
        Err(CoreError::InvalidOperation(
            if frame.derivation.is_some() || !frame.steps.is_empty() {
                "These rows are computed by the chain above them. Edit the chain, or take \
                 ownership of the result to edit it directly."
            } else {
                "These rows are read from a source that a refresh would replace. Take \
                 ownership of them to make them the document's own."
            }
            .into(),
        ))
    }

    /// The ordinal a page row id names, for a frame read from a parquet.
    ///
    /// `page_row_ids` writes these, and they are the only identity such a
    /// row has: `source:<frame>:<n>`, where `n` is the row's position in the
    /// data layer — before any display sort or filter moved it around.
    fn artifact_row_ordinal(frame_id: &str, row_id: &str) -> Option<usize> {
        row_id
            .strip_prefix(&format!("source:{frame_id}:"))
            .and_then(|ordinal| ordinal.parse().ok())
    }

    pub(crate) fn prepare_set_cell(
        &self,
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        raw: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        // A frame whose values live in a parquet it owns is edited by
        // rewriting that parquet. Same gesture in the grid, different write
        // underneath, which is why the branch is here rather than in the
        // interface: the interface should not have to know where a frame
        // keeps its values in order to set one.
        if !self.frame(&frame_id)?.owns_its_rows() {
            self.ensure_rows_are_editable(&frame_id)?;
            let frame = self.frame(&frame_id)?;
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .ok_or(CoreError::ColumnNotFound)?;
            validate_category_raw(column, &raw)?;
            // Parsed here so a bad value is refused before anything is
            // written, with the message that names the type it wanted.
            parse_scalar_value(&raw, column.data_type).map_err(CoreError::Import)?;
            let row_ordinal =
                Self::artifact_row_ordinal(&frame_id, &row_id).ok_or(CoreError::RowNotFound)?;
            return Ok(ReplicatedOperation::SetArtifactCell {
                frame_id,
                row_ordinal,
                column_id,
                raw,
            });
        }
        Ok({
            self.ensure_rows_are_editable(&frame_id)?;
            let frame = self.frame(&frame_id)?;
            let column = frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .ok_or(CoreError::ColumnNotFound)?;
            validate_category_raw(column, &raw)?;
            ReplicatedOperation::SetCell {
                frame_id,
                row_id,
                column_id,
                raw,
            }
        })
    }

    pub(crate) fn prepare_set_cells(
        &self,
        frame_id: Id,
        cells: Vec<CellUpdate>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            self.ensure_rows_are_editable(&frame_id)?;
            let frame = self.frame(&frame_id)?;
            for update in &cells {
                if !frame.rows.iter().any(|row| row.id == update.row_id) {
                    return Err(CoreError::RowNotFound);
                }
                let column = frame
                    .columns
                    .iter()
                    .find(|column| column.id == update.column_id)
                    .ok_or(CoreError::ColumnNotFound)?;
                validate_category_raw(column, &update.raw)?;
            }
            ReplicatedOperation::SetCells { frame_id, cells }
        })
    }

    /// Whether a frame is one that pasting may rebuild outright.
    ///
    /// Only a literal frame with nothing in it. A frame that already holds
    /// values has something to lose, and an imported or derived one does not
    /// own its columns in the first place — its schema comes from the file
    /// or the transformation above it.
    fn empty_literal_frame(&self, frame_id: &str) -> Result<&FrameObject, CoreError> {
        let frame = self.frame(frame_id)?;
        if !frame.owns_its_rows() {
            return Err(CoreError::InvalidOperation(
                "Only a frame that owns its own rows can be filled by pasting".into(),
            ));
        }
        let has_values = frame.rows.iter().any(|row| {
            row.cells
                .values()
                .any(|cell| !cell.raw.trim().is_empty() || cell.override_formula.is_some())
        });
        if has_values || frame.columns.iter().any(|column| column.formula.is_some()) {
            return Err(CoreError::InvalidOperation(
                "This frame already has data — paste into a cell instead".into(),
            ));
        }
        Ok(frame)
    }

    pub(crate) fn prepare_set_frame_from_pasted_text(
        &self,
        frame_id: Id,
        text: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.empty_literal_frame(&frame_id)?;
        let frame = read_pasted_frame(&text)?;
        let (columns, rows) = Self::frame_content_from_frame(&frame);
        if columns.is_empty() {
            return Err(CoreError::Import("There is nothing to paste".into()));
        }
        Ok(ReplicatedOperation::SetFrameContent {
            frame_id,
            columns,
            rows,
        })
    }

    /// Resolves a paste into the cells it writes plus the rows it has to add.
    ///
    /// Columns are clipped to the frame's width — widening a frame is a
    /// schema change, and a paste that silently added columns would be one
    /// made by accident. Rows are not clipped: running past the last row is
    /// the ordinary way a paste arrives.
    pub(crate) fn prepare_paste_cells(
        &self,
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        grid: Vec<Vec<String>>,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.ensure_rows_are_editable(&frame_id)?;
        let frame = self.frame(&frame_id)?;
        let first_row = frame
            .rows
            .iter()
            .position(|row| row.id == row_id)
            .ok_or(CoreError::RowNotFound)?;
        let first_column = frame
            .columns
            .iter()
            .position(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;

        let mut cells = Vec::new();
        let mut appended_rows: Vec<Row> = Vec::new();
        for (offset, values) in grid.iter().enumerate() {
            let row_index = first_row + offset;
            for (column_offset, raw) in values.iter().enumerate() {
                let Some(column) = frame.columns.get(first_column + column_offset) else {
                    break;
                };
                if column.formula.is_some() {
                    continue;
                }
                validate_category_raw(column, raw)?;
                match frame.rows.get(row_index) {
                    Some(row) => cells.push(CellUpdate {
                        row_id: row.id.clone(),
                        column_id: column.id.clone(),
                        raw: raw.clone(),
                    }),
                    None => {
                        while appended_rows.len() <= row_index - frame.rows.len() {
                            appended_rows.push(Row {
                                id: id(),
                                cells: frame
                                    .columns
                                    .iter()
                                    .map(|column| (column.id.clone(), Cell::default()))
                                    .collect(),
                            });
                        }
                        if let Some(row) = appended_rows.get_mut(row_index - frame.rows.len()) {
                            row.cells.insert(
                                column.id.clone(),
                                Cell {
                                    raw: raw.clone(),
                                    ..Cell::default()
                                },
                            );
                        }
                    }
                }
            }
        }
        Ok(ReplicatedOperation::PasteCells {
            frame_id,
            cells,
            appended_rows,
        })
    }

    pub(crate) fn prepare_add_row(
        &self,
        frame_id: Id,
        values: BTreeMap<Id, String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            let frame = self.frame(&frame_id)?;
            // Rows added to a frame that reads from somewhere else are not
            // rejected by the read path, they are ignored by it: the rows
            // come from the artifact or the transformation, and the added
            // one sits in the document producing nothing. Refusing says so.
            if !frame.owns_its_rows() {
                return Err(CoreError::InvalidOperation(
                    "This frame's rows come from its source, so a row cannot be added to it here"
                        .into(),
                ));
            }
            if values
                .keys()
                .any(|column_id| !frame.columns.iter().any(|column| column.id == *column_id))
            {
                return Err(CoreError::ColumnNotFound);
            }
            for column in &frame.columns {
                validate_category_raw(
                    column,
                    values
                        .get(&column.id)
                        .map(String::as_str)
                        .unwrap_or_default(),
                )?;
            }
            ReplicatedOperation::AddRow {
                frame_id,
                after_row_id: frame.rows.last().map(|row| row.id.clone()),
                row: Row {
                    id: id(),
                    cells: frame
                        .columns
                        .iter()
                        .map(|column| {
                            (
                                column.id.clone(),
                                Cell {
                                    raw: values.get(&column.id).cloned().unwrap_or_default(),
                                    ..Cell::default()
                                },
                            )
                        })
                        .collect(),
                },
            }
        })
    }

    pub(crate) fn prepare_delete_row(
        &self,
        frame_id: Id,
        row_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        if !self.frame(&frame_id)?.owns_its_rows() {
            return Err(CoreError::InvalidOperation(
                "This frame's rows come from its source. Filter them out in the chain instead"
                    .into(),
            ));
        }
        Ok(ReplicatedOperation::DeleteRow { frame_id, row_id })
    }

    /// Checks that there is something to take ownership *of*.
    ///
    /// A frame that already owns its rows is not refused for tidiness: the
    /// action would silently rewrite its identity — replacing rows the
    /// document holds with a parquet — for no gain at all.
    pub(crate) fn prepare_adopt_frame_rows(
        &self,
        frame_id: Id,
        artifact: DataArtifact,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.owns_its_rows() && frame.steps.is_empty() {
            return Err(CoreError::InvalidOperation(
                "This frame's rows are already the document's own".into(),
            ));
        }
        Ok(ReplicatedOperation::AdoptFrameRows { frame_id, artifact })
    }

    /// Works out what packaging this document would actually cut.
    ///
    /// The connectors come from the document; the artifacts for frames that
    /// were reading a path directly come from the caller, since writing them
    /// is file work. A document with nothing to cut is refused rather than
    /// recorded as an edit that changed nothing.
    pub(crate) fn prepare_package_document(
        &self,
        adopted: Vec<(Id, DataArtifact)>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let unlinked = self
            .objects
            .iter()
            .filter_map(|object| match object {
                DataObject::Frame(frame) if frame.connector.is_some() => Some(frame.id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (frame_id, _) in &adopted {
            self.frame(frame_id)?;
        }
        if unlinked.is_empty() && adopted.is_empty() {
            return Err(CoreError::InvalidOperation(
                "This document already depends on nothing outside it".into(),
            ));
        }
        Ok(ReplicatedOperation::PackageDocument { unlinked, adopted })
    }

    /// Checks an entry column can mean something before it exists: the
    /// frame's rows must be computed (a frame someone can type into needs no
    /// key-addressed storage), and the key must be an enforced unique key so
    /// each entry lands on exactly one row.
    pub(crate) fn prepare_add_entry_column(
        &self,
        frame_id: Id,
        name: String,
        data_type: DataType,
        key_column_ids: Vec<Id>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        if !frame.is_computed() {
            return Err(CoreError::InvalidOperation(
                "This frame's cells can be typed into directly; an entry column is for \
                 frames whose rows are computed."
                    .into(),
            ));
        }
        if key_column_ids.is_empty() {
            return Err(CoreError::InvalidOperation(
                "An entry column needs at least one key column to address its rows".into(),
            ));
        }
        for key_column_id in &key_column_ids {
            if !frame
                .columns
                .iter()
                .any(|column| column.id == *key_column_id)
            {
                return Err(CoreError::ColumnNotFound);
            }
        }
        // An entry column needs its key columns enforced unique, so each
        // entry lands on exactly one row — but "set the key first" was a
        // second trip nobody benefits from. The key is part of what adding
        // the column *means*, so a missing one is minted here and carried
        // with the operation; the post-apply validation still refuses the
        // whole edit if the data holds duplicates.
        let keyed = frame.unique_keys.iter().any(|unique_key| {
            unique_key.column_ids.len() == key_column_ids.len()
                && key_column_ids
                    .iter()
                    .all(|key| unique_key.column_ids.contains(key))
        });
        let unique_key = (!keyed).then(|| UniqueKeyConstraint {
            id: id(),
            column_ids: key_column_ids.clone(),
        });
        Ok(ReplicatedOperation::AddEntryColumn {
            frame_id,
            column: Column {
                id: column_id(&name),
                name,
                source_name: None,
                data_type,
                categories: Vec::new(),
                format: None,
                formula: None,
            },
            key_column_ids,
            entries: Vec::new(),
            unique_key,
        })
    }

    pub(crate) fn prepare_set_entry_value(
        &self,
        frame_id: Id,
        column_id: Id,
        key: Vec<String>,
        raw: String,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let entry_column = frame
            .entry_columns
            .iter()
            .find(|entry_column| entry_column.column_id == column_id)
            .ok_or_else(|| {
                CoreError::InvalidOperation(
                    "Only an entry column stores values by key. Use setCell elsewhere.".into(),
                )
            })?;
        if key.len() != entry_column.key_column_ids.len() {
            return Err(CoreError::InvalidOperation(format!(
                "This entry column is keyed by {} column{}, so the key needs that many values",
                entry_column.key_column_ids.len(),
                if entry_column.key_column_ids.len() == 1 {
                    ""
                } else {
                    "s"
                }
            )));
        }
        Ok(ReplicatedOperation::SetEntryValue {
            frame_id,
            column_id,
            key,
            raw,
        })
    }

    pub(crate) fn prepare_set_cell_override(
        &self,
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        formula: Option<String>,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            // An override is recorded against a row of the document, so a
            // frame read a page at a time has nowhere to put one: its rows
            // are scanned from a file and the override would never be
            // consulted. Accepting it would be the same silent nothing as a
            // typed value on an imported frame.
            if self.frame_depends_on_artifact(&frame_id, &mut HashSet::new()) {
                return Err(CoreError::InvalidOperation(
                    "This frame is read a page at a time from its source, so a single cell \
                     cannot carry an override. Change the column's formula instead."
                        .into(),
                ));
            }
            let formula = formula
                .as_deref()
                .map(|source| self.prepare_formula_for_frame(&frame_id, source))
                .transpose()?
                .map(|expression| Formula { expression });
            self.validate_cell_override(&frame_id, &column_id, formula.as_ref())?;
            ReplicatedOperation::SetCellOverride {
                frame_id,
                row_id,
                column_id,
                formula,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn literal_rows_columns_and_column_types_are_typed_operations() {
        let mut store = demo_store();
        let frame = store
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame),
                _ => None,
            })
            .unwrap();
        let frame_id = frame.id.clone();
        let first_column_id = frame.columns[0].id.clone();
        let original_rows = frame.rows.len();

        let view = store
            .apply(Operation::AddRow {
                frame_id: frame_id.clone(),
                values: BTreeMap::from([(first_column_id.clone(), "9".into())]),
            })
            .unwrap();
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .unwrap();
        assert_eq!(frame.rows.len(), original_rows + 1);
        assert_eq!(frame.rows.last().unwrap().cells[&first_column_id].raw, "9");

        let view = store
            .apply(Operation::AddColumn {
                frame_id: frame_id.clone(),
                name: "Discount".into(),
                data_type: DataType::Currency,
                after_column_id: Some(first_column_id.clone()),
            })
            .unwrap();
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .unwrap();
        let new_column = &frame.columns[1];
        assert_eq!(new_column.name, "Discount");
        assert_eq!(new_column.data_type, DataType::Currency);
        assert!(
            frame
                .rows
                .iter()
                .all(|row| row.cells.contains_key(&new_column.id))
        );
        let new_column_id = new_column.id.clone();

        let view = store
            .apply(Operation::SetColumnType {
                frame_id: frame_id.clone(),
                column_id: new_column_id.clone(),
                data_type: DataType::Percentage,
            })
            .unwrap();
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.id == frame_id => Some(frame),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            frame
                .columns
                .iter()
                .find(|column| column.id == new_column_id)
                .unwrap()
                .data_type,
            DataType::Percentage
        );
    }
}
