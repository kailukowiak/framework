//! What it takes to undo one edit.
//!
//! Every applied operation gets an inverse computed here, against the
//! document *before* it applies — the prior value has to be read while it
//! still exists. Undo then applies that inverse forward, like any other
//! edit, which is the whole point: history is a list of operations rather
//! than a list of document snapshots, so a remote edit landing in the
//! middle no longer invalidates it, and undoing reverts one edit instead of
//! every difference between two snapshots.
//!
//! Most operations invert to *themselves*, replayed with the values they
//! replaced: the inverse of moving a card is moving it back, the inverse of
//! setting a cell is setting the old text. Where that is impossible the
//! inverse restores the affected subtree instead — see the `Restore*`
//! operations. Those are the cases where the forward edit destroyed
//! something no operation can describe:
//!
//! * dropping a column also drops its summaries and its cells' one-off
//!   overrides, so `AddColumn` + `SetCells` would put back a column that has
//!   quietly lost things;
//! * rebuilding a frame's content or chain leaves no earlier shape for the
//!   same operation to name;
//! * the tab operations add and remove whole cards as strips empty and
//!   fill, so no single tab operation reverses one.
//!
//! A restore carries one frame, one object, or the view list — never the
//! document. That bound is what makes this cheaper than the snapshots it
//! replaces, not just more correct: a literal frame's rows are small by
//! construction, and an imported frame keeps none in the document at all.

use crate::*;

impl Document {
    /// The operations that put this document back as it is now, if
    /// `operation` were applied to it.
    ///
    /// Read before applying, never after.
    pub(crate) fn invert(
        &self,
        operation: &ReplicatedOperation,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        Ok(match operation {
            ReplicatedOperation::AddObject { object, .. } => {
                vec![ReplicatedOperation::DeleteObject {
                    object_id: object.id().to_string(),
                }]
            }

            ReplicatedOperation::RefreshFrameArtifact { frame_id, .. } => {
                let frame = self.frame(frame_id)?;
                match &frame.artifact {
                    Some(artifact) => vec![ReplicatedOperation::RefreshFrameArtifact {
                        frame_id: frame_id.clone(),
                        artifact: artifact.clone(),
                        columns: frame.columns.clone(),
                        base_columns: frame.base_columns.clone(),
                    }],
                    None => vec![Self::restore_frame(frame)],
                }
            }

            // The prior connector may have been absent, which `SetFrameSource`
            // cannot express — it always sets one.
            ReplicatedOperation::SetFrameSource { frame_id, .. } => {
                vec![Self::restore_frame(self.frame(frame_id)?)]
            }

            ReplicatedOperation::RenameObject {
                object_id, blocks, ..
            } => {
                let object = self.object(object_id)?;
                // The old name, and the text as it read under the old name.
                // Read off the document as it stands, which is before the
                // rename — so this is simply what those blocks say now.
                vec![ReplicatedOperation::RenameObject {
                    object_id: object_id.clone(),
                    name: object.name().to_string(),
                    blocks: blocks
                        .iter()
                        .map(|(block_id, _)| {
                            Ok((block_id.clone(), self.block(block_id)?.lines.clone()))
                        })
                        .collect::<Result<Vec<_>, CoreError>>()?,
                }]
            }

            ReplicatedOperation::DeleteObject { object_id } => {
                self.invert_delete_object(object_id)?
            }

            ReplicatedOperation::SetValue { object_id, .. } => {
                let DataObject::Value(value) = self.object(object_id)? else {
                    return Err(CoreError::ObjectNotFound);
                };
                vec![ReplicatedOperation::SetValue {
                    object_id: object_id.clone(),
                    raw: value.raw.clone(),
                }]
            }

            ReplicatedOperation::SetResultFormula { object_id, .. } => {
                let DataObject::Result(result) = self.object(object_id)? else {
                    return Err(CoreError::ObjectNotFound);
                };
                vec![ReplicatedOperation::SetResultFormula {
                    object_id: object_id.clone(),
                    formula: result.formula.clone(),
                }]
            }

            // The whole list back as it stood, ids included — so undoing a
            // retype restores the lines other formulas were pointing at,
            // not lookalikes of them.
            ReplicatedOperation::SetBlockLines { block_id, lines } => {
                self.invert_set_block_lines(block_id, lines)?
            }

            // Inverting to the *effective* segments rather than the stored
            // ones means undo restores what the card said even when what it
            // said was still held in the legacy string.
            ReplicatedOperation::SetTextSegments { object_id, .. } => {
                vec![ReplicatedOperation::SetTextSegments {
                    object_id: object_id.clone(),
                    segments: self.text_object(object_id)?.effective_segments(),
                }]
            }

            ReplicatedOperation::SetSeries { object_id, .. }
            | ReplicatedOperation::SetSeriesType { object_id, .. } => {
                let DataObject::Series(series) = self.object(object_id)? else {
                    return Err(CoreError::ObjectNotFound);
                };
                vec![ReplicatedOperation::SetSeries {
                    object_id: object_id.clone(),
                    values: series.values.clone(),
                    data_type: series.data_type,
                }]
            }

            // Both containers the move touched, put back as they were. A
            // move names the two lists it rewrites, so its inverse is the
            // same operation with the lists it found.
            ReplicatedOperation::SetContainerMembers { members } => {
                let mut previous = Vec::with_capacity(members.len());
                for (container_id, _) in members {
                    let DataObject::Container(container) = self.object(container_id)? else {
                        return Err(CoreError::ObjectNotFound);
                    };
                    previous.push((container_id.clone(), container.member_ids.clone()));
                }
                vec![ReplicatedOperation::SetContainerMembers { members: previous }]
            }

            ReplicatedOperation::SetPlotSpec { plot_id, .. } => {
                let DataObject::Plot(plot) = self.object(plot_id)? else {
                    return Err(CoreError::ObjectNotFound);
                };
                vec![ReplicatedOperation::SetPlotSpec {
                    plot_id: plot_id.clone(),
                    spec: plot.spec.clone(),
                }]
            }

            ReplicatedOperation::MoveView { view_id, .. } => {
                let view = self.view(view_id)?;
                vec![ReplicatedOperation::MoveView {
                    view_id: view_id.clone(),
                    x: view.x,
                    y: view.y,
                }]
            }

            ReplicatedOperation::ResizeView { view_id, .. } => {
                let view = self.view(view_id)?;
                vec![ReplicatedOperation::ResizeView {
                    view_id: view_id.clone(),
                    width: view.width,
                    height: view.height,
                }]
            }

            ReplicatedOperation::SetViewCollapsed { view_id, .. } => {
                vec![ReplicatedOperation::SetViewCollapsed {
                    view_id: view_id.clone(),
                    collapsed: self.view(view_id)?.collapsed,
                }]
            }

            // Every window's prior position, not just the ones that moved:
            // tidying is one edit, so undoing it is one edit too.
            ReplicatedOperation::SetViewLayout { .. } => {
                vec![ReplicatedOperation::SetViewLayout {
                    placements: self
                        .views
                        .iter()
                        .map(|view| ViewPlacement {
                            view_id: view.id.clone(),
                            x: view.x,
                            y: view.y,
                        })
                        .collect(),
                }]
            }

            ReplicatedOperation::SetFrameDisplayOrientation { frame_id, .. } => {
                vec![ReplicatedOperation::SetFrameDisplayOrientation {
                    frame_id: frame_id.clone(),
                    orientation: self.frame(frame_id)?.display.orientation,
                }]
            }

            ReplicatedOperation::SetFrameDisplayCrosstab { frame_id, .. } => {
                vec![ReplicatedOperation::SetFrameDisplayCrosstab {
                    frame_id: frame_id.clone(),
                    crosstab: self.frame(frame_id)?.display.crosstab.clone(),
                }]
            }

            // A tab arriving adds an object *and* rearranges a strip; the
            // strip goes back wholesale, and the object it brought goes away.
            ReplicatedOperation::AddTab { object, .. } => {
                vec![
                    ReplicatedOperation::DeleteObject {
                        object_id: object.id().to_string(),
                    },
                    Self::restore_views(self),
                ]
            }

            ReplicatedOperation::MoveTab { .. } | ReplicatedOperation::DetachTab { .. } => {
                vec![Self::restore_views(self)]
            }

            ReplicatedOperation::SetActiveTab { view_id, .. } => {
                vec![ReplicatedOperation::SetActiveTab {
                    view_id: view_id.clone(),
                    object_id: self.view(view_id)?.object_id.clone(),
                }]
            }

            ReplicatedOperation::SetFrameDisplayFilter { frame_id, .. } => {
                self.invert_set_frame_display_filter(frame_id)?
            }

            ReplicatedOperation::SetFrameDisplaySort { frame_id, .. } => {
                self.invert_set_frame_display_sort(frame_id)?
            }

            operation @ (ReplicatedOperation::SetFrameSummaryRows { .. }
            | ReplicatedOperation::SetFrameSummaryDrawer { .. }) => {
                self.invert_frame_summary(operation)?
            }

            // An absent style is restored by setting an empty one, which is
            // how `apply_set_frame_style` spells removal.
            ReplicatedOperation::SetFrameStyle {
                frame_id, target, ..
            } => {
                let style = self
                    .frame(frame_id)?
                    .display
                    .styles
                    .iter()
                    .find(|entry| entry.target == *target)
                    .map(|entry| entry.style.clone())
                    .unwrap_or_default();
                vec![ReplicatedOperation::SetFrameStyle {
                    frame_id: frame_id.clone(),
                    target: target.clone(),
                    style,
                }]
            }

            ReplicatedOperation::SetFrameStyleRules { frame_id, .. } => {
                vec![ReplicatedOperation::SetFrameStyleRules {
                    frame_id: frame_id.clone(),
                    rules: self.frame(frame_id)?.display.style_rules.clone(),
                }]
            }

            // Promotion moves the display layer into the chain; nothing
            // moves it back out.
            ReplicatedOperation::PromoteDisplayToSteps { frame_id } => {
                vec![Self::restore_frame(self.frame(frame_id)?)]
            }

            ReplicatedOperation::SetCell {
                frame_id,
                row_id,
                column_id,
                ..
            } => {
                vec![ReplicatedOperation::SetCell {
                    frame_id: frame_id.clone(),
                    row_id: row_id.clone(),
                    column_id: column_id.clone(),
                    raw: self.cell_raw(frame_id, row_id, column_id)?,
                }]
            }

            ReplicatedOperation::SetCells { frame_id, cells } => {
                vec![ReplicatedOperation::SetCells {
                    frame_id: frame_id.clone(),
                    cells: self.prior_cells(frame_id, cells)?,
                }]
            }

            ReplicatedOperation::AddRow { frame_id, row, .. } => {
                vec![ReplicatedOperation::DeleteRow {
                    frame_id: frame_id.clone(),
                    row_id: row.id.clone(),
                }]
            }

            // A pasted block can add rows, and `SetCells` cannot take them
            // away again.
            ReplicatedOperation::PasteCells {
                frame_id,
                cells,
                appended_rows,
            } => self.invert_paste_cells(frame_id, cells, appended_rows)?,

            ReplicatedOperation::SetFrameContent { frame_id, .. }
            | ReplicatedOperation::SetFrameDerivation { frame_id, .. }
            | ReplicatedOperation::SetFrameSteps { frame_id, .. }
            | ReplicatedOperation::DeleteColumn { frame_id, .. }
            | ReplicatedOperation::AddSummary { frame_id, .. } => {
                vec![Self::restore_frame(self.frame(frame_id)?)]
            }

            // The row goes back where it was, which is what `after_row_id`
            // is for.
            ReplicatedOperation::DeleteRow { frame_id, row_id } => {
                let frame = self.frame(frame_id)?;
                let index = frame
                    .rows
                    .iter()
                    .position(|row| row.id == *row_id)
                    .ok_or(CoreError::RowNotFound)?;
                vec![ReplicatedOperation::AddRow {
                    frame_id: frame_id.clone(),
                    row: frame.rows[index].clone(),
                    after_row_id: index
                        .checked_sub(1)
                        .map(|previous| frame.rows[previous].id.clone()),
                }]
            }

            ReplicatedOperation::AddColumn {
                frame_id, column, ..
            } => {
                vec![ReplicatedOperation::DeleteColumn {
                    frame_id: frame_id.clone(),
                    column_id: column.id.clone(),
                }]
            }

            ReplicatedOperation::RenameColumn {
                frame_id,
                column_id,
                ..
            } => {
                vec![ReplicatedOperation::RenameColumn {
                    frame_id: frame_id.clone(),
                    column_id: column_id.clone(),
                    name: self.column(frame_id, column_id)?.name.clone(),
                }]
            }

            // Retyping a column also rebuilds its category list, so putting
            // the type back is only half of it.
            ReplicatedOperation::SetColumnType {
                frame_id,
                column_id,
                ..
            } => self.invert_set_column_type(frame_id, column_id)?,

            ReplicatedOperation::SetColumnCategories {
                frame_id,
                column_id,
                ..
            } => {
                vec![ReplicatedOperation::SetColumnCategories {
                    frame_id: frame_id.clone(),
                    column_id: column_id.clone(),
                    categories: self.column(frame_id, column_id)?.categories.clone(),
                }]
            }

            ReplicatedOperation::SetColumnFormat {
                frame_id,
                column_id,
                ..
            } => {
                vec![ReplicatedOperation::SetColumnFormat {
                    frame_id: frame_id.clone(),
                    column_id: column_id.clone(),
                    format: self.column(frame_id, column_id)?.format.clone(),
                }]
            }

            // A column that had no formula cannot be described by
            // `SetColumnFormula`, which always sets one.
            ReplicatedOperation::SetColumnFormula {
                frame_id,
                column_id,
                ..
            } => {
                let column = self.column(frame_id, column_id)?;
                match &column.formula {
                    Some(formula) => vec![ReplicatedOperation::SetColumnFormula {
                        frame_id: frame_id.clone(),
                        column_id: column_id.clone(),
                        formula: formula.clone(),
                        data_type: column.data_type,
                    }],
                    None => vec![Self::restore_frame(self.frame(frame_id)?)],
                }
            }

            ReplicatedOperation::SetCellOverride {
                frame_id,
                row_id,
                column_id,
                ..
            } => self.invert_set_cell_override(frame_id, row_id, column_id)?,

            ReplicatedOperation::SetUniqueKeys { frame_id, .. } => {
                vec![ReplicatedOperation::SetUniqueKeys {
                    frame_id: frame_id.clone(),
                    unique_keys: self.frame(frame_id)?.unique_keys.clone(),
                }]
            }

            ReplicatedOperation::AddEntryColumn {
                frame_id,
                column,
                unique_key,
                ..
            } => self.invert_add_entry_column(frame_id, column, unique_key)?,

            ReplicatedOperation::RemoveEntryColumn {
                frame_id,
                column_id,
            } => self.invert_remove_entry_column(frame_id, column_id)?,

            ReplicatedOperation::SetEntryValue {
                frame_id,
                column_id,
                key,
                ..
            } => self.invert_set_entry_value(frame_id, column_id, key)?,

            ReplicatedOperation::SetFrameGenerator { frame_id, .. } => {
                let frame = self.frame(frame_id)?;
                let generator = frame.generator.clone().ok_or_else(|| {
                    CoreError::InvalidOperation(
                        "Only a generated frame has a rule to replace".into(),
                    )
                })?;
                vec![ReplicatedOperation::SetFrameGenerator {
                    frame_id: frame_id.clone(),
                    generator,
                    columns: frame.columns.clone(),
                }]
            }

            ReplicatedOperation::SetFrameMaterialization { frame_id, .. } => {
                vec![ReplicatedOperation::SetFrameMaterialization {
                    frame_id: frame_id.clone(),
                    materialization: self.frame(frame_id)?.materialization.clone(),
                }]
            }

            ReplicatedOperation::SetFrameComment { frame_id, .. } => {
                vec![ReplicatedOperation::SetFrameComment {
                    frame_id: frame_id.clone(),
                    comment: self.frame(frame_id)?.comment.clone(),
                }]
            }

            ReplicatedOperation::SetFrozenValue { object_id, .. } => {
                vec![ReplicatedOperation::SetFrozenValue {
                    object_id: object_id.clone(),
                    frozen: self.frozen_values.get(object_id).cloned(),
                }]
            }

            ReplicatedOperation::RenameDocument { .. } => {
                vec![ReplicatedOperation::RenameDocument {
                    name: self.name.clone(),
                }]
            }

            // Taking ownership rewrites what a frame *is* — its chain, its
            // connector, where its values come from — so the way back is the
            // frame as it stood. The artifact it adopted stays on disk;
            // nothing else points at it, and redo will want it.
            ReplicatedOperation::AdoptFrameRows { frame_id, .. } => {
                vec![Self::restore_frame(self.frame(frame_id)?)]
            }

            // One entry in history, one frame restored per frame it touched.
            // Packaging a document with a dozen connections in it should cost
            // one undo, not a dozen — and with the stack bounded, a dozen
            // would not all fit anyway.
            ReplicatedOperation::PackageDocument { unlinked, adopted } => unlinked
                .iter()
                .chain(adopted.iter().map(|(frame_id, _)| frame_id))
                .map(|frame_id| self.frame(frame_id).map(Self::restore_frame))
                .collect::<Result<Vec<_>, _>>()?,

            // The value that is there now is the value to put back. Read
            // from the file rather than carried in the operation, for the
            // same reason every other inverse is read at apply time: the
            // operation describes what to do, and the document is what knows
            // what it is about to lose.
            ReplicatedOperation::SetArtifactCell {
                frame_id,
                row_ordinal,
                column_id,
                ..
            } => self.invert_set_artifact_cell(frame_id, *row_ordinal, column_id)?,

            // A restore is already a prior state, so inverting one is
            // capturing the state it is about to replace.
            ReplicatedOperation::RestoreFrame { frame } => {
                vec![Self::restore_frame(self.frame(&frame.id)?)]
            }
            ReplicatedOperation::RestoreObject { object, .. } => self.invert_restore_object(object),
            ReplicatedOperation::RestoreViews { .. } => vec![Self::restore_views(self)],
        })
    }

    fn restore_frame(frame: &FrameObject) -> ReplicatedOperation {
        ReplicatedOperation::RestoreFrame {
            frame: frame.clone(),
        }
    }

    fn invert_frame_summary(
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

    fn invert_delete_object(&self, object_id: &Id) -> Result<Vec<ReplicatedOperation>, CoreError> {
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
        Ok(inverse)
    }

    fn invert_set_block_lines(
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

    fn invert_set_frame_display_filter(
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

    fn invert_set_frame_display_sort(
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

    // A pasted block can add rows, and `SetCells` cannot take them away
    // again.
    fn invert_paste_cells(
        &self,
        frame_id: &Id,
        cells: &[CellUpdate],
        appended_rows: &[Row],
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        if appended_rows.is_empty() {
            Ok(vec![ReplicatedOperation::SetCells {
                frame_id: frame_id.clone(),
                cells: self.prior_cells(frame_id, cells)?,
            }])
        } else {
            Ok(vec![Self::restore_frame(self.frame(frame_id)?)])
        }
    }

    // Retyping a column also rebuilds its category list, so putting the
    // type back is only half of it.
    fn invert_set_column_type(
        &self,
        frame_id: &Id,
        column_id: &Id,
    ) -> Result<Vec<ReplicatedOperation>, CoreError> {
        let column = self.column(frame_id, column_id)?;
        Ok(vec![
            ReplicatedOperation::SetColumnType {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                data_type: column.data_type,
            },
            ReplicatedOperation::SetColumnCategories {
                frame_id: frame_id.clone(),
                column_id: column_id.clone(),
                categories: column.categories.clone(),
            },
        ])
    }

    fn invert_set_cell_override(
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

    fn invert_add_entry_column(
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

    fn invert_remove_entry_column(
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

    fn invert_set_entry_value(
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
    fn invert_set_artifact_cell(
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

    fn invert_restore_object(&self, object: &DataObject) -> Vec<ReplicatedOperation> {
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

    fn restore_views(&self) -> ReplicatedOperation {
        ReplicatedOperation::RestoreViews {
            views: self.views.clone(),
        }
    }

    fn column(&self, frame_id: &str, column_id: &str) -> Result<&Column, CoreError> {
        self.frame(frame_id)?
            .columns
            .iter()
            .find(|column| column.id == column_id)
            .ok_or(CoreError::ColumnNotFound)
    }

    fn cell_raw(&self, frame_id: &str, row_id: &str, column_id: &str) -> Result<String, CoreError> {
        Ok(self
            .frame(frame_id)?
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .and_then(|row| row.cells.get(column_id))
            .map(|cell| cell.raw.clone())
            .unwrap_or_default())
    }

    fn prior_cells(
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
