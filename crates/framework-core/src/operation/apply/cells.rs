//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Cell and row edits.

use crate::*;

impl Document {
    pub(crate) fn apply_set_cell(
        &mut self,
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        raw: String,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let column = self
            .frame(&frame_id)?
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        validate_category_raw(column, &raw)?;
        let frame = self.frame_mut(&frame_id)?;
        if !frame.columns.iter().any(|column| column.id == column_id) {
            return Err(CoreError::ColumnNotFound);
        }
        let row = frame
            .rows
            .iter_mut()
            .find(|row| row.id == row_id)
            .ok_or(CoreError::RowNotFound)?;
        row.cells.entry(column_id).or_default().raw = raw;
        Ok(())
    }

    pub(crate) fn apply_set_cells(
        &mut self,
        frame_id: Id,
        cells: Vec<CellUpdate>,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
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
        let frame = self.frame_mut(&frame_id)?;
        for update in cells {
            let row = frame
                .rows
                .iter_mut()
                .find(|row| row.id == update.row_id)
                .ok_or(CoreError::RowNotFound)?;
            row.cells.entry(update.column_id).or_default().raw = update.raw;
        }
        Ok(())
    }

    /// Replaces an empty frame's columns and rows with pasted content.
    ///
    /// The emptiness check runs again here rather than being trusted from
    /// `prepare`: a replayed event may arrive at a frame someone has since
    /// typed into, and replacing its columns would orphan every cell.
    pub(crate) fn apply_set_frame_content(
        &mut self,
        frame_id: Id,
        columns: Vec<Column>,
        rows: Vec<Row>,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.derivation.is_some() || frame.artifact.is_some() || frame.source_file.is_some() {
            return Err(CoreError::InvalidOperation(
                "Only a frame that owns its own rows can be filled by pasting".into(),
            ));
        }
        if frame.rows.iter().any(|row| {
            row.cells
                .values()
                .any(|cell| !cell.raw.trim().is_empty() || cell.override_formula.is_some())
        }) {
            return Err(CoreError::InvalidOperation(
                "This frame already has data — paste into a cell instead".into(),
            ));
        }
        let frame = self.frame_mut(&frame_id)?;
        frame.columns = columns;
        frame.rows = rows;
        frame.summaries.clear();
        frame.unique_keys.clear();
        frame.display = FrameDisplay::default();
        Ok(())
    }

    /// Points the frame at data of its own and lets go of everything that
    /// would have overwritten it.
    ///
    /// Everything: the chain and the derivation that would recompute these
    /// values, the connector that would refresh over them, the snapshot that
    /// was a cache of them. What is left is an ordinary imported frame whose
    /// file the document wrote itself, which is exactly the shape every
    /// other path already knows how to read, page, and now edit.
    pub(crate) fn apply_adopt_frame_rows(
        &mut self,
        frame_id: Id,
        artifact: DataArtifact,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        frame.artifact = Some(artifact);
        frame.derivation = None;
        frame.steps.clear();
        frame.base_columns.clear();
        frame.connector = None;
        frame.source_file = None;
        frame.materialization = None;
        // The rows a derived frame was carrying for the view are the same
        // values, now read from the file instead.
        frame.rows.clear();
        Ok(())
    }

    /// Cuts every outside dependency at once.
    ///
    /// Dropping a connector is all it takes for a frame that already has its
    /// own copy of the data — the copy is what it reads, and the connector
    /// was only the thing that would have replaced it. A frame reading a
    /// path directly has no copy, so it is handed one.
    pub(crate) fn apply_package_document(
        &mut self,
        unlinked: Vec<Id>,
        adopted: Vec<(Id, DataArtifact)>,
    ) -> Result<(), CoreError> {
        for frame_id in unlinked {
            self.frame_mut(&frame_id)?.connector = None;
        }
        for (frame_id, artifact) in adopted {
            self.apply_adopt_frame_rows(frame_id, artifact)?;
        }
        Ok(())
    }

    /// Rewrites one cell of the parquet a frame owns.
    ///
    /// The one operation in the model that writes a file while applying.
    /// It has to: the value being changed *is* the file, and an undo has to
    /// change it back the same way. The write is a rename over a temporary,
    /// so a failure leaves the old artifact intact and the document
    /// untouched — this returns before anything in the model moves.
    pub(crate) fn apply_set_artifact_cell(
        &mut self,
        frame_id: Id,
        row_ordinal: usize,
        column_id: Id,
        raw: String,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        let column = frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        let (name, data_type) = (column.name.clone(), column.data_type);
        let artifact = frame
            .artifact
            .as_ref()
            .ok_or_else(|| CoreError::InvalidOperation("This frame has no data file".into()))?;
        let rewritten = write_artifact_cell(artifact, &name, data_type, row_ordinal, &raw)?;
        self.frame_mut(&frame_id)?.artifact = Some(rewritten);
        Ok(())
    }

    pub(crate) fn apply_paste_cells(
        &mut self,
        frame_id: Id,
        cells: Vec<CellUpdate>,
        appended_rows: Vec<Row>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        for row in appended_rows {
            let frame = self.frame(&frame_id)?;
            if frame.rows.iter().any(|existing| existing.id == row.id) {
                continue;
            }
            for column in &frame.columns {
                let raw = row
                    .cells
                    .get(&column.id)
                    .map(|cell| cell.raw.as_str())
                    .unwrap_or_default();
                validate_category_raw(column, raw)?;
            }
            self.frame_mut(&frame_id)?.rows.push(row);
        }
        self.apply_set_cells(frame_id, cells)
    }

    pub(crate) fn apply_add_row(
        &mut self,
        frame_id: Id,
        row: Row,
        after_row_id: Option<Id>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        {
            let frame = self.frame(&frame_id)?;
            for column in &frame.columns {
                let raw = row
                    .cells
                    .get(&column.id)
                    .map(|cell| cell.raw.as_str())
                    .unwrap_or_default();
                validate_category_raw(column, raw)?;
            }
        }
        let frame = self.frame_mut(&frame_id)?;
        if frame.rows.iter().any(|existing| existing.id == row.id)
            || row
                .cells
                .keys()
                .any(|column_id| !frame.columns.iter().any(|column| column.id == *column_id))
        {
            return Err(CoreError::InvalidOperation(
                "row IDs and cells must be unique and belong to the target frame".into(),
            ));
        }
        if row.cells.len() != frame.columns.len()
            || frame
                .columns
                .iter()
                .any(|column| !row.cells.contains_key(&column.id))
        {
            return Err(CoreError::InvalidOperation(
                "row must contain exactly one cell for each frame column".into(),
            ));
        }
        let insert_at = match after_row_id {
            Some(row_id) => frame
                .rows
                .iter()
                .position(|candidate| candidate.id == row_id)
                .map(|index| index + 1)
                .ok_or(CoreError::RowNotFound)?,
            None => frame.rows.len(),
        };
        frame.rows.insert(insert_at, row);
        Ok(())
    }

    pub(crate) fn apply_delete_row(&mut self, frame_id: Id, row_id: Id) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        {
            let frame = self.frame_mut(&frame_id)?;
            let row_index = frame
                .rows
                .iter()
                .position(|row| row.id == row_id)
                .ok_or(CoreError::RowNotFound)?;
            frame.rows.remove(row_index);
        }
        // A style anchored to the row that just went away has nothing left
        // to paint.
        self.frame_mut(&frame_id)?
            .display
            .styles
            .retain(|entry| match &entry.target {
                FrameStyleTarget::Row {
                    row_id: styled_row_id,
                }
                | FrameStyleTarget::Cell {
                    row_id: styled_row_id,
                    ..
                } => *styled_row_id != row_id,
                _ => true,
            });
        Ok(())
    }

    pub(crate) fn apply_add_entry_column(
        &mut self,
        frame_id: Id,
        column: Column,
        key_column_ids: Vec<Id>,
        entries: Vec<EntryValue>,
        unique_key: Option<UniqueKeyConstraint>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        let column_id = column.id.clone();
        frame.columns.push(column);
        // The key travels with the add when the frame lacked one: it was
        // minted at prepare so replicas agree, and the uniqueness of the
        // data under it is checked by the validation pass that follows
        // every apply.
        if let Some(unique_key) = unique_key {
            frame.unique_keys.push(unique_key);
        }
        frame.entry_columns.push(EntryColumn {
            column_id,
            key_column_ids,
            entries,
        });
        Ok(())
    }

    pub(crate) fn apply_remove_entry_column(
        &mut self,
        frame_id: Id,
        column_id: Id,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        frame.columns.retain(|column| column.id != column_id);
        frame
            .entry_columns
            .retain(|entry_column| entry_column.column_id != column_id);
        Ok(())
    }

    pub(crate) fn apply_set_entry_value(
        &mut self,
        frame_id: Id,
        column_id: Id,
        key: Vec<String>,
        raw: String,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        let entry_column = frame
            .entry_columns
            .iter_mut()
            .find(|entry_column| entry_column.column_id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        // Blank means "nothing entered", and nothing entered is an absent
        // entry, not an entry holding emptiness — an absent entry stops
        // shadowing a future row with the same key.
        if raw.trim().is_empty() {
            entry_column.entries.retain(|entry| entry.key != key);
            return Ok(());
        }
        match entry_column
            .entries
            .iter_mut()
            .find(|entry| entry.key == key)
        {
            Some(entry) => entry.raw = raw,
            None => entry_column.entries.push(EntryValue { key, raw }),
        }
        Ok(())
    }

    pub(crate) fn apply_set_cell_override(
        &mut self,
        frame_id: Id,
        row_id: Id,
        column_id: Id,
        formula: Option<Formula>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        self.validate_cell_override(&frame_id, &column_id, formula.as_ref())?;
        let frame = self.frame_mut(&frame_id)?;
        let row = frame
            .rows
            .iter_mut()
            .find(|row| row.id == row_id)
            .ok_or(CoreError::RowNotFound)?;
        row.cells.entry(column_id).or_default().override_formula = formula;
        Ok(())
    }
}
