//! Prior-value capture and subtree restoration for inverse operations.
use crate::*;

impl Document {
    /// The scenario list and the activation exactly as they stand.
    pub(crate) fn restore_scenarios(&self) -> ReplicatedOperation {
        ReplicatedOperation::RestoreScenarios {
            scenarios: self.scenarios.clone(),
            active_scenario: self.active_scenario.clone(),
        }
    }

    pub(crate) fn require_scenario_name(&self, scenario_id: &str) -> Result<String, CoreError> {
        self.scenario(scenario_id)
            .map(|scenario| scenario.name.clone())
            .ok_or_else(|| {
                CoreError::InvalidOperation("That scenario is no longer in this document.".into())
            })
    }

    /// The inverse of an edit made of splices to a frame's direct formats,
    /// as splices turned round.
    ///
    /// For a range format they are the splices it will make, read before it
    /// applies, like every inverse, so exactly the ones
    /// `apply_set_frame_style` is about to make. For splices already
    /// recorded they are resolved against the list as it stands now, so one
    /// a later edit has made inapplicable — and which will therefore be
    /// skipped — is not undone either.
    pub(crate) fn invert_frame_style_splices(
        &self,
        operation: &ReplicatedOperation,
    ) -> Result<ReplicatedOperation, CoreError> {
        let (frame_id, splices) = match operation {
            ReplicatedOperation::SetFrameStyle {
                frame_id,
                target:
                    target @ FrameStyleTarget::Range {
                        row_ids,
                        column_ids,
                    },
                style,
            } => {
                let covered = CoveredCells::new(row_ids, column_ids);
                let display = &self.frame(frame_id)?.display;
                (
                    frame_id,
                    display.range_style_splices(target, &covered, style),
                )
            }
            ReplicatedOperation::SpliceFrameStyles { frame_id, splices } => (
                frame_id,
                self.frame(frame_id)?.display.resolve_style_splices(splices),
            ),
            _ => unreachable!("only range formats and style splices are inverted as splices"),
        };
        Ok(ReplicatedOperation::SpliceFrameStyles {
            frame_id: frame_id.clone(),
            splices: invert_style_splices(&splices),
        })
    }

    /// Each named column's layout as it stands, as the same edit: a column
    /// with no entry is automatic and unwrapped, which is what setting it
    /// back to that removes.
    pub(crate) fn prior_column_layout(
        &self,
        frame_id: &Id,
        columns: &[FrameColumnLayoutUpdate],
    ) -> Result<ReplicatedOperation, CoreError> {
        let display = &self.frame(frame_id)?.display;
        Ok(ReplicatedOperation::SetFrameDisplayColumnLayout {
            frame_id: frame_id.clone(),
            columns: columns
                .iter()
                .map(|update| {
                    let prior = display
                        .column_layout
                        .get(&update.column_id)
                        .cloned()
                        .unwrap_or_default();
                    FrameColumnLayoutUpdate {
                        column_id: update.column_id.clone(),
                        width: prior.width,
                        wrap: prior.wrap,
                    }
                })
                .collect(),
        })
    }

    pub(crate) fn restore_frame(frame: &FrameObject) -> ReplicatedOperation {
        ReplicatedOperation::RestoreFrame {
            frame: frame.clone(),
        }
    }

    /// What a block of overlay patches is about to replace: each cell's
    /// prior patch, or `None` where there was none, which withdraws the patch
    /// rather than writing an empty one — the difference between a cell nobody
    /// touched and one somebody cleared. Read once per cell from the state
    /// before any of them applied, so a cell named twice in one block still
    /// goes back to what it was before the block. The row count is put back
    /// only when the forward edit set it.
    pub(crate) fn invert_set_overlay_cells(
        &self,
        frame_id: &str,
        appended_rows: Option<u32>,
        cells: &[OverlayCellPatch],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        Ok(vec![ReplicatedOperation::SetOverlayCells {
            frame_id: frame_id.to_string(),
            appended_rows: appended_rows.map(|_| frame.appended_rows),
            cells: cells
                .iter()
                .map(|cell| OverlayCellPatch {
                    row_ordinal: cell.row_ordinal,
                    column_id: cell.column_id.clone(),
                    raw: frame
                        .overlay_value(&cell.column_id, cell.row_ordinal)
                        .map(str::to_owned),
                })
                .collect(),
        }])
    }

    pub(crate) fn invert_frame_summary(
        &self,
        operation: &ReplicatedOperation,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame_id = match operation {
            ReplicatedOperation::SetFrameSummaryRows { frame_id, .. }
            | ReplicatedOperation::SetFrameSummaryDrawer { frame_id, .. } => frame_id,
            _ => unreachable!("only frame profile operations are routed here"),
        };
        let display = &self.frame(frame_id)?.display;
        Ok(vec![match operation {
            ReplicatedOperation::SetFrameSummaryRows { .. } => {
                ReplicatedOperation::SetFrameSummaryRows {
                    frame_id: frame_id.clone(),
                    summary_rows: display.summary_rows.clone(),
                }
            }
            ReplicatedOperation::SetFrameSummaryDrawer { .. } => {
                ReplicatedOperation::SetFrameSummaryDrawer {
                    frame_id: frame_id.clone(),
                    open: display.summary_drawer_open,
                    height: display.summary_drawer_height,
                }
            }
            _ => unreachable!("only frame profile operations are routed here"),
        }])
    }

    pub(crate) fn invert_delete_object(
        &self,
        object_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let object = self.object(object_id)?.clone();
        let mut inverse = vec![ReplicatedOperation::RestoreObject {
            views: self
                .views
                .iter()
                .filter(|view| view.tabs().iter().any(|tab| tab == object_id))
                .cloned()
                .collect(),
            object,
        }];
        // Deleting also takes it out of whatever container held it, so
        // putting it back has to put it back *there* — otherwise undo
        // returns the object and loses where it lived.
        if let Some(container) = self.container_of(object_id) {
            inverse.push(ReplicatedOperation::SetContainerMembers {
                members: vec![(container.id.clone(), container.member_ids.clone())],
            });
        }
        // And it takes its scenario overrides with it, which is a change to
        // every scenario at once and so has no forward operation of its own.
        // Only paid for when some scenario actually said something about it.
        if self
            .scenarios
            .iter()
            .any(|scenario| scenario.values.contains_key(object_id))
        {
            inverse.push(self.restore_scenarios());
        }
        Ok(inverse)
    }

    pub(crate) fn invert_set_block_lines(
        &self,
        block_id: &Id,
        lines: &[BlockLine],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let current = &self.block(block_id)?.lines;
        let changed = crate::operation::apply::blocks::changed_block_line_ids(current, lines);
        let mut inverse = vec![ReplicatedOperation::SetBlockLines {
            block_id: block_id.clone(),
            lines: current.clone(),
        }];
        inverse.extend(changed.into_iter().filter_map(|object_id| {
            self.frozen_values.get(&object_id).cloned().map(|frozen| {
                ReplicatedOperation::SetFrozenValue {
                    object_id,
                    frozen: Some(frozen),
                }
            })
        }));
        Ok(inverse)
    }

    pub(crate) fn invert_set_frame_display_filter(
        &self,
        frame_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let display = &self.frame(frame_id)?.display;
        let (predicates, match_all) = display.filter().unwrap_or((&[], true));
        Ok(vec![ReplicatedOperation::SetFrameDisplayFilter {
            frame_id: frame_id.clone(),
            filters: predicates
                .iter()
                .map(|expression| Formula {
                    expression: expression.clone(),
                })
                .collect(),
            filter_match_all: match_all,
        }])
    }

    pub(crate) fn invert_set_frame_display_sort(
        &self,
        frame_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let steps = frame
            .derivation
            .as_ref()
            .map(|derivation| derivation.steps())
            .unwrap_or_else(|| std::borrow::Cow::Borrowed(&frame.steps));
        Ok(vec![ReplicatedOperation::SetFrameDisplaySort {
            frame_id: frame_id.clone(),
            keys: match steps.last() {
                Some(FrameStep::Sort { keys }) => keys.clone(),
                _ => Vec::new(),
            },
        }])
    }

    /// A pasted block can add rows and columns, and `SetCells` cannot take
    /// them away again.
    ///
    /// This used to restore the whole frame, which also reverted any
    /// collaborator's edit to that frame merged between the paste and its
    /// undo. The paste is fully described by three things, so the inverse
    /// names exactly those: the overwritten cells get their prior text, the
    /// rows it appended go, and the columns it added go — `DeleteColumn`
    /// takes them out of every frame scored from this one too, which is
    /// where `insert_literal_column` carried them. Cells written into an
    /// appended row or an added column need no restoring; they leave with
    /// it. A row the paste named but that already existed was skipped by
    /// `apply_paste_cells`, so it is not the paste's to delete.
    pub(crate) fn invert_paste_cells(
        &self,
        frame_id: &Id,
        cells: &[CellUpdate],
        appended_rows: &[Row],
        added_columns: &[Column],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let new_rows: Vec<Id> = appended_rows
            .iter()
            .filter(|row| !frame.rows.iter().any(|existing| existing.id == row.id))
            .map(|row| row.id.clone())
            .collect();
        let overwritten: Vec<CellUpdate> = cells
            .iter()
            .filter(|cell| {
                !new_rows.contains(&cell.row_id)
                    && !added_columns
                        .iter()
                        .any(|column| column.id == cell.column_id)
            })
            .cloned()
            .collect();
        let mut inverse = Vec::new();
        if !overwritten.is_empty() || (new_rows.is_empty() && added_columns.is_empty()) {
            inverse.push(ReplicatedOperation::SetCells {
                frame_id: frame_id.clone(),
                cells: self.prior_cells(frame_id, &overwritten)?,
            });
        }
        if !new_rows.is_empty() {
            inverse.push(ReplicatedOperation::DeleteRows {
                frame_id: frame_id.clone(),
                row_ids: new_rows,
            });
        }
        inverse.extend(added_columns.iter().rev().map(|column| {
            ReplicatedOperation::DeleteColumn {
                frame_id: frame_id.clone(),
                column_id: column.id.clone(),
            }
        }));
        Ok(inverse)
    }

    /// Puts deleted rows back where each one stood, with its cells.
    ///
    /// One `AddRow` per row, last row first. Each names the row that followed
    /// it before the delete, and going from the bottom up means that row is
    /// already back — either it survived the delete, or it was restored a
    /// step earlier — so every row lands exactly where it was however the
    /// selection was scattered. Row styles anchored to a deleted row are not
    /// carried back, the same as undoing a single `DeleteRow` has never done.
    pub(crate) fn invert_delete_rows(
        &self,
        frame_id: &Id,
        row_ids: &[Id],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let mut indexes = row_ids
            .iter()
            .map(|row_id| {
                frame
                    .rows
                    .iter()
                    .position(|row| row.id == *row_id)
                    .ok_or(CoreError::RowNotFound)
            })
            .collect::<Result<Vec<_>, _>>()?;
        indexes.sort_unstable();
        indexes.dedup();
        Ok(indexes
            .into_iter()
            .rev()
            .map(|index| ReplicatedOperation::AddRow {
                frame_id: frame_id.clone(),
                row: frame.rows[index].clone(),
                before_row_id: frame.rows.get(index + 1).map(|next| next.id.clone()),
            })
            .collect())
    }

    // Retyping a column also rebuilds its category list, so putting the
    // type back is only half of it.
    pub(crate) fn invert_set_column_type(
        &self,
        frame_id: &Id,
        column_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let column = self.column(frame_id, column_id)?;
        let mut inverse = vec![ReplicatedOperation::SetColumnType {
            frame_id: frame_id.clone(),
            column_id: column_id.clone(),
            data_type: column.data_type,
            scale: column.scale,
        }];
        // Only a column that had a list gets it back. Setting an empty list
        // is refused, and an inverse that is refused is dropped by undo —
        // which then reaches for the edit before this one.
        if !column.categories.is_empty() {
            inverse.push(ReplicatedOperation::SetColumnCategories {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                categories: column.categories.clone(),
            });
        }
        Ok(inverse)
    }

    pub(crate) fn invert_set_cell_override(
        &self,
        frame_id: &Id,
        row_id: &Id,
        column_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let formula = frame
            .rows
            .iter()
            .find(|row| row.id == *row_id)
            .and_then(|row| row.cells.get(column_id))
            .and_then(|cell| cell.override_formula.clone());
        Ok(vec![ReplicatedOperation::SetCellOverride {
            frame_id: frame_id.clone(),
            row_id: row_id.clone(),
            column_id: column_id.clone(),
            formula,
        }])
    }

    pub(crate) fn invert_add_entry_column(
        &self,
        frame_id: &Id,
        column: &Column,
        unique_key: &Option<UniqueKeyConstraint>,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let mut inverse = vec![ReplicatedOperation::RemoveEntryColumn {
            frame_id: frame_id.clone(),
            column_id: column.id.clone(),
        }];
        // An add that minted a unique key gives it back on undo — the key
        // list as it stands now is the one to restore.
        if unique_key.is_some() {
            inverse.push(ReplicatedOperation::SetUniqueKeys {
                frame_id: frame_id.clone(),
                unique_keys: self.frame(frame_id)?.unique_keys.clone(),
            });
        }
        Ok(inverse)
    }

    pub(crate) fn invert_remove_entry_column(
        &self,
        frame_id: &Id,
        column_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let column = frame
            .columns
            .iter()
            .find(|column| column.id == *column_id)
            .ok_or(CoreError::ColumnNotFound)?
            .clone();
        let entry_column = frame
            .entry_columns
            .iter()
            .find(|entry_column| entry_column.column_id == *column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        Ok(vec![ReplicatedOperation::AddEntryColumn {
            frame_id: frame_id.clone(),
            column,
            key_column_ids: entry_column.key_column_ids.clone(),
            entries: entry_column.entries.clone(),
            // The key survived the removal, so putting the column back
            // mints nothing.
            unique_key: None,
        }])
    }

    pub(crate) fn invert_set_entry_value(
        &self,
        frame_id: &Id,
        column_id: &Id,
        key: &[String],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let previous = self
            .frame(frame_id)?
            .entry_columns
            .iter()
            .find(|entry_column| entry_column.column_id == *column_id)
            .and_then(|entry_column| entry_column.entries.iter().find(|entry| entry.key == key))
            .map(|entry| entry.raw.clone())
            .unwrap_or_default();
        Ok(vec![ReplicatedOperation::SetEntryValue {
            frame_id: frame_id.clone(),
            column_id: column_id.clone(),
            key: key.to_vec(),
            raw: previous,
        }])
    }

    // The value that is there now is the value to put back. Read from the
    // file rather than carried in the operation, for the same reason every
    // other inverse is read at apply time: the operation describes what to
    // do, and the document is what knows what it is about to lose.
    pub(crate) fn invert_set_artifact_cell(
        &self,
        frame_id: &Id,
        row_ordinal: usize,
        column_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let frame = self.frame(frame_id)?;
        let column = frame
            .columns
            .iter()
            .find(|column| column.id == *column_id)
            .ok_or(CoreError::ColumnNotFound)?;
        let artifact = frame
            .artifact
            .as_ref()
            .ok_or_else(|| CoreError::InvalidOperation("This frame has no data file".into()))?;
        Ok(vec![ReplicatedOperation::SetArtifactCell {
            frame_id: frame_id.clone(),
            row_ordinal,
            column_id: column_id.clone(),
            raw: read_artifact_cell(artifact, &column.name, row_ordinal)?,
        }])
    }

    pub(crate) fn invert_restore_object(&self, object: &DataObject) -> Vec<ReplicatedOperation> {
        let object_id = object.id();
        match self
            .objects
            .iter()
            .find(|existing| existing.id() == object_id)
        {
            Some(existing) => vec![ReplicatedOperation::RestoreObject {
                object: existing.clone(),
                views: self
                    .views
                    .iter()
                    .filter(|view| view.tabs().iter().any(|tab| tab == object_id))
                    .cloned()
                    .collect(),
            }],
            None => vec![ReplicatedOperation::DeleteObject {
                object_id: object_id.to_string(),
            }],
        }
    }

    pub(crate) fn restore_views(&self) -> ReplicatedOperation {
        ReplicatedOperation::RestoreViews {
            views: self.views.clone(),
        }
    }

    pub(crate) fn column(&self, frame_id: &str, column_id: &str) -> Result<&Column, CoreError> {
        self.frame(frame_id)?
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)
    }

    pub(crate) fn cell_raw(
        &self,
        frame_id: &str,
        row_id: &str,
        column_id: &str,
    ) -> Result<String, CoreError> {
        Ok(self
            .frame(frame_id)?
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .and_then(|row| row.cells.get(column_id))
            .map(|cell| cell.raw.clone())
            .unwrap_or_default())
    }

    pub(crate) fn prior_cells(
        &self,
        frame_id: &str,
        cells: &[CellUpdate],
    ) -> Result<Vec<CellUpdate>, CoreError> {
        cells
            .iter()
            .map(|update| {
                Ok(CellUpdate {
                    row_id: update.row_id.clone(),
                    column_id: update.column_id.clone(),
                    raw: self.cell_raw(frame_id, &update.row_id, &update.column_id)?,
                })
            })
            .collect()
    }
}
