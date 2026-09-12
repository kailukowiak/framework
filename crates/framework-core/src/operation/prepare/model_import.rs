//! Imports carry data, never executable Python. Validation is performed before
//! the resolved object is published, and fitted preprocessing stays with it.
use super::ml::add_model;
use crate::engine::ml::ml_error;
use crate::*;

impl Document {
    pub(crate) fn prepare_model_import(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        let (name, source_frame_id, feature_column_ids, result, x, y) = match operation {
            Operation::ImportModel {
                name,
                source_frame_id,
                feature_column_ids,
                json,
                iteration_range,
                x,
                y,
            } => {
                let result =
                    framework_ml::import_xgboost_with_range(json.as_bytes(), iteration_range)
                        .map_err(ml_error)?;
                (name, source_frame_id, feature_column_ids, result, x, y)
            }
            Operation::ImportOnnxModel {
                name,
                source_frame_id,
                feature_column_ids,
                bytes,
                x,
                y,
            } => {
                let result = framework_ml::import_onnx(&bytes).map_err(ml_error)?;
                (name, source_frame_id, feature_column_ids, result, x, y)
            }
            _ => return Err(ml_error("Not a model import operation")),
        };
        self.model_binding_columns(&source_frame_id, &feature_column_ids, &result)?;
        let fitted = ModelFit {
            id: id(),
            training_revision: self.revision,
            training_fingerprint: None,
            spec: None,
            result,
            evaluation_metrics: None,
            split: None,
        };
        Ok(add_model(
            ModelObject {
                id: id(),
                name,
                spec: None,
                fitted: Some(fitted),
                imported_input: Some(ModelInput {
                    source_frame_id,
                    feature_column_ids,
                }),
            },
            x,
            y,
        ))
    }
}
