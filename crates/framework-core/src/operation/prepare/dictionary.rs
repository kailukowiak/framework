//! Dictionary creation uses the ordinary frame, history, and key machinery.
use crate::*;
impl Document {
    pub(crate) fn prepare_add_dictionary(
        &self,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let name = self.unique_frame_name(&name, None);
        let (mut frame, view) = Self::build_frame_with_types(
            name,
            vec![
                vec!["Key".into(), "Value".into()],
                vec!["".into(), "".into()],
            ],
            vec![DataType::String, DataType::String],
            x,
            y,
        );
        frame.unique_keys.push(UniqueKeyConstraint {
            id: id(),
            column_ids: vec![frame.columns[0].id.clone()],
        });
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }
}

impl Document {
    pub(crate) fn prepare_table_creation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        Ok(match operation {
            Operation::AddDictionary { name, x, y } => self.prepare_add_dictionary(name, x, y)?,
            Operation::AddFrame { name, grid, x, y } => self.prepare_add_frame(name, grid, x, y)?,
            Operation::AddFrameFromPastedText { name, text, x, y } => {
                self.prepare_add_frame_from_pasted_text(name, text, x, y)?
            }
            Operation::AddGeneratorFrame {
                name,
                formula,
                column_name,
                x,
                y,
            } => self.prepare_add_generator_frame(name, formula, column_name, x, y)?,
            _ => {
                return Err(CoreError::InvalidOperation(
                    "Expected a table creation operation".into(),
                ));
            }
        })
    }
}
