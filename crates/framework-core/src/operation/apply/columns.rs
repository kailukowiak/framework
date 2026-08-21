//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Column lifecycle: adding, deleting, retyping, formatting, and the
//! formulas and summaries attached to a column.

use crate::*;

impl Document {
    pub(crate) fn apply_add_column(
        &mut self,
        frame_id: Id,
        column: Column,
        after_column_id: Option<Id>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        if let Some(formula) = &column.formula {
            let inferred_type = self
                .frame(&frame_id)?
                .infer_polars_expression_type(self, &formula.expression)
                .map_err(CoreError::Formula)?;
            if inferred_type != column.data_type {
                return Err(CoreError::InvalidOperation(
                    "formula result type does not match its replicated column type".into(),
                ));
            }
        }
        let frame = self.frame_mut(&frame_id)?;
        if frame
            .columns
            .iter()
            .any(|existing| existing.id == column.id)
        {
            return Err(CoreError::InvalidOperation(
                "column ID already exists in the target frame".into(),
            ));
        }
        let insert_at = match after_column_id {
            Some(column_id) => frame
                .columns
                .iter()
                .position(|column| column.id == column_id)
                .map(|index| index + 1)
                .ok_or(CoreError::ColumnNotFound)?,
            None => frame.columns.len(),
        };
        let column_id = column.id.clone();
        let has_formula = column.formula.is_some();
        frame.columns.insert(insert_at, column);
        for row in &mut frame.rows {
            row.cells.insert(column_id.clone(), Cell::default());
        }
        if has_formula {
            self.ensure_acyclic(&frame_id)?;
        }
        Ok(())
    }

    pub(crate) fn apply_delete_column(
        &mut self,
        frame_id: Id,
        column_id: Id,
    ) -> Result<(), CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        if !frame.columns.iter().any(|column| column.id == column_id) {
            return Err(CoreError::ColumnNotFound);
        }
        if frame.columns.len() == 1 {
            return Err(CoreError::CannotDeleteLastColumn);
        }
        let going = as_named(
            frame
                .columns
                .iter()
                .find(|column| column.id == column_id)
                .map_or("", |column| column.name.as_str()),
        );
        if frame.references_column_from_other_formulas(&column_id) {
            return Err(CoreError::ReferencedByFormula(format!(
                "Another column of {} reads {going}, so it cannot be deleted. \
                 Change the formula that reads it first.",
                as_named(&frame.name)
            )));
        }
        if frame.display.references_column(&column_id) {
            return Err(CoreError::ReferencedByFormula(format!(
                "{going} is what this frame is sorted or filtered by, so it cannot be \
                 deleted. Clear that first."
            )));
        }
        if let Some(derived) = self.objects.iter().find_map(|object| match object {
            DataObject::Frame(derived) => derived
                .wrangle_reads_foreign_column(&frame_id, &column_id)
                .then(|| as_named(&derived.name)),
            _ => None,
        }) {
            return Err(CoreError::ReferencedByFormula(format!(
                "{derived} is built from {going}, so it cannot be deleted. \
                 Change that frame's steps first."
            )));
        }
        if let Some(plot) = self.objects.iter().find_map(|object| match object {
            DataObject::Plot(plot) if plot.source_frame_id == frame_id => {
                json_contains_string(&plot.spec, &column_id).then(|| as_named(&plot.name))
            }
            _ => None,
        }) {
            return Err(CoreError::ReferencedByFormula(format!(
                "{plot} is drawn from {going}, so it cannot be deleted. \
                 Change that plot first."
            )));
        }

        let frame = self.frame_mut(&frame_id)?;
        frame.columns.retain(|column| column.id != column_id);
        for row in &mut frame.rows {
            row.cells.remove(&column_id);
        }
        frame
            .summaries
            .retain(|summary| summary.column_id != column_id);
        // A style anchored to the column that just went away has nothing
        // left to paint.
        frame.display.styles.retain(|entry| match &entry.target {
            FrameStyleTarget::Column {
                column_id: styled_column_id,
            }
            | FrameStyleTarget::Cell {
                column_id: styled_column_id,
                ..
            } => *styled_column_id != column_id,
            _ => true,
        });
        Ok(())
    }

    pub(crate) fn apply_rename_column(
        &mut self,
        frame_id: Id,
        column_id: Id,
        name: String,
    ) -> Result<(), CoreError> {
        let old_name = self
            .frame(&frame_id)?
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?
            .name
            .clone();
        let next_name = name.clone();
        let frame = self.frame_mut(&frame_id)?;
        frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?
            .name = name;
        for object in &mut self.objects {
            if let DataObject::Plot(plot) = object
                && plot.source_frame_id == frame_id
            {
                update_plot_field_titles(&mut plot.spec, &column_id, &old_name, &next_name);
            }
        }
        Ok(())
    }

    pub(crate) fn apply_set_column_type(
        &mut self,
        frame_id: Id,
        column_id: Id,
        data_type: DataType,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let frame = self.frame_mut(&frame_id)?;
        let column = frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        column.data_type = data_type;
        column.categories = if data_type == DataType::Categorical {
            distinct_category_values(column, &frame.rows)
        } else {
            Vec::new()
        };
        Ok(())
    }

    pub(crate) fn apply_set_column_categories(
        &mut self,
        frame_id: Id,
        column_id: Id,
        categories: Vec<String>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let frame = self.frame(&frame_id)?;
        let column = frame
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        let categories = normalized_categories(categories)?;
        validate_category_values(column, &frame.rows, &categories)?;
        let frame = self.frame_mut(&frame_id)?;
        let column = frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        column.data_type = DataType::Categorical;
        column.categories = categories;
        Ok(())
    }

    pub(crate) fn apply_set_column_format(
        &mut self,
        frame_id: Id,
        column_id: Id,
        format: Option<ColumnFormat>,
    ) -> Result<(), CoreError> {
        // Formatting is display-only metadata, so derived output
        // columns may carry it even though their data is read-only.
        let frame = self.frame_mut(&frame_id)?;
        let column = frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        column.format = format;
        Ok(())
    }

    pub(crate) fn apply_set_column_formula(
        &mut self,
        frame_id: Id,
        column_id: Id,
        formula: Formula,
        data_type: DataType,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.is_computed() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let inferred_type = self
            .frame(&frame_id)?
            .infer_polars_expression_type(self, &formula.expression)
            .map_err(CoreError::Formula)?;
        if inferred_type != data_type {
            return Err(CoreError::InvalidOperation(
                "formula result type does not match its replicated type".into(),
            ));
        }
        let frame = self.frame_mut(&frame_id)?;
        let column = frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        column.data_type = data_type;
        column.categories.clear();
        column.formula = Some(formula);
        self.ensure_acyclic(&frame_id)?;
        Ok(())
    }

    pub(crate) fn apply_add_summary(
        &mut self,
        frame_id: Id,
        summary: Summary,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        if !frame
            .columns
            .iter()
            .any(|column| column.id == summary.column_id)
        {
            return Err(CoreError::ColumnNotFound);
        }
        if frame
            .summaries
            .iter()
            .any(|existing| existing.id == summary.id)
        {
            return Err(CoreError::InvalidOperation(
                "summary ID already exists in the target frame".into(),
            ));
        }
        frame.summaries.push(summary);
        Ok(())
    }
}
