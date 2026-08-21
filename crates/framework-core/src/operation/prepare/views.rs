//! Resolving `Operation`s in this family into fully determined
//! `ReplicatedOperation`s: IDs minted, formula names bound to column IDs, and
//! every precondition checked before anything is applied.
//!
//! Canvas geometry, frame tabs, and a frame's display layer — position, size,
//! collapse, which frames a card offers as tabs, and each frame's own display
//! filter, sort, orientation, and styles.

use crate::*;

impl Document {
    pub(crate) fn prepare_frame_summary_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        match operation {
            Operation::SetFrameSummaryRows {
                frame_id,
                summary_rows,
            } => self.prepare_set_frame_summary_rows(frame_id, summary_rows),
            Operation::SetFrameSummaryDrawer {
                frame_id,
                open,
                height,
            } => self.prepare_set_frame_summary_drawer(frame_id, open, height),
            _ => unreachable!("only frame profile operations are routed here"),
        }
    }

    pub(crate) fn prepare_move_view(
        &self,
        view_id: Id,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::MoveView { view_id, x, y })
    }

    pub(crate) fn prepare_resize_view(
        &self,
        view_id: Id,
        width: f64,
        height: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok({
            ReplicatedOperation::ResizeView {
                view_id,
                width,
                height,
            }
        })
    }

    pub(crate) fn prepare_set_view_collapsed(
        &self,
        view_id: Id,
        collapsed: bool,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::SetViewCollapsed { view_id, collapsed })
    }

    /// Resolves the tidied arrangement here rather than at apply time, so
    /// the operation carries positions instead of an instruction. A replica
    /// replaying it lands on the same canvas even if its own document has
    /// since gained a window.
    pub(crate) fn prepare_tidy_layout(&self) -> Result<ReplicatedOperation, CoreError> {
        Ok(ReplicatedOperation::SetViewLayout {
            placements: self.tidy_layout(),
        })
    }

    pub(crate) fn prepare_set_frame_display_orientation(
        &self,
        frame_id: Id,
        orientation: FrameViewOrientation,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.frame(&frame_id)?;
        Ok(ReplicatedOperation::SetFrameDisplayOrientation {
            frame_id,
            orientation,
        })
    }

    /// A crosstab is a way of looking, so it asks only that its two columns
    /// exist and differ; the grouping of the remaining columns is worked
    /// out at render time from whatever the chain currently produces.
    pub(crate) fn prepare_set_frame_display_crosstab(
        &self,
        frame_id: Id,
        crosstab: Option<CrosstabDisplay>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        if let Some(crosstab) = &crosstab {
            for column_id in [&crosstab.names_column_id, &crosstab.values_column_id] {
                if !frame.columns.iter().any(|column| column.id == *column_id) {
                    return Err(CoreError::ColumnNotFound);
                }
            }
            if crosstab.names_column_id == crosstab.values_column_id {
                return Err(CoreError::InvalidOperation(
                    "A crosstab needs two different columns: one to name the columns, \
                     one to fill the cells"
                        .into(),
                ));
            }
        }
        Ok(ReplicatedOperation::SetFrameDisplayCrosstab { frame_id, crosstab })
    }

    /// A branched tab is a new frame: a pass-through child of `frame_id` with
    /// an empty wrangle chain and a display layer of its own. Two tabs filter
    /// independently because they are two frames, not because a parallel
    /// state machine keeps their filters apart.
    pub(crate) fn prepare_branch_frame(
        &self,
        view_id: Id,
        frame_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        let view = self.view(&view_id)?;
        if !view.tabs().contains(&frame_id) {
            return Err(CoreError::InvalidOperation(
                "That frame is not a tab of this card".into(),
            ));
        }
        let source = self.frame(&frame_id)?;
        let name = self.unique_frame_name(&format!("{} copy", source.name), None);
        Ok(ReplicatedOperation::AddTab {
            view_id,
            object: DataObject::Frame(source.pass_through_child(name)),
        })
    }

    pub(crate) fn prepare_move_tab(
        &self,
        source_view_id: Id,
        target_view_id: Id,
        object_id: Id,
        target_index: usize,
    ) -> Result<ReplicatedOperation, CoreError> {
        if !self.view(&source_view_id)?.tabs().contains(&object_id) {
            return Err(CoreError::InvalidOperation(
                "That object is not a tab of this card".into(),
            ));
        }
        self.validate_tab_target(&target_view_id, &object_id)?;
        Ok(ReplicatedOperation::MoveTab {
            source_view_id,
            target_view_id,
            object_id,
            target_index,
        })
    }

    pub(crate) fn prepare_detach_tab(
        &self,
        view_id: Id,
        object_id: Id,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let source = self.view(&view_id)?;
        if !source.tabs().contains(&object_id) {
            return Err(CoreError::InvalidOperation(
                "That object is not a tab of this card".into(),
            ));
        }
        // Detaching the only tab is just moving the card it already has.
        if source.tabs().len() == 1 {
            return Ok(ReplicatedOperation::MoveView { view_id, x, y });
        }
        Ok(ReplicatedOperation::DetachTab {
            source_view_id: view_id,
            object_id: object_id.clone(),
            new_view: CanvasView {
                id: id(),
                object_id,
                x: x.max(0.0),
                y: y.max(0.0),
                width: source.width,
                height: source.height,
                collapsed: false,
                tab_object_ids: Vec::new(),
            },
        })
    }

    pub(crate) fn prepare_set_active_tab(
        &self,
        view_id: Id,
        object_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        if !self.view(&view_id)?.tabs().contains(&object_id) {
            return Err(CoreError::InvalidOperation(
                "That object is not a tab of this card".into(),
            ));
        }
        Ok(ReplicatedOperation::SetActiveTab { view_id, object_id })
    }

    pub(crate) fn prepare_set_frame_display_filter(
        &self,
        frame_id: Id,
        filters: Vec<String>,
        filter_match_all: bool,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let mut parsed = Vec::new();
        for formula in filters {
            let expression = self.prepare_formula_for_frame(&frame.id, &formula)?;
            let data_type = frame
                .infer_polars_expression_type(self, &expression)
                .map_err(CoreError::Formula)?;
            if data_type != DataType::Boolean {
                return Err(CoreError::InvalidOperation(
                    "A display filter must produce true or false".into(),
                ));
            }
            parsed.push(Formula { expression });
        }
        Ok(ReplicatedOperation::SetFrameDisplayFilter {
            frame_id,
            filters: parsed,
            filter_match_all,
        })
    }

    pub(crate) fn prepare_set_frame_display_sort(
        &self,
        frame_id: Id,
        keys: Vec<DerivedSort>,
    ) -> Result<ReplicatedOperation, CoreError> {
        validate_sort_keys(self.frame(&frame_id)?, &keys)?;
        Ok(ReplicatedOperation::SetFrameDisplaySort { frame_id, keys })
    }

    pub(crate) fn prepare_set_frame_summary_rows(
        &self,
        frame_id: Id,
        summary_rows: Vec<SummaryOperation>,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.frame(&frame_id)?;
        let mut unique_rows = Vec::new();
        for operation in summary_rows {
            if !unique_rows.contains(&operation) {
                unique_rows.push(operation);
            }
        }
        Ok(ReplicatedOperation::SetFrameSummaryRows {
            frame_id,
            summary_rows: Some(unique_rows),
        })
    }

    pub(crate) fn prepare_set_frame_summary_drawer(
        &self,
        frame_id: Id,
        open: bool,
        height: Option<f64>,
    ) -> Result<ReplicatedOperation, CoreError> {
        self.frame(&frame_id)?;
        if height.is_some_and(|value| !value.is_finite()) {
            return Err(CoreError::InvalidOperation(
                "A profile drawer height must be a finite number".into(),
            ));
        }
        Ok(ReplicatedOperation::SetFrameSummaryDrawer {
            frame_id,
            open,
            height: height.map(|value| value.clamp(72.0, 600.0)),
        })
    }

    pub(crate) fn prepare_set_frame_style(
        &self,
        frame_id: Id,
        target: FrameStyleTarget,
        style: FrameCellStyle,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        validate_frame_style_target(frame, &target)?;
        validate_frame_cell_style(&style)?;
        Ok(ReplicatedOperation::SetFrameStyle {
            frame_id,
            target,
            style,
        })
    }

    pub(crate) fn prepare_set_frame_style_rules(
        &self,
        frame_id: Id,
        rules: Vec<FrameStyleRuleInput>,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let mut prepared = Vec::with_capacity(rules.len());
        let mut ids = std::collections::HashSet::new();
        for rule in rules {
            let rule_id = rule.id.unwrap_or_else(id);
            if !ids.insert(rule_id.clone()) {
                return Err(CoreError::InvalidOperation(
                    "Conditional-formatting rule ids must be unique".into(),
                ));
            }
            if let Some(column_id) = &rule.column_id
                && !frame.columns.iter().any(|column| column.id == *column_id)
            {
                return Err(CoreError::InvalidOperation(
                    "A conditional-formatting rule names a missing column".into(),
                ));
            }
            let expression = self.prepare_formula_for_frame(&frame.id, &rule.formula)?;
            let data_type = frame
                .infer_polars_expression_type(self, &expression)
                .map_err(CoreError::Formula)?;
            validate_frame_style_output(&rule.output, data_type)?;
            prepared.push(FrameStyleRule {
                id: rule_id,
                formula: Formula { expression },
                column_id: rule.column_id,
                output: rule.output,
            });
        }
        Ok(ReplicatedOperation::SetFrameStyleRules {
            frame_id,
            rules: prepared,
        })
    }

    pub(crate) fn prepare_promote_display_to_steps(
        &self,
        frame_id: Id,
    ) -> Result<ReplicatedOperation, CoreError> {
        validate_promotable_display(self.frame(&frame_id)?)?;
        Ok(ReplicatedOperation::PromoteDisplayToSteps { frame_id })
    }
}
