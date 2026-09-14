use crate::engine::ml::ml_error;
use crate::*;

impl Document {
    pub(crate) fn apply_model_spec(
        &mut self,
        model_id: Id,
        spec: ModelSpec,
    ) -> Result<(), CoreError> {
        self.validate_model_spec_shape(&spec)?;
        let DataObject::Model(model) = self.object_mut(&model_id)? else {
            return Err(ml_error("That is not a model"));
        };
        if model.spec.is_none() {
            return Err(ml_error(
                "An imported model has no editable training specification",
            ));
        }
        model.spec = Some(spec);
        Ok(())
    }
    pub(crate) fn apply_model_fit(
        &mut self,
        model_id: Id,
        fitted: Option<ModelFit>,
    ) -> Result<(), CoreError> {
        if let Some(fit) = &fitted {
            framework_ml::validate_fitted(&fit.result).map_err(ml_error)?;
        }
        let DataObject::Model(model) = self.object_mut(&model_id)? else {
            return Err(ml_error("That is not a model"));
        };
        if let (Some(before), Some(after)) = (&model.fitted, &fitted)
            && before.id == after.id
            && before != after
        {
            return Err(ml_error(
                "A fitted revision is immutable; use a new fitted revision ID",
            ));
        }
        model.fitted = fitted;
        Ok(())
    }
    pub(crate) fn validate_model_object(&self, object: &DataObject) -> Result<(), CoreError> {
        match object {
            DataObject::Model(model) => {
                if let Some(spec) = &model.spec {
                    self.validate_model_spec_shape(spec)?;
                }
                if let Some(fitted) = &model.fitted {
                    framework_ml::validate_fitted(&fitted.result).map_err(ml_error)?;
                }
            }
            DataObject::Frame(frame) if frame.prediction.is_some() => {
                let binding = frame.prediction.as_ref().expect("guarded");
                let source = frame
                    .derivation
                    .as_ref()
                    .ok_or_else(|| ml_error("A prediction frame needs a source"))?;
                if source.source_frame_id == frame.id {
                    return Err(ml_error("A prediction frame cannot read itself"));
                }
                let inputs: std::collections::HashSet<_> =
                    binding.feature_column_ids.iter().collect();
                let outputs: std::collections::HashSet<_> =
                    binding.output_column_ids.iter().collect();
                if inputs.len() != binding.feature_column_ids.len()
                    || outputs.len() != binding.output_column_ids.len()
                {
                    return Err(ml_error("Prediction bindings cannot repeat column IDs"));
                }
                let model = self.model(&binding.model_id)?;
                let fit = model
                    .fitted
                    .as_ref()
                    .ok_or_else(|| ml_error("Fit the model before predicting"))?;
                if binding.feature_column_ids.len() != fit.result.feature_names.len()
                    || binding.output_column_ids.len()
                        != framework_ml::output_names(&fit.result).len()
                {
                    return Err(ml_error(
                        "Prediction binding does not match the fitted model",
                    ));
                }
                if !frame.rows.is_empty()
                    || frame.source_file.is_some()
                    || frame.artifact.is_some()
                    || frame.generator.is_some()
                {
                    return Err(ml_error(
                        "A prediction frame cannot also own or import rows",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Where a carried column goes in a scored frame's column list: after the
/// neighbour it followed in the source when that neighbour is here, else
/// last among the carried columns, ahead of the outputs.
fn insert_carried(
    columns: &mut Vec<Column>,
    column: Column,
    after_column_id: Option<&str>,
    output_column_ids: &[Id],
) {
    if columns.iter().any(|existing| existing.id == column.id) {
        return;
    }
    let at = after_column_id
        .and_then(|after| columns.iter().position(|existing| existing.id == after))
        .map(|index| index + 1)
        .or_else(|| {
            columns
                .iter()
                .position(|existing| output_column_ids.contains(&existing.id))
        })
        .unwrap_or(columns.len());
    columns.insert(at, column);
}

/// A scored frame is its source's columns with the model's outputs after
/// them, and it stays that: a column added to, dropped from, renamed,
/// retyped or reformatted in the source is followed here. The carried
/// columns keep the source's ids, which is what makes following them a
/// matter of looking them up. A name or format the person changed on the
/// scored frame itself is theirs and is left alone.
impl Document {
    /// The prediction frames scored from `source_id` that carry any of its
    /// columns — a frame from before columns were carried declares only its
    /// outputs, and gains nothing by following a source it never mirrored.
    pub(crate) fn scored_frames(&self, source_id: &str) -> impl Iterator<Item = &FrameObject> {
        self.objects.iter().filter_map(move |object| match object {
            DataObject::Frame(frame) if carries_source_columns(frame, source_id) => Some(frame),
            _ => None,
        })
    }

    fn scored_frames_mut(&mut self, source_id: &str) -> impl Iterator<Item = &mut FrameObject> {
        self.objects
            .iter_mut()
            .filter_map(move |object| match object {
                DataObject::Frame(frame) if carries_source_columns(frame, source_id) => Some(frame),
                _ => None,
            })
    }

    /// The chain input always gains the column. The declared columns gain
    /// it only while the chain is empty: with steps in the way they are the
    /// chain's output and the chain decides, so saving the chain again is
    /// what picks the column up.
    pub(crate) fn carry_column_into_scored_frames(
        &mut self,
        source_id: &str,
        column: &Column,
        after_column_id: Option<&str>,
    ) {
        for frame in self.scored_frames_mut(source_id) {
            let outputs = output_column_ids(frame);
            let carried = Column {
                id: column.id.clone(),
                name: column.name.clone(),
                source_name: None,
                data_type: column.data_type,
                categories: column.categories.clone(),
                format: column.format.clone(),
                formula: None,
            };
            insert_carried(
                &mut frame.base_columns,
                carried.clone(),
                after_column_id,
                &outputs,
            );
            if frame
                .derivation
                .as_ref()
                .is_none_or(|derivation| derivation.steps.is_empty())
            {
                insert_carried(&mut frame.columns, carried, after_column_id, &outputs);
            }
        }
    }

    pub(crate) fn drop_column_from_scored_frames(&mut self, source_id: &str, column_id: &str) {
        for frame in self.scored_frames_mut(source_id) {
            frame.base_columns.retain(|column| column.id != column_id);
            frame.columns.retain(|column| column.id != column_id);
            frame
                .summaries
                .retain(|summary| summary.column_id != column_id);
            frame.display.styles.retain(|entry| match &entry.target {
                FrameStyleTarget::Column {
                    column_id: styled_column_id,
                }
                | FrameStyleTarget::Cell {
                    column_id: styled_column_id,
                    ..
                } => styled_column_id != column_id,
                _ => true,
            });
        }
    }

    /// Followed only while the carried column still wore the source's old
    /// name, and never onto a name another column here already has.
    pub(crate) fn rename_carried_column(
        &mut self,
        source_id: &str,
        column_id: &str,
        old_name: &str,
        name: &str,
    ) {
        for frame in self.scored_frames_mut(source_id) {
            if frame
                .columns
                .iter()
                .chain(frame.base_columns.iter())
                .any(|column| column.id != column_id && column.name == name)
            {
                continue;
            }
            for column in frame
                .columns
                .iter_mut()
                .chain(frame.base_columns.iter_mut())
                .filter(|column| column.id == column_id && column.name == old_name)
            {
                column.name = name.to_string();
            }
        }
    }

    pub(crate) fn retype_carried_column(
        &mut self,
        source_id: &str,
        column_id: &str,
        data_type: DataType,
        categories: &[String],
    ) {
        for frame in self.scored_frames_mut(source_id) {
            for column in frame
                .columns
                .iter_mut()
                .chain(frame.base_columns.iter_mut())
                .filter(|column| column.id == column_id)
            {
                column.data_type = data_type;
                column.categories = categories.to_vec();
            }
        }
    }

    /// Followed only while the carried column still showed the source's
    /// old format: a format chosen on the scored frame itself stays.
    pub(crate) fn reformat_carried_column(
        &mut self,
        source_id: &str,
        column_id: &str,
        old_format: Option<&ColumnFormat>,
        format: Option<&ColumnFormat>,
    ) {
        for frame in self.scored_frames_mut(source_id) {
            for column in frame
                .columns
                .iter_mut()
                .chain(frame.base_columns.iter_mut())
                .filter(|column| column.id == column_id && column.format.as_ref() == old_format)
            {
                column.format = format.cloned();
            }
        }
    }
}

fn output_column_ids(frame: &FrameObject) -> Vec<Id> {
    frame
        .prediction
        .as_ref()
        .map(|binding| binding.output_column_ids.clone())
        .unwrap_or_default()
}

fn carries_source_columns(frame: &FrameObject, source_id: &str) -> bool {
    let Some(binding) = &frame.prediction else {
        return false;
    };
    frame
        .derivation
        .as_ref()
        .is_some_and(|derivation| derivation.source_frame_id == source_id)
        && frame
            .base_columns
            .iter()
            .any(|column| !binding.output_column_ids.contains(&column.id))
}
