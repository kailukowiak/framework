//! File-backed inputs and explicit transitions to editable literal data.
//! Keep their dispatch together so the root operation router does not also
//! become the place where file ownership and materialization are implemented.
use crate::*;

impl Document {
    pub(super) fn prepare_file_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(match operation {
            Operation::RenameColumn {
                frame_id,
                column_id,
                name,
            } => self.prepare_rename_column(frame_id, column_id, name)?,
            Operation::RenameColumnsUsingMapping {
                frame_id,
                mapping_frame_id,
                key_column_id,
                value_column_id,
            } => self.prepare_mapping_rename(
                frame_id,
                mapping_frame_id,
                key_column_id,
                value_column_id,
            )?,
            Operation::RenameColumns { frame_id, names } => {
                let mut frame = self.frame(&frame_id)?.clone();
                let mut seen = std::collections::HashSet::new();
                for (column_id, name) in names {
                    if !seen.insert(column_id.clone()) || name.trim().is_empty() {
                        return Err(CoreError::InvalidOperation(
                            "Supply each column once with a nonempty name".into(),
                        ));
                    }
                    let column = frame
                        .columns
                        .iter_mut()
                        .find(|c| c.id == column_id)
                        .ok_or(CoreError::ColumnNotFound)?;
                    column.name = name.clone();
                    if let Some(base) = frame.base_columns.iter_mut().find(|c| c.id == column_id) {
                        base.name = name;
                    }
                }
                let mut names = std::collections::HashSet::new();
                if frame.columns.iter().any(|c| !names.insert(c.name.clone())) {
                    return Err(CoreError::InvalidOperation(
                        "Column names must be unique".into(),
                    ));
                }
                ReplicatedOperation::RestoreFrame { frame }
            }
            Operation::OpenDelimitedFile { name, path, x, y } => {
                self.prepare_open_delimited(name, path, x, y)?
            }
            Operation::BakeFrame { frame_id } => ReplicatedOperation::RestoreFrame {
                frame: self.bake_frame(&frame_id)?,
            },
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
            _ => {
                return Err(CoreError::InvalidOperation(
                    "Expected a file operation".into(),
                ));
            }
        })
    }
}
