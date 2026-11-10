//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Column lifecycle: adding, deleting, retyping, formatting, and the
//! formulas and summaries attached to a column.

use super::grid_shape::ColumnPlace;
use crate::*;
use std::collections::BTreeMap;

impl Document {
    pub(crate) fn apply_add_column(
        &mut self,
        frame_id: Id,
        column: Column,
        place: ColumnPlace,
        values: BTreeMap<Id, String>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.column_schema_is_read_only() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        // Every row is checked before the column goes in, so a replica that
        // lost a row to a peer refuses the whole edit instead of adding the
        // column with only some of its values.
        {
            let frame = self.frame(&frame_id)?;
            if !values.is_empty() && !frame.owns_its_rows() {
                return Err(CoreError::DerivedFrameReadOnly);
            }
            if values
                .keys()
                .any(|row_id| !frame.rows.iter().any(|row| row.id == *row_id))
            {
                return Err(CoreError::RowNotFound);
            }
        }
        if let Some(formula) = &column.formula {
            let inferred_type = self
                .frame(&frame_id)?
                .inferred_column_type(self, &formula.expression)
                .map_err(CoreError::Formula)?;
            if inferred_type != column.data_type {
                return Err(CoreError::InvalidOperation(
                    "formula result type does not match its replicated column type".into(),
                ));
            }
        }
        let has_formula = column.formula.is_some();
        let column_id = column.id.clone();
        self.insert_literal_column(&frame_id, column, &place)?;
        if !values.is_empty() {
            for row in &mut self.frame_mut(&frame_id)?.rows {
                if let Some(raw) = values.get(&row.id) {
                    row.cells.entry(column_id.clone()).or_default().raw = raw.clone();
                }
            }
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
        if frame.column_schema_is_read_only() {
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
        if frame.own_chain_reads_column(&column_id) {
            return Err(CoreError::ReferencedByFormula(format!(
                "A step in the Wrangle chain of {} reads {going}, so it cannot be deleted. \
                 Change that step first.",
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
        // A scored frame carries this column and will drop it too; what it
        // shows sorted or filtered by the column has to be cleared first,
        // the same as here.
        if let Some(scored) = self
            .scored_frames(&frame_id)
            .find(|scored| scored.display.references_column(&column_id))
            .map(|scored| as_named(&scored.name))
        {
            return Err(CoreError::ReferencedByFormula(format!(
                "{scored} is sorted or filtered by {going}, so it cannot be deleted. \
                 Clear that first."
            )));
        }

        let frame = self.frame_mut(&frame_id)?;
        frame.columns.retain(|column| column.id != column_id);
        // A literal column of a frame whose chain only adds calculations is
        // also one of the inputs that chain reads; leaving it there would
        // keep a field in the base scan that nothing shows. Only that case:
        // an imported frame's input schema is the file's, not this edit's.
        if frame.owns_its_columns() {
            frame.base_columns.retain(|column| column.id != column_id);
        }
        for row in &mut frame.rows {
            row.cells.remove(&column_id);
        }
        frame
            .summaries
            .retain(|summary| summary.column_id != column_id);
        // A style or a width anchored to the column that just went away has
        // nothing left to paint.
        frame
            .display
            .forget_columns(|styled_column_id| styled_column_id == column_id);
        self.drop_column_from_scored_frames(&frame_id, &column_id);
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
            .name = next_name.clone();
        frame.mirror_input_column(&column_id);
        for object in &mut self.objects {
            if let DataObject::Plot(plot) = object
                && plot.source_frame_id == frame_id
            {
                update_plot_field_titles(&mut plot.spec, &column_id, &old_name, &next_name);
            }
        }
        self.rename_carried_column(&frame_id, &column_id, &old_name, &next_name);
        Ok(())
    }

    pub(crate) fn apply_set_column_type(
        &mut self,
        frame_id: Id,
        column_id: Id,
        data_type: DataType,
        scale: Option<u8>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.column_schema_is_read_only() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let frame = self.frame_mut(&frame_id)?;
        let column = frame
            .columns
            .iter_mut()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        column.data_type = data_type;
        // The scale is a fact about an amount and nothing else: it goes
        // when the type does, and an amount arrives with the places it was
        // asked for, or keeps its own, or gets the default.
        column.scale = match data_type {
            DataType::Accounting => scale.or(column.scale).or(Some(DEFAULT_ACCOUNTING_SCALE)),
            _ => None,
        };
        column.categories = if data_type == DataType::Categorical {
            distinct_category_values(column, &frame.rows)
        } else {
            Vec::new()
        };
        let categories = column.categories.clone();
        let scale = column.scale;
        frame.mirror_input_column(&column_id);
        self.retype_carried_column(&frame_id, &column_id, data_type, scale, &categories);
        Ok(())
    }

    pub(crate) fn apply_set_column_categories(
        &mut self,
        frame_id: Id,
        column_id: Id,
        categories: Vec<String>,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.column_schema_is_read_only() {
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
        column.categories = categories.clone();
        frame.mirror_input_column(&column_id);
        self.retype_carried_column(
            &frame_id,
            &column_id,
            DataType::Categorical,
            None,
            &categories,
        );
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
        let old_format = std::mem::replace(&mut column.format, format.clone());
        self.reformat_carried_column(&frame_id, &column_id, old_format.as_ref(), format.as_ref());
        Ok(())
    }

    pub(crate) fn apply_set_column_formula(
        &mut self,
        frame_id: Id,
        column_id: Id,
        formula: Formula,
        data_type: DataType,
    ) -> Result<(), CoreError> {
        if self.frame(&frame_id)?.column_schema_is_read_only() {
            return Err(CoreError::DerivedFrameReadOnly);
        }
        let (inferred_type, scale) = self
            .frame(&frame_id)?
            .inferred_column_typing(self, &formula.expression)
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
        column.scale = scale;
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
