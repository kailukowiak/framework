//! Source replacement reconciles physical names while preserving the model.
use crate::*;

impl Document {
    pub(crate) fn prepare_refresh_frame_artifact(
        &self,
        frame_id: Id,
        artifact: DataArtifact,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        if frame.connector.is_none() {
            return Err(CoreError::InvalidOperation(
                "This imported snapshot has no connector to refresh".into(),
            ));
        }
        let (columns, base_columns, _) = self.reconcile_source_schemas(frame, &artifact)?;
        Ok(ReplicatedOperation::RefreshFrameArtifact {
            frame_id,
            artifact,
            columns,
            base_columns,
        })
    }

    /// What binding this artifact to this frame would change about its
    /// schema, worked out without changing anything.
    ///
    /// The same reconciliation the refresh itself runs, asked for its
    /// account rather than its columns. Answered separately instead of
    /// riding along inside the operation because the operation is history:
    /// it must replicate to a collaborator and replay under undo, and a
    /// sentence about what somebody saw once is neither.
    pub fn source_schema_diff(
        &self,
        frame_id: &str,
        artifact: &DataArtifact,
    ) -> Result<SchemaDiff, CoreError> {
        let frame = self.frame(frame_id)?;
        let (_, _, diff) = self.reconcile_source_schemas(frame, artifact)?;
        Ok(diff)
    }

    /// Points an imported frame at a different file.
    ///
    /// The frame survives the move — same ID, same column IDs, so every
    /// formula, join, and derived frame downstream keeps working. That is
    /// only true while the schema holds, so a file that does not match is
    /// refused here rather than silently breaking lineage. A genuinely
    /// different dataset is a new import, not a repoint.
    pub(crate) fn prepare_set_frame_source(
        &self,
        frame_id: Id,
        artifact: DataArtifact,
        connector: ConnectorRecipe,
    ) -> Result<ReplicatedOperation, CoreError> {
        let mut candidate = self.clone();
        {
            let frame = candidate.frame_mut(&frame_id)?;
            if frame.derivation.is_some() || frame.generator.is_some() {
                return Err(CoreError::InvalidOperation(
                    "Replace the source on the input frame, not on a derived or generated frame"
                        .into(),
                ));
            }
            if let Some(recipe) = frame.disconnected_read.take() {
                frame.columns = recipe.columns;
                frame.base_columns = recipe.base_columns;
                frame.steps = recipe.steps;
            }
            for column in &mut frame.columns {
                if column.source_name.is_none() && column.formula.is_none() {
                    column.source_name = Some(column.name.clone());
                }
            }
            for column in &mut frame.base_columns {
                if column.source_name.is_none() && column.formula.is_none() {
                    column.source_name = Some(column.name.clone());
                }
            }
            frame.rows.clear();
            frame.source_file = None;
            frame.materialization = None;
            frame.file_origin = None;
            frame.replace_base_artifact(artifact.clone());
            frame.connector = Some(connector);
        }
        let frame = candidate.frame(&frame_id)?;
        let (columns, base_columns, _) = candidate.reconcile_source_schemas(frame, &artifact)?;
        let frame = candidate.frame_mut(&frame_id)?;
        frame.columns = columns;
        frame.base_columns = base_columns;
        let mut frame = frame.clone();
        if self.frame(&frame_id)?.file_origin.is_some() {
            self.attach_editable_source(&mut frame)?;
        }
        Ok(ReplicatedOperation::RestoreFrame { frame })
    }

    /// Reconciles the physical input first, then lets an existing wrangle
    /// chain describe its output schema again. A filter or calculation passes
    /// a newly arrived source field through; an explicit Select or Summarize
    /// continues to omit it. If the new source is already broken because a
    /// referenced field vanished, the old output contract is retained so the
    /// replacement can still land and report that missing binding.
    fn reconcile_source_schemas(
        &self,
        frame: &FrameObject,
        artifact: &DataArtifact,
    ) -> Result<(Vec<Column>, Vec<Column>, SchemaDiff), CoreError> {
        let (inputs, diff) = self.reconcile_source_columns(frame, artifact)?;
        if frame.base_columns.is_empty() {
            return Ok((inputs, Vec::new(), diff));
        }

        let mut candidate = self.clone();
        {
            let candidate_frame = candidate.frame_mut(&frame.id)?;
            candidate_frame.replace_base_artifact(artifact.clone());
            candidate_frame.base_columns = inputs.clone();
        }
        let output = (|| {
            let mut plan = candidate
                .frame(&frame.id)?
                .materialize_polars_lazy(&candidate)
                .map_err(CoreError::Import)?;
            let schema = plan
                .collect_schema()
                .map_err(|error| CoreError::Import(error.to_string()))?;
            schema
                .iter()
                .map(|(id, data_type)| {
                    let id = id.to_string();
                    let mut column = frame
                        .columns
                        .iter()
                        .chain(inputs.iter())
                        .find(|column| column.id == id)
                        .cloned()
                        .unwrap_or_else(|| Column {
                            id: id.clone(),
                            name: id.clone(),
                            source_name: None,
                            data_type: DataType::String,
                            categories: Vec::new(),
                            format: None,
                            formula: None,
                        });
                    column.data_type =
                        framework_type_from_polars(data_type).map_err(CoreError::Import)?;
                    Ok(column)
                })
                .collect::<Result<Vec<_>, CoreError>>()
        })()
        .unwrap_or_else(|_| frame.columns.clone());
        Ok((output, inputs, diff))
    }

    /// Binds a replacement artifact to the identities this frame already has.
    ///
    /// A source move is not a schema migration ceremony. Fields that retain
    /// their physical names retain their IDs, fields newly present get IDs,
    /// and a field that vanished is kept only when something still reads it.
    /// Keeping that last identity is what turns an unavoidable broken model
    /// into a useful "source field X is missing" failure instead of `#REF`.
    fn reconcile_source_columns(
        &self,
        frame: &FrameObject,
        artifact: &DataArtifact,
    ) -> Result<(Vec<Column>, SchemaDiff), CoreError> {
        frame.artifact.as_ref().ok_or_else(|| {
            CoreError::InvalidOperation("Only an imported frame has a source file".into())
        })?;
        let replacement_schema = artifact_schema(artifact)?;
        let inputs = frame.input_columns();
        let mut reconciled = Vec::with_capacity(replacement_schema.len());
        let mut matched = std::collections::HashSet::new();
        let mut diff = SchemaDiff::default();

        // Source order is the useful order after a replacement. Identity is
        // found through the physical binding rather than the editable label.
        for (source_name, data_type) in replacement_schema {
            if let Some(existing) = inputs
                .iter()
                .find(|column| column.source_name.as_deref() == Some(source_name.as_str()))
            {
                let mut column = existing.clone();
                if column.data_type != data_type {
                    diff.type_changed
                        .push((column.name.clone(), column.data_type, data_type));
                }
                column.data_type = data_type;
                matched.insert(column.id.clone());
                reconciled.push(column);
            } else {
                diff.added.push(source_name.clone());
                reconciled.push(Column {
                    id: column_id(&source_name),
                    name: source_name.clone(),
                    source_name: Some(source_name),
                    data_type,
                    categories: Vec::new(),
                    format: None,
                    formula: None,
                });
            }
        }

        for missing in inputs
            .iter()
            .filter(|column| column.source_name.is_some() && !matched.contains(column.id.as_str()))
        {
            match self.column_still_read(frame, missing) {
                Some(reason) => {
                    diff.kept_missing.push((missing.name.clone(), reason));
                    reconciled.push(missing.clone());
                }
                None => diff.removed.push(missing.name.clone()),
            }
        }
        // Calculations layered onto an imported frame are model columns, not
        // fields supplied by the artifact. A source swap has no authority to
        // remove them; their expressions are also what may have kept a
        // missing source field above alive for a precise failure.
        reconciled.extend(
            inputs
                .iter()
                .filter(|column| column.source_name.is_none())
                .cloned(),
        );
        Ok((reconciled, diff))
    }

    /// What still reads a column whose source field has gone, in words,
    /// or `None` when nothing does and it can simply be dropped.
    ///
    /// The phrase is the whole point of the answer: "kept" on its own reads
    /// as a bug — a column with no data that will not go away — where "kept,
    /// still read by the Margin formula" reads as an explanation and points
    /// at the thing to fix. Checked in the order a person would care about:
    /// this frame's own work first, then the document around it.
    fn column_still_read(&self, frame: &FrameObject, missing: &Column) -> Option<String> {
        if let Some(reader) = frame.columns.iter().find(|column| {
            column.id != missing.id
                && column
                    .formula
                    .as_ref()
                    .is_some_and(|formula| formula.expression.references_column(&missing.id))
        }) {
            return Some(format!("still read by the {} formula", reader.name));
        }
        if frame.references_column_from_other_formulas(&missing.id) {
            return Some("still read by a cell formula".into());
        }
        if frame.display.references_column(&missing.id) {
            return Some("still used by this frame's filter or sort".into());
        }
        if frame
            .summaries
            .iter()
            .any(|summary| summary.column_id == missing.id)
        {
            return Some("still summarised in this frame's footer".into());
        }
        if frame.unique_keys.iter().any(|key| {
            key.column_ids
                .iter()
                .any(|column_id| column_id == &missing.id)
        }) {
            return Some("still part of this frame's unique key".into());
        }
        self.objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(candidate)
                    if candidate.id != frame.id
                        && candidate.wrangle_reads_foreign_column(&frame.id, &missing.id) =>
                {
                    Some(format!("still read by {}", candidate.name))
                }
                DataObject::Plot(plot)
                    if plot.source_frame_id == frame.id
                        && json_contains_string(&plot.spec, &missing.id) =>
                {
                    Some(format!("still drawn by {}", plot.name))
                }
                _ => None,
            })
            .or_else(|| {
                self.column_read_by(&frame.id, &missing.id)
                    .map(|reader| format!("still read by {reader}"))
            })
    }
}
