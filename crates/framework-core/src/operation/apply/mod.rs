//! Applying already-resolved `ReplicatedOperation`s in this family.
//!
//! This file is the dispatch frame only; each arm delegates to the family
//! module that owns it.

pub mod blocks;
pub mod cells;
pub mod columns;
pub mod derivation;
pub mod objects;
pub mod views;

use crate::*;

impl Document {
    pub(crate) fn apply_replicated(
        &mut self,
        operation: ReplicatedOperation,
    ) -> Result<(), CoreError> {
        match operation {
            ReplicatedOperation::AddObject {
                object,
                view,
                container_id,
            } => self.apply_add_object(object, view, container_id)?,
            ReplicatedOperation::RefreshFrameArtifact {
                frame_id,
                artifact,
                columns,
                base_columns,
            } => self.apply_refresh_frame_artifact(frame_id, artifact, columns, base_columns)?,
            ReplicatedOperation::SetFrameSource {
                frame_id,
                artifact,
                connector,
                columns,
                base_columns,
            } => {
                self.apply_set_frame_source(frame_id, artifact, connector, columns, base_columns)?
            }
            ReplicatedOperation::RenameObject {
                object_id,
                name,
                blocks,
            } => {
                self.apply_rename_object(object_id, name)?;
                for (block_id, lines) in blocks {
                    self.apply_set_block_lines(block_id, lines)?;
                }
            }
            ReplicatedOperation::DeleteObject { object_id } => {
                self.apply_delete_object(object_id)?
            }
            ReplicatedOperation::SetValue { object_id, raw } => {
                self.apply_set_value(object_id, raw)?
            }
            ReplicatedOperation::SetResultFormula { object_id, formula } => {
                self.apply_set_result_formula(object_id, formula)?
            }
            ReplicatedOperation::SetBlockLines { block_id, lines } => {
                self.apply_set_block_lines(block_id, lines)?
            }
            ReplicatedOperation::SetSeries {
                object_id,
                values,
                data_type,
            } => self.apply_set_series(object_id, values, data_type)?,
            ReplicatedOperation::SetSeriesType {
                object_id,
                data_type,
            } => self.apply_set_series_type(object_id, data_type)?,
            ReplicatedOperation::SetContainerMembers { members } => {
                self.apply_set_container_members(members)?
            }
            ReplicatedOperation::SetPlotSpec { plot_id, spec } => {
                self.apply_set_plot_spec(plot_id, spec)?
            }
            ReplicatedOperation::MoveView { view_id, x, y } => {
                self.apply_move_view(view_id, x, y)?
            }
            ReplicatedOperation::ResizeView {
                view_id,
                width,
                height,
            } => self.apply_resize_view(view_id, width, height)?,
            ReplicatedOperation::SetViewCollapsed { view_id, collapsed } => {
                self.apply_set_view_collapsed(view_id, collapsed)?
            }
            ReplicatedOperation::SetViewLayout { placements } => {
                self.apply_set_view_layout(placements)?
            }
            ReplicatedOperation::SetFrameDisplayOrientation {
                frame_id,
                orientation,
            } => self.apply_set_frame_display_orientation(frame_id, orientation)?,
            ReplicatedOperation::SetFrameDisplayCrosstab { frame_id, crosstab } => {
                self.apply_set_frame_display_crosstab(frame_id, crosstab)?
            }
            ReplicatedOperation::AddTab { view_id, object } => {
                self.apply_add_tab(view_id, object)?
            }
            ReplicatedOperation::MoveTab {
                source_view_id,
                target_view_id,
                object_id,
                target_index,
            } => self.apply_move_tab(source_view_id, target_view_id, object_id, target_index)?,
            ReplicatedOperation::DetachTab {
                source_view_id,
                object_id,
                new_view,
            } => self.apply_detach_tab(source_view_id, object_id, new_view)?,
            ReplicatedOperation::SetActiveTab { view_id, object_id } => {
                self.apply_set_active_tab(view_id, object_id)?
            }
            ReplicatedOperation::SetFrameDisplayFilter {
                frame_id,
                filters,
                filter_match_all,
            } => self.apply_set_frame_display_filter(frame_id, filters, filter_match_all)?,
            ReplicatedOperation::SetFrameDisplaySort { frame_id, keys } => {
                self.apply_set_frame_display_sort(frame_id, keys)?
            }
            operation @ (ReplicatedOperation::SetFrameSummaryRows { .. }
            | ReplicatedOperation::SetFrameSummaryDrawer { .. }) => {
                self.apply_frame_summary_operation(operation)?
            }
            ReplicatedOperation::SetFrameStyle {
                frame_id,
                target,
                style,
            } => self.apply_set_frame_style(frame_id, target, style)?,
            ReplicatedOperation::SetFrameStyleRules { frame_id, rules } => {
                self.apply_set_frame_style_rules(frame_id, rules)?
            }
            ReplicatedOperation::PromoteDisplayToSteps { frame_id } => {
                self.apply_promote_display_to_steps(frame_id)?
            }
            ReplicatedOperation::SetCell {
                frame_id,
                row_id,
                column_id,
                raw,
            } => self.apply_set_cell(frame_id, row_id, column_id, raw)?,
            ReplicatedOperation::SetCells { frame_id, cells } => {
                self.apply_set_cells(frame_id, cells)?
            }
            ReplicatedOperation::AddRow {
                frame_id,
                row,
                after_row_id,
            } => self.apply_add_row(frame_id, row, after_row_id)?,
            ReplicatedOperation::SetFrameContent {
                frame_id,
                columns,
                rows,
            } => self.apply_set_frame_content(frame_id, columns, rows)?,
            ReplicatedOperation::AdoptFrameRows { frame_id, artifact } => {
                self.apply_adopt_frame_rows(frame_id, artifact)?
            }
            ReplicatedOperation::PackageDocument { unlinked, adopted } => {
                self.apply_package_document(unlinked, adopted)?
            }
            ReplicatedOperation::SetArtifactCell {
                frame_id,
                row_ordinal,
                column_id,
                raw,
            } => self.apply_set_artifact_cell(frame_id, row_ordinal, column_id, raw)?,
            ReplicatedOperation::PasteCells {
                frame_id,
                cells,
                appended_rows,
            } => self.apply_paste_cells(frame_id, cells, appended_rows)?,
            ReplicatedOperation::DeleteRow { frame_id, row_id } => {
                self.apply_delete_row(frame_id, row_id)?
            }
            ReplicatedOperation::AddColumn {
                frame_id,
                column,
                after_column_id,
            } => self.apply_add_column(frame_id, column, after_column_id)?,
            ReplicatedOperation::DeleteColumn {
                frame_id,
                column_id,
            } => self.apply_delete_column(frame_id, column_id)?,
            ReplicatedOperation::RenameColumn {
                frame_id,
                column_id,
                name,
            } => self.apply_rename_column(frame_id, column_id, name)?,
            ReplicatedOperation::SetColumnType {
                frame_id,
                column_id,
                data_type,
            } => self.apply_set_column_type(frame_id, column_id, data_type)?,
            ReplicatedOperation::SetColumnCategories {
                frame_id,
                column_id,
                categories,
            } => self.apply_set_column_categories(frame_id, column_id, categories)?,
            ReplicatedOperation::SetColumnFormat {
                frame_id,
                column_id,
                format,
            } => self.apply_set_column_format(frame_id, column_id, format)?,
            ReplicatedOperation::SetColumnFormula {
                frame_id,
                column_id,
                formula,
                data_type,
            } => self.apply_set_column_formula(frame_id, column_id, formula, data_type)?,
            ReplicatedOperation::SetCellOverride {
                frame_id,
                row_id,
                column_id,
                formula,
            } => self.apply_set_cell_override(frame_id, row_id, column_id, formula)?,
            ReplicatedOperation::AddSummary { frame_id, summary } => {
                self.apply_add_summary(frame_id, summary)?
            }
            ReplicatedOperation::SetUniqueKeys {
                frame_id,
                unique_keys,
            } => self.apply_set_unique_keys(frame_id, unique_keys)?,
            ReplicatedOperation::SetFrameGenerator {
                frame_id,
                generator,
                columns,
            } => self.apply_set_frame_generator(frame_id, generator, columns)?,
            ReplicatedOperation::AddEntryColumn {
                frame_id,
                column,
                key_column_ids,
                entries,
                unique_key,
            } => {
                self.apply_add_entry_column(frame_id, column, key_column_ids, entries, unique_key)?
            }
            ReplicatedOperation::RemoveEntryColumn {
                frame_id,
                column_id,
            } => self.apply_remove_entry_column(frame_id, column_id)?,
            ReplicatedOperation::SetEntryValue {
                frame_id,
                column_id,
                key,
                raw,
            } => self.apply_set_entry_value(frame_id, column_id, key, raw)?,
            ReplicatedOperation::SetFrameDerivation {
                frame_id,
                name,
                columns,
                derivation,
            } => self.apply_set_frame_derivation(frame_id, name, columns, derivation)?,
            ReplicatedOperation::SetFrameSteps {
                frame_id,
                columns,
                base_columns,
                steps,
            } => self.apply_set_frame_steps(frame_id, columns, base_columns, steps)?,
            ReplicatedOperation::SetFrameMaterialization {
                frame_id,
                materialization,
            } => self.apply_set_frame_materialization(frame_id, materialization)?,
            ReplicatedOperation::SetFrameComment { frame_id, comment } => {
                self.frame_mut(&frame_id)?.comment = comment;
            }
            ReplicatedOperation::SetTextSegments {
                object_id,
                segments,
            } => {
                let text = self.text_object_mut(&object_id)?;
                text.segments = segments;
                // The legacy string is superseded the moment a real edit
                // arrives; leaving it would make it a second answer to
                // "what does this card say".
                text.text = String::new();
            }
            ReplicatedOperation::SetFrozenValue { object_id, frozen } => match frozen {
                Some(frozen) => {
                    self.frozen_values.insert(object_id, frozen);
                }
                None => {
                    self.frozen_values.remove(&object_id);
                }
            },
            ReplicatedOperation::RenameDocument { name } => self.apply_rename_document(name)?,
            ReplicatedOperation::RestoreFrame { frame } => self.apply_restore_frame(frame)?,
            ReplicatedOperation::RestoreObject { object, views } => {
                self.apply_restore_object(object, views)?
            }
            ReplicatedOperation::RestoreViews { views } => self.apply_restore_views(views)?,
        }
        self.validate_unique_keys()?;
        self.validate_join_derivations()?;
        Ok(())
    }
}
