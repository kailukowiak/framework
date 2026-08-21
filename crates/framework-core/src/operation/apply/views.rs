//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! Canvas geometry, frame tabs, and a frame's display layer — position, size,
//! collapse, which frames a card offers as tabs, and each frame's own display
//! filter, sort, orientation, and styles.

use crate::*;

impl Document {
    pub(crate) fn apply_frame_summary_operation(
        &mut self,
        operation: ReplicatedOperation,
    ) -> Result<(), CoreError> {
        match operation {
            ReplicatedOperation::SetFrameSummaryRows {
                frame_id,
                summary_rows,
            } => self.apply_set_frame_summary_rows(frame_id, summary_rows),
            ReplicatedOperation::SetFrameSummaryDrawer {
                frame_id,
                open,
                height,
            } => self.apply_set_frame_summary_drawer(frame_id, open, height),
            _ => unreachable!("only frame profile operations are routed here"),
        }
    }

    pub(crate) fn apply_set_frame_style_rules(
        &mut self,
        frame_id: Id,
        rules: Vec<FrameStyleRule>,
    ) -> Result<(), CoreError> {
        self.frame_mut(&frame_id)?.display.style_rules = rules;
        Ok(())
    }

    pub(crate) fn apply_move_view(&mut self, view_id: Id, x: f64, y: f64) -> Result<(), CoreError> {
        let view = self.view_mut(&view_id)?;
        view.x = x.max(0.0);
        view.y = y.max(0.0);
        Ok(())
    }

    pub(crate) fn apply_resize_view(
        &mut self,
        view_id: Id,
        width: f64,
        height: f64,
    ) -> Result<(), CoreError> {
        let object_id = self.view(&view_id)?.object_id.clone();
        let roomy = self.objects.iter().any(|object| {
            object.id() == object_id && matches!(object, DataObject::Frame(_) | DataObject::Plot(_))
        });
        let (minimum_width, minimum_height) = if roomy {
            (360.0, 210.0)
        } else {
            (180.0, 100.0)
        };
        let view = self.view_mut(&view_id)?;
        view.width = width.max(minimum_width);
        view.height = height.max(minimum_height);
        Ok(())
    }

    pub(crate) fn apply_set_view_collapsed(
        &mut self,
        view_id: Id,
        collapsed: bool,
    ) -> Result<(), CoreError> {
        self.view_mut(&view_id)?.collapsed = collapsed;
        Ok(())
    }

    /// A placement naming a window this replica does not have is skipped
    /// rather than refused: the arrangement is presentation, and losing one
    /// card's position is not worth rejecting the whole tidy.
    pub(crate) fn apply_set_view_layout(
        &mut self,
        placements: Vec<ViewPlacement>,
    ) -> Result<(), CoreError> {
        for placement in placements {
            if let Ok(view) = self.view_mut(&placement.view_id) {
                view.x = placement.x.max(0.0);
                view.y = placement.y.max(0.0);
            }
        }
        Ok(())
    }

    pub(crate) fn apply_set_frame_display_orientation(
        &mut self,
        frame_id: Id,
        orientation: FrameViewOrientation,
    ) -> Result<(), CoreError> {
        self.frame_mut(&frame_id)?.display.orientation = orientation;
        Ok(())
    }

    pub(crate) fn apply_set_frame_display_crosstab(
        &mut self,
        frame_id: Id,
        crosstab: Option<CrosstabDisplay>,
    ) -> Result<(), CoreError> {
        self.frame_mut(&frame_id)?.display.crosstab = crosstab;
        Ok(())
    }

    /// Adds a prepared object to a card's strip and selects it.
    ///
    /// Both ways into this — branching a frame, plotting one — are checked
    /// by the same rule the drag-between-cards path uses, so a tab can never
    /// arrive on a card that does not show what it reads.
    pub(crate) fn apply_add_tab(
        &mut self,
        view_id: Id,
        object: DataObject,
    ) -> Result<(), CoreError> {
        let object_id = object.id().to_string();
        if self
            .objects
            .iter()
            .any(|existing| existing.id() == object_id)
        {
            return Err(CoreError::InvalidOperation(
                "An object with that ID already exists".into(),
            ));
        }
        // The object is not in the document yet, so the shared rule cannot
        // look it up: push first, and undo the push if it does not belong.
        self.objects.push(object);
        if let Err(error) = self.validate_tab_target(&view_id, &object_id) {
            self.objects.pop();
            return Err(error);
        }
        let view = self.view_mut(&view_id)?;
        let mut tabs = view.tabs().to_vec();
        tabs.push(object_id.clone());
        view.set_tabs(tabs, object_id);
        Ok(())
    }

    pub(crate) fn apply_move_tab(
        &mut self,
        source_view_id: Id,
        target_view_id: Id,
        object_id: Id,
        target_index: usize,
    ) -> Result<(), CoreError> {
        self.validate_tab_target(&target_view_id, &object_id)?;
        let (mut tabs, from) = self.take_tab(&source_view_id, &object_id)?;

        if source_view_id == target_view_id {
            // Removing the tab shifts everything after it down one, so an
            // index that pointed past the old position has to follow.
            let insert_at = if from < target_index {
                target_index.saturating_sub(1)
            } else {
                target_index
            }
            .min(tabs.len());
            tabs.insert(insert_at, object_id.clone());
            self.view_mut(&source_view_id)?.set_tabs(tabs, object_id);
            return Ok(());
        }

        self.close_tab(&source_view_id, tabs, from)?;
        let target = self.view_mut(&target_view_id)?;
        let mut tabs = target.tabs().to_vec();
        tabs.insert(target_index.min(tabs.len()), object_id.clone());
        target.set_tabs(tabs, object_id);
        Ok(())
    }

    pub(crate) fn apply_detach_tab(
        &mut self,
        source_view_id: Id,
        object_id: Id,
        new_view: CanvasView,
    ) -> Result<(), CoreError> {
        if self.views.iter().any(|view| view.id == new_view.id) {
            return Err(CoreError::InvalidOperation(
                "A canvas window with that ID already exists".into(),
            ));
        }
        if new_view.object_id != object_id || !new_view.tab_object_ids.is_empty() {
            return Err(CoreError::InvalidOperation(
                "A detached window must show exactly the moved tab".into(),
            ));
        }
        if self.view(&source_view_id)?.tabs().len() <= 1 {
            return Err(CoreError::InvalidOperation(
                "The final tab should move its existing window".into(),
            ));
        }
        let (tabs, from) = self.take_tab(&source_view_id, &object_id)?;
        self.close_tab(&source_view_id, tabs, from)?;
        self.views.push(new_view);
        Ok(())
    }

    /// Removes `object_id` from a card's strip, returning what is left and
    /// the position it held. Nothing is written back yet — the caller
    /// decides where the tab lands.
    fn take_tab(&self, view_id: &str, object_id: &str) -> Result<(Vec<Id>, usize), CoreError> {
        let mut tabs = self.view(view_id)?.tabs().to_vec();
        let from = tabs
            .iter()
            .position(|tab| tab == object_id)
            .ok_or(CoreError::ViewNotFound)?;
        tabs.remove(from);
        Ok((tabs, from))
    }

    /// Writes back a strip a tab has just left, selecting the tab that slid
    /// into its place. A card with nothing left to show is removed.
    fn close_tab(&mut self, view_id: &str, tabs: Vec<Id>, from: usize) -> Result<(), CoreError> {
        if tabs.is_empty() {
            self.views.retain(|view| view.id != view_id);
            return Ok(());
        }
        let next = tabs[from.min(tabs.len() - 1)].clone();
        self.view_mut(view_id)?.set_tabs(tabs, next);
        Ok(())
    }

    pub(crate) fn apply_set_active_tab(
        &mut self,
        view_id: Id,
        object_id: Id,
    ) -> Result<(), CoreError> {
        let view = self.view_mut(&view_id)?;
        if !view.tabs().contains(&object_id) {
            return Err(CoreError::InvalidOperation(
                "That object is not a tab of this card".into(),
            ));
        }
        view.object_id = object_id;
        Ok(())
    }

    pub(crate) fn apply_set_frame_display_filter(
        &mut self,
        frame_id: Id,
        filters: Vec<Formula>,
        filter_match_all: bool,
    ) -> Result<(), CoreError> {
        let predicates = filters
            .into_iter()
            .map(|formula| formula.expression)
            .collect();
        self.frame_mut(&frame_id)?
            .display
            .set_filter(predicates, filter_match_all);
        Ok(())
    }

    pub(crate) fn apply_set_frame_display_sort(
        &mut self,
        frame_id: Id,
        keys: Vec<DerivedSort>,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        validate_sort_keys(frame, &keys)?;
        // Header sorting is a data decision now. Only the trailing sort is
        // the header's compact authoring surface: an earlier sort may exist
        // specifically to establish order for shift/window calculations, so
        // reaching into the chain to replace it would silently change math.
        let steps = if let Some(derivation) = &mut frame.derivation {
            if derivation.steps.is_empty()
                && let Some(join) = &derivation.join
            {
                derivation
                    .steps
                    .push(FrameStep::Join { join: join.clone() });
            }
            &mut derivation.steps
        } else {
            &mut frame.steps
        };
        if matches!(steps.last(), Some(FrameStep::Sort { .. })) {
            steps.pop();
        }
        if !keys.is_empty() {
            steps.push(FrameStep::Sort { keys });
        }
        Ok(())
    }

    pub(crate) fn apply_set_frame_summary_rows(
        &mut self,
        frame_id: Id,
        summary_rows: Option<Vec<SummaryOperation>>,
    ) -> Result<(), CoreError> {
        self.frame_mut(&frame_id)?.display.summary_rows = summary_rows;
        Ok(())
    }

    pub(crate) fn apply_set_frame_summary_drawer(
        &mut self,
        frame_id: Id,
        open: bool,
        height: Option<f64>,
    ) -> Result<(), CoreError> {
        let display = &mut self.frame_mut(&frame_id)?.display;
        display.summary_drawer_open = open;
        display.summary_drawer_height = height
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(72.0, 600.0));
        Ok(())
    }

    pub(crate) fn apply_set_frame_style(
        &mut self,
        frame_id: Id,
        target: FrameStyleTarget,
        style: FrameCellStyle,
    ) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        validate_frame_style_target(frame, &target)?;
        validate_frame_cell_style(&style)?;
        let styles = &mut frame.display.styles;
        match styles
            .iter()
            .position(|candidate| candidate.target == target)
        {
            Some(index) if style.is_empty() => {
                styles.remove(index);
            }
            Some(index) => styles[index].style = style,
            None if !style.is_empty() => styles.push(FrameStyle { target, style }),
            None => {}
        }
        Ok(())
    }

    pub(crate) fn apply_promote_display_to_steps(&mut self, frame_id: Id) -> Result<(), CoreError> {
        let frame = self.frame_mut(&frame_id)?;
        validate_promotable_display(frame)?;
        let promoted = std::mem::take(&mut frame.display.steps);
        match &mut frame.derivation {
            // A join has no chain to append to, which is why
            // `validate_promotable_display` refused one above; everything
            // else is a chain already.
            Some(derivation) => derivation.steps.extend(promoted),
            None => frame.steps.extend(promoted),
        }
        Ok(())
    }
}
