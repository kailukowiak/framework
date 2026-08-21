//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Whole-object lifecycle: adding, importing, renaming, and deleting the
//! values, frames, plots, and text a document holds.

use crate::*;

impl Document {
    pub(crate) fn apply_add_object(
        &mut self,
        object: DataObject,
        view: CanvasView,
        container_id: Option<Id>,
    ) -> Result<(), CoreError> {
        if object.id() != view.object_id {
            return Err(CoreError::InvalidOperation(
                "view does not reference the object being added".into(),
            ));
        }
        if self
            .objects
            .iter()
            .any(|existing| existing.id() == object.id())
            || self.views.iter().any(|existing| existing.id == view.id)
        {
            return Err(CoreError::InvalidOperation(
                "an added object or view ID already exists".into(),
            ));
        }
        let object_id = object.id().to_string();
        self.objects.push(object);
        self.views.push(view);
        if let Some(container_id) = container_id {
            let DataObject::Container(container) = self.object_mut(&container_id)? else {
                return Err(CoreError::InvalidOperation(
                    "That is not a container".into(),
                ));
            };
            container.member_ids.push(object_id);
        }
        Ok(())
    }

    pub(crate) fn apply_refresh_frame_artifact(
        &mut self,
        frame_id: Id,
        artifact: DataArtifact,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        if frame.artifact.is_none() || frame.connector.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only an imported snapshot with a connector can be refreshed".into(),
            ));
        }
        frame.artifact = Some(artifact);
        replace_source_columns(frame, columns, base_columns);
        Ok(())
    }

    /// Swaps both the snapshot and the recipe that produced it, so the frame
    /// reads from somewhere else from now on. Columns are untouched: the
    /// schema check in `prepare` is what makes that safe.
    pub(crate) fn apply_set_frame_source(
        &mut self,
        frame_id: Id,
        artifact: DataArtifact,
        connector: ConnectorRecipe,
        columns: Vec<Column>,
        base_columns: Vec<Column>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        if frame.artifact.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only an imported frame has a source file to change".into(),
            ));
        }
        frame.artifact = Some(artifact);
        frame.connector = Some(connector);
        replace_source_columns(frame, columns, base_columns);
        Ok(())
    }

    pub(crate) fn apply_rename_object(
        &mut self,
        object_id: Id,
        name: String,
    ) -> Result<(), CoreError> {
        match self.object_mut(&object_id)? {
            DataObject::Value(value) => value.name = name,
            DataObject::Result(result) => result.name = name,
            DataObject::Block(block) => block.name = name,
            DataObject::Series(series) => series.name = name,
            DataObject::Container(container) => container.name = name,
            DataObject::Frame(frame) => frame.name = name,
            DataObject::Text(text) => text.name = name,
            DataObject::Plot(plot) => plot.name = name,
        }
        Ok(())
    }

    pub(crate) fn apply_delete_object(&mut self, object_id: Id) -> Result<(), CoreError> {
        let object_index = self
            .objects
            .iter()
            .position(|object| object.id() == object_id)
            .ok_or(CoreError::ObjectNotFound)?;
        // A value, a result, a list, and a block's lines are all read by id
        // from a formula, so all are held in place by one being written —
        // whether the formula sits in a frame, a result, or a block. A block
        // brings every line id it holds: deleting the card is deleting the
        // lines, and a formula holds a line, never the block itself.
        let referenced_ids: Vec<&str> = match &self.objects[object_index] {
            DataObject::Value(_) | DataObject::Result(_) | DataObject::Series(_) => {
                vec![object_id.as_str()]
            }
            DataObject::Block(block) => block.lines.iter().map(|line| line.id.as_str()).collect(),
            _ => Vec::new(),
        };
        let going = as_named(self.objects[object_index].name());
        let referenced_by = referenced_ids
            .iter()
            .find_map(|target| {
                self.objects.iter().find_map(|object| {
                    // The object being deleted does not hold itself in place:
                    // a block's lines reading each other go out together.
                    if object.id() == object_id {
                        return None;
                    }
                    match object {
                        DataObject::Frame(frame) => (frame.references_object(target)
                            || frame.display.references_object(target))
                        .then(|| as_named(&frame.name)),
                        DataObject::Result(result) => result
                            .formula
                            .expression
                            .references_object(target)
                            .then(|| as_named(&result.name)),
                        DataObject::Block(block) => block.lines.iter().find_map(|line| {
                            line.expression()?
                                .references_object(target)
                                .then(|| as_line_named(block, line))
                        }),
                        _ => None,
                    }
                })
            })
            .map(|reader| {
                format!("{reader} reads {going}, so it cannot be deleted. Change the formula that reads it first.")
            });
        let derived_from = self.objects.iter().find_map(|object| match object {
            // A union in a source frame's own chain reads the stacked frame
            // without any derivation existing, so both step lists answer
            // for "built from", not just the derivation.
            DataObject::Frame(frame) => (frame
                .derivation
                .as_ref()
                .is_some_and(|derivation| derivation.references_frame(&object_id))
                || frame
                    .steps
                    .iter()
                    .filter_map(FrameStep::lookup_frame_id)
                    .any(|lookup_id| *lookup_id == object_id))
            .then(|| {
                format!(
                    "{} is built from {going}, so it cannot be deleted. Delete that frame first.",
                    as_named(&frame.name)
                )
            }),
            _ => None,
        });
        let drawn_from = self.objects.iter().find_map(|object| match object {
            DataObject::Plot(plot) if plot.source_frame_id == object_id => Some(format!(
                "{} is drawn from {going}, so it cannot be deleted. Delete that plot first.",
                as_named(&plot.name)
            )),
            _ => None,
        });
        let read_across = matches!(&self.objects[object_index], DataObject::Frame(_))
            .then(|| self.frame_read_by(&object_id))
            .flatten()
            .map(|reader| {
                format!("{reader} reads {going}, so it cannot be deleted. Change the formula that reads it first.")
            });
        if let Some(refusal) = referenced_by
            .or(derived_from)
            .or(drawn_from)
            .or(read_across)
        {
            return Err(CoreError::ReferencedByFormula(refusal));
        }
        self.objects.remove(object_index);
        // Whatever container held it stops holding it. A member id left
        // pointing at nothing would draw a gap in that container's card and
        // break every name that resolved through it.
        for object in &mut self.objects {
            if let DataObject::Container(container) = object {
                container.member_ids.retain(|member| *member != object_id);
            }
        }
        // A card showing the deleted object as one of several tabs keeps
        // going on the tab that took its place; a card left with nothing to
        // show goes away with it.
        self.views.retain_mut(|view| {
            let mut tabs = view.tabs().to_vec();
            let Some(index) = tabs.iter().position(|tab| *tab == object_id) else {
                return true;
            };
            tabs.remove(index);
            if tabs.is_empty() {
                return false;
            }
            let next = tabs[index.min(tabs.len() - 1)].clone();
            view.set_tabs(tabs, next);
            true
        });
        Ok(())
    }

    pub(crate) fn apply_set_value(&mut self, object_id: Id, raw: String) -> Result<(), CoreError> {
        match self.object_mut(&object_id)? {
            DataObject::Value(value) => {
                value.data_type = infer_data_type(&raw);
                value.raw = raw;
            }
            _ => return Err(CoreError::ObjectNotFound),
        }
        Ok(())
    }

    pub(crate) fn apply_set_result_formula(
        &mut self,
        object_id: Id,
        formula: Formula,
    ) -> Result<(), CoreError> {
        match self.object_mut(&object_id)? {
            DataObject::Result(result) => result.formula = formula,
            _ => return Err(CoreError::ObjectNotFound),
        }
        Ok(())
    }

    pub(crate) fn apply_set_series(
        &mut self,
        object_id: Id,
        values: Vec<String>,
        data_type: DataType,
    ) -> Result<(), CoreError> {
        match self.object_mut(&object_id)? {
            DataObject::Series(series) => {
                series.values = values;
                series.data_type = data_type;
            }
            _ => return Err(CoreError::ObjectNotFound),
        }
        Ok(())
    }

    /// Retypes a list without touching what is in it.
    ///
    /// Refused when a value would stop being readable, which is the only way
    /// this can go wrong and the only thing worth stopping: calling a list of
    /// names a list of numbers turns every one of them into a null, and
    /// silently.
    pub(crate) fn apply_set_series_type(
        &mut self,
        object_id: Id,
        data_type: DataType,
    ) -> Result<(), CoreError> {
        let DataObject::Series(series) = self.object_mut(&object_id)? else {
            return Err(CoreError::ObjectNotFound);
        };
        if let Some(unreadable) = series
            .values
            .iter()
            .find(|raw| parse_scalar_value(raw, data_type).is_err())
        {
            return Err(CoreError::InvalidOperation(format!(
                "‘{unreadable}’ is not a {}",
                data_type_name(data_type)
            )));
        }
        series.data_type = data_type;
        Ok(())
    }

    pub(crate) fn apply_set_container_members(
        &mut self,
        members: Vec<(Id, Vec<Id>)>,
    ) -> Result<(), CoreError> {
        for (container_id, member_ids) in members {
            let DataObject::Container(container) = self.object_mut(&container_id)? else {
                return Err(CoreError::ObjectNotFound);
            };
            container.member_ids = member_ids;
        }
        Ok(())
    }

    pub(crate) fn apply_set_plot_spec(
        &mut self,
        plot_id: Id,
        spec: serde_json::Value,
    ) -> Result<(), CoreError> {
        match self.object_mut(&plot_id)? {
            DataObject::Plot(plot) => plot.spec = spec,
            _ => return Err(CoreError::ObjectNotFound),
        }
        Ok(())
    }

    /// Puts a frame back as it was, replacing whatever stands in its place.
    ///
    /// Deliberately unguarded, unlike the operations it inverts: it is
    /// generated from a state this replica held, and refusing to restore
    /// what was just there would make the edit that produced it permanent.
    pub(crate) fn apply_restore_frame(&mut self, frame: FrameObject) -> Result<(), CoreError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id() == frame.id)
            .ok_or(CoreError::FrameNotFound)?;
        self.objects[index] = DataObject::Frame(frame);
        Ok(())
    }

    /// Puts a deleted object back together with the cards that showed it.
    ///
    /// A card is restored only if it is gone: deleting the object may have
    /// removed a whole card, or merely dropped one tab from a card that is
    /// still there and has since moved.
    pub(crate) fn apply_restore_object(
        &mut self,
        object: DataObject,
        views: Vec<CanvasView>,
    ) -> Result<(), CoreError> {
        if !self
            .objects
            .iter()
            .any(|existing| existing.id() == object.id())
        {
            self.objects.push(object);
        }
        for view in views {
            match self
                .views
                .iter_mut()
                .find(|existing| existing.id == view.id)
            {
                Some(existing) => *existing = view,
                None => self.views.push(view),
            }
        }
        Ok(())
    }

    /// Puts the canvas back: the whole view list, because the tab
    /// operations can add and remove cards as well as rearrange them.
    pub(crate) fn apply_restore_views(&mut self, views: Vec<CanvasView>) -> Result<(), CoreError> {
        if views.iter().any(|view| {
            !self
                .objects
                .iter()
                .any(|object| object.id() == view.object_id)
        }) {
            return Err(CoreError::ObjectNotFound);
        }
        self.views = views;
        Ok(())
    }

    pub(crate) fn apply_rename_document(&mut self, name: String) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::InvalidOperation(
                "Document name cannot be empty".into(),
            ));
        }
        self.name = name.into();
        Ok(())
    }
}

/// A chain stores its input schema separately from its output schema. Source
/// replacement edits the former; a plain imported frame has only one schema,
/// so the same reconciliation becomes its visible columns immediately.
fn replace_source_columns(
    frame: &mut FrameObject,
    columns: Vec<Column>,
    base_columns: Vec<Column>,
) {
    let retained = columns
        .iter()
        .map(|column| column.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    frame.display.styles.retain(|style| match &style.target {
        FrameStyleTarget::Column { column_id } | FrameStyleTarget::Cell { column_id, .. } => {
            retained.contains(column_id.as_str())
        }
        _ => true,
    });
    frame.columns = columns;
    frame.base_columns = base_columns;
}
