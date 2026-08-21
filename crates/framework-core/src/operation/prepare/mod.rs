//! Resolving `Operation`s in this family into fully determined
//!
//! This file is the dispatch frame only; each arm delegates to the family
//! module that owns it.

pub mod blocks;
pub mod cells;
pub mod columns;
pub mod derivation;
pub mod objects;
pub mod pass_through;
pub mod views;

use crate::*;

impl Document {
    pub(crate) fn prepare_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(match operation {
            Operation::AddValue {
                name,
                raw,
                x,
                y,
                container_id,
            } => self.prepare_add_value(name, raw, x, y, container_id)?,
            Operation::AddResult {
                name,
                formula,
                x,
                y,
                container_id,
            } => self.prepare_add_result(name, formula, x, y, container_id)?,
            Operation::SetResultFormula { object_id, formula } => {
                self.prepare_set_result_formula(object_id, formula)?
            }
            Operation::AddBlock { name, x, y } => self.prepare_add_block(name, x, y)?,
            Operation::AddText { x, y } => self.prepare_add_text(x, y)?,
            Operation::SetTextSource { object_id, source } => {
                self.prepare_set_text_source(object_id, source)?
            }
            Operation::SetBlockSource {
                block_id,
                source,
                editing,
            } => self.prepare_set_block_source(block_id, source, editing)?,
            Operation::AddContainer {
                name,
                x,
                y,
                container_id,
            } => self.prepare_add_container(name, x, y, container_id)?,
            Operation::MoveIntoContainer {
                object_id,
                container_id,
            } => self.prepare_move_into_container(object_id, container_id)?,
            Operation::AddSeries {
                name,
                values,
                x,
                y,
                container_id,
            } => self.prepare_add_series(name, values, x, y, container_id)?,
            Operation::ImportSeriesFromFile {
                name,
                path,
                column,
                x,
                y,
                container_id,
            } => self.prepare_import_series_from_file(name, path, column, x, y, container_id)?,
            Operation::SetSeries { object_id, values } => {
                self.prepare_set_series(object_id, values)?
            }
            Operation::SetSeriesType {
                object_id,
                data_type,
            } => ReplicatedOperation::SetSeriesType {
                object_id,
                data_type,
            },
            Operation::AddFrame { name, grid, x, y } => self.prepare_add_frame(name, grid, x, y)?,
            Operation::AddGeneratorFrame {
                name,
                formula,
                column_name,
                x,
                y,
            } => self.prepare_add_generator_frame(name, formula, column_name, x, y)?,
            Operation::SetFrameGenerator { frame_id, formula } => {
                self.prepare_set_frame_generator(frame_id, formula)?
            }
            Operation::AddEntryColumn {
                frame_id,
                name,
                data_type,
                key_column_ids,
            } => self.prepare_add_entry_column(frame_id, name, data_type, key_column_ids)?,
            Operation::SetEntryValue {
                frame_id,
                column_id,
                key,
                raw,
            } => self.prepare_set_entry_value(frame_id, column_id, key, raw)?,
            Operation::RefreshFramePipeline { frame_id } => {
                self.prepare_refresh_frame_pipeline(frame_id)?
            }
            Operation::ImportFrameFromFile { name, path, x, y } => {
                self.prepare_import_frame_from_file(name, path, x, y)?
            }
            Operation::ImportFrameFromArtifact {
                name,
                artifact,
                connector,
                x,
                y,
            } => self.prepare_import_frame_from_artifact(name, artifact, connector, x, y)?,
            Operation::RefreshFrameArtifact { frame_id, artifact } => {
                self.prepare_refresh_frame_artifact(frame_id, artifact)?
            }
            Operation::SetFrameSource {
                frame_id,
                artifact,
                connector,
            } => self.prepare_set_frame_source(frame_id, artifact, connector)?,
            Operation::AddPlot {
                name,
                source_frame_id,
                spec,
                x,
                y,
                view_id,
            } => self.prepare_add_plot(name, source_frame_id, spec, x, y, view_id)?,
            Operation::RenameObject { object_id, name } => {
                self.prepare_rename_object(object_id, name)?
            }
            Operation::DeleteObject { object_id } => self.prepare_delete_object(object_id)?,
            Operation::SetValue { object_id, raw } => self.prepare_set_value(object_id, raw)?,
            Operation::SetPlotSpec { plot_id, spec } => {
                self.prepare_set_plot_spec(plot_id, spec)?
            }
            Operation::MoveView { view_id, x, y } => self.prepare_move_view(view_id, x, y)?,
            Operation::ResizeView {
                view_id,
                width,
                height,
            } => self.prepare_resize_view(view_id, width, height)?,
            Operation::SetViewCollapsed { view_id, collapsed } => {
                self.prepare_set_view_collapsed(view_id, collapsed)?
            }
            Operation::TidyLayout => self.prepare_tidy_layout()?,
            Operation::SetFrameDisplayOrientation {
                frame_id,
                orientation,
            } => self.prepare_set_frame_display_orientation(frame_id, orientation)?,
            Operation::SetFrameDisplayCrosstab { frame_id, crosstab } => {
                self.prepare_set_frame_display_crosstab(frame_id, crosstab)?
            }
            Operation::BranchFrame { view_id, frame_id } => {
                self.prepare_branch_frame(view_id, frame_id)?
            }
            Operation::MoveTab {
                source_view_id,
                target_view_id,
                object_id,
                target_index,
            } => self.prepare_move_tab(source_view_id, target_view_id, object_id, target_index)?,
            Operation::DetachTab {
                view_id,
                object_id,
                x,
                y,
            } => self.prepare_detach_tab(view_id, object_id, x, y)?,
            Operation::SetActiveTab { view_id, object_id } => {
                self.prepare_set_active_tab(view_id, object_id)?
            }
            Operation::SetFrameDisplayFilter {
                frame_id,
                filters,
                filter_match_all,
            } => self.prepare_set_frame_display_filter(frame_id, filters, filter_match_all)?,
            Operation::SetFrameDisplaySort { frame_id, keys } => {
                self.prepare_set_frame_display_sort(frame_id, keys)?
            }
            operation @ (Operation::SetFrameSummaryRows { .. }
            | Operation::SetFrameSummaryDrawer { .. }) => {
                self.prepare_frame_summary_operation(operation)?
            }
            Operation::SetFrameStyle {
                frame_id,
                target,
                style,
            } => self.prepare_set_frame_style(frame_id, target, style)?,
            Operation::SetFrameStyleRules { frame_id, rules } => {
                self.prepare_set_frame_style_rules(frame_id, rules)?
            }
            Operation::SetCell {
                frame_id,
                row_id,
                column_id,
                raw,
            } => self.prepare_set_cell(frame_id, row_id, column_id, raw)?,
            Operation::SetCells { frame_id, cells } => self.prepare_set_cells(frame_id, cells)?,
            Operation::AddRow { frame_id, values } => self.prepare_add_row(frame_id, values)?,
            Operation::SetFrameFromPastedText { frame_id, text } => {
                self.prepare_set_frame_from_pasted_text(frame_id, text)?
            }
            Operation::PasteCells {
                frame_id,
                row_id,
                column_id,
                grid,
            } => self.prepare_paste_cells(frame_id, row_id, column_id, grid)?,
            Operation::DeleteRow { frame_id, row_id } => {
                self.prepare_delete_row(frame_id, row_id)?
            }
            Operation::AddColumn {
                frame_id,
                name,
                data_type,
                after_column_id,
            } => self.prepare_add_column(frame_id, name, data_type, after_column_id)?,
            Operation::DeleteColumn {
                frame_id,
                column_id,
            } => self.prepare_delete_column(frame_id, column_id)?,
            Operation::RenameColumn {
                frame_id,
                column_id,
                name,
            } => self.prepare_rename_column(frame_id, column_id, name)?,
            Operation::SetColumnType {
                frame_id,
                column_id,
                data_type,
            } => self.prepare_set_column_type(frame_id, column_id, data_type)?,
            Operation::SetColumnCategories {
                frame_id,
                column_id,
                categories,
            } => self.prepare_set_column_categories(frame_id, column_id, categories)?,
            Operation::SetColumnFormat {
                frame_id,
                column_id,
                format,
            } => self.prepare_set_column_format(frame_id, column_id, format)?,
            Operation::AddComputedColumn {
                frame_id,
                name,
                formula,
                after_column_id,
            } => self.prepare_add_computed_column(frame_id, name, formula, after_column_id)?,
            Operation::SetColumnFormula {
                frame_id,
                column_id,
                formula,
            } => self.prepare_set_column_formula(frame_id, column_id, formula)?,
            Operation::SetCellOverride {
                frame_id,
                row_id,
                column_id,
                formula,
            } => self.prepare_set_cell_override(frame_id, row_id, column_id, formula)?,
            Operation::AddSummary {
                frame_id,
                column_id,
                operation,
            } => self.prepare_add_summary(frame_id, column_id, operation)?,
            Operation::AddDerivedFrame {
                source_frame_id,
                name,
                group_keys,
                aggregates,
                maintain_order,
                x,
                y,
            } => self.prepare_add_derived_frame(
                source_frame_id,
                name,
                group_keys,
                aggregates,
                maintain_order,
                x,
                y,
            )?,
            Operation::AddLinkedFrame {
                source_frame_id,
                name,
                x,
                y,
            } => self.prepare_add_linked_frame(source_frame_id, name, x, y)?,
            Operation::SetFrameMaterialization { frame_id, artifact } => {
                self.prepare_set_frame_materialization(frame_id, artifact)?
            }
            Operation::RenameDocument { name } => self.prepare_rename_document(name)?,
            Operation::ClearFrameMaterialization { frame_id } => {
                self.prepare_clear_frame_materialization(frame_id)?
            }
            // Nothing to resolve: the answer was computed and written before
            // this got here, and a value that has one is a value that has
            // one. Clearing is likewise always allowed — the formula is
            // still there to be worked out live.
            Operation::SetFrozenValue { object_id, frozen } => {
                if self.value_expression(&object_id).is_err() {
                    return Err(CoreError::ObjectNotFound);
                }
                ReplicatedOperation::SetFrozenValue { object_id, frozen }
            }
            // Nothing to resolve: markdown is not parsed. A blank comment is
            // no comment — normalizing here means "cleared by deleting the
            // text" and "cleared by the menu" replicate as the same edit.
            Operation::SetFrameComment { frame_id, comment } => {
                self.frame(&frame_id)?;
                ReplicatedOperation::SetFrameComment {
                    frame_id,
                    comment: comment.filter(|text| !text.trim().is_empty()),
                }
            }
            Operation::AdoptFrameRows { frame_id, artifact } => {
                self.prepare_adopt_frame_rows(frame_id, artifact)?
            }
            Operation::PackageDocument { adopted } => self.prepare_package_document(adopted)?,
            Operation::PromoteDisplayToSteps { frame_id } => {
                self.prepare_promote_display_to_steps(frame_id)?
            }
            Operation::SetUniqueKey {
                frame_id,
                column_ids,
                enabled,
            } => self.prepare_set_unique_key(frame_id, column_ids, enabled)?,
            Operation::AddJoinFrame {
                primary_frame_id,
                lookup_frame_id,
                primary_key_column_ids,
                lookup_key_column_ids,
                join_type,
                columns: output_inputs,
                name,
                x,
                y,
            } => self.prepare_add_join_frame(
                primary_frame_id,
                lookup_frame_id,
                primary_key_column_ids,
                lookup_key_column_ids,
                join_type,
                output_inputs,
                name,
                x,
                y,
            )?,
            Operation::SetFramePipeline { frame_id, steps } => {
                self.prepare_set_frame_pipeline(frame_id, steps)?
            }
        })
    }
}
