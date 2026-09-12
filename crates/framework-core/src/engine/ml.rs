//! The document boundary for fitted models: stable column IDs become ordered
//! numerical inputs here, and prediction columns re-enter the ordinary frame
//! plan so downstream formulas read today's rows with the saved fit.
use crate::*;
use polars::prelude as pl;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const MAX_MODEL_ROWS: usize = 100_000;

pub(crate) fn ml_error(error: impl ToString) -> CoreError {
    CoreError::InvalidOperation(error.to_string())
}

impl Document {
    pub(crate) fn model_prediction_fingerprint(&self, frame: &FrameObject) -> Option<String> {
        let binding = frame.prediction.as_ref()?;
        let fitted = self
            .model(&binding.model_id)
            .ok()
            .and_then(|model| model.fitted.as_ref());
        serde_json::to_string(&(binding, fitted.map(|fit| &fit.id))).ok()
    }

    pub(crate) fn model_columns(
        &self,
        source_id: &str,
        feature_ids: &[Id],
    ) -> Result<Vec<String>, CoreError> {
        let frame = self.frame(source_id)?;
        if feature_ids.is_empty() || feature_ids.len() > 257 {
            return Err(ml_error("Choose between one and 257 numerical columns"));
        }
        let mut seen = HashSet::new();
        feature_ids
            .iter()
            .map(|id| {
                if !seen.insert(id) {
                    return Err(ml_error("A model feature is selected twice"));
                }
                let column = frame
                    .columns
                    .iter()
                    .find(|column| &column.id == id)
                    .ok_or(CoreError::ColumnNotFound)?;
                if !matches!(
                    column.data_type,
                    DataType::Integer
                        | DataType::Number
                        | DataType::Currency
                        | DataType::Percentage
                        | DataType::Boolean
                ) {
                    return Err(ml_error(format!(
                        "‘{}’ must be numerical; encode categories in Wrangle first",
                        column.name
                    )));
                }
                Ok(column.name.clone())
            })
            .collect()
    }

    pub(crate) fn model_data(
        &self,
        source_id: &str,
        ids: &[Id],
    ) -> Result<Vec<Vec<f64>>, CoreError> {
        self.model_columns(source_id, ids)?;
        let frame = self
            .materialize_frame_lazy(source_id, Layer::Data, &mut HashSet::new())
            .map_err(ml_error)?
            .select(
                ids.iter()
                    .map(|id| pl::col(id.as_str()).cast(pl::DataType::Float64))
                    .collect::<Vec<_>>(),
            )
            .limit((MAX_MODEL_ROWS + 1) as u32)
            .collect()
            .map_err(ml_error)?;
        if frame.height() > MAX_MODEL_ROWS {
            return Err(ml_error("Models currently accept at most 100,000 rows"));
        }
        numeric_rows(&frame, ids).map_err(ml_error)
    }

    pub(crate) fn model_fingerprint(&self, spec: &ModelSpec) -> Result<String, CoreError> {
        let mut ids = spec.feature_column_ids.clone();
        ids.push(
            spec.target_column_id
                .clone()
                .ok_or_else(|| ml_error("Choose a target column"))?,
        );
        let rows = self.model_data(&spec.source_frame_id, &ids)?;
        Ok(training_fingerprint(&rows, &ids))
    }

    pub(crate) fn compute_models(&self) -> HashMap<Id, ComputedModel> {
        self.objects
            .iter()
            .filter_map(|object| {
                let DataObject::Model(model) = object else {
                    return None;
                };
                let status = match (&model.spec, &model.fitted) {
                    (Some(spec), Some(fit)) => match self.model_fingerprint(spec) {
                        Ok(fingerprint) => ComputedModel {
                            stale: fit.spec.as_ref() != Some(spec)
                                || fit.training_fingerprint.as_ref() != Some(&fingerprint),
                            error: None,
                        },
                        Err(error) => ComputedModel {
                            stale: true,
                            error: Some(error.to_string()),
                        },
                    },
                    _ => ComputedModel {
                        stale: false,
                        error: None,
                    },
                };
                Some((model.id.clone(), status))
            })
            .collect()
    }

    pub(crate) fn model_prediction_plan(
        &self,
        frame: &FrameObject,
        source: pl::LazyFrame,
    ) -> Result<pl::LazyFrame, String> {
        use crate::engine::model_inputs::{typed_model_rows, typed_prediction_columns};
        use pl::IntoLazy;
        let binding = frame
            .prediction
            .as_ref()
            .ok_or("Missing model prediction binding")?;
        let model = self.model(&binding.model_id).map_err(|e| e.to_string())?;
        let fitted = model
            .fitted
            .as_ref()
            .ok_or("Fit the model before predicting")?;
        let rows = typed_model_rows(source, &binding.feature_column_ids, &fitted.result)?;
        let predictions =
            framework_ml::predict_scalars(&fitted.result, &rows).map_err(|e| e.to_string())?;
        let columns =
            typed_prediction_columns(&predictions, &binding.output_column_ids, &fitted.result)?;
        Ok(pl::DataFrame::new(rows.len(), columns)
            .map_err(|e| e.to_string())?
            .lazy())
    }
}

fn numeric_rows(frame: &pl::DataFrame, ids: &[Id]) -> Result<Vec<Vec<f64>>, String> {
    let columns = ids
        .iter()
        .map(|id| frame.column(id).and_then(|column| column.f64()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok((0..frame.height())
        .map(|row| {
            columns
                .iter()
                .map(|column| column.get(row).unwrap_or(f64::NAN))
                .collect()
        })
        .collect())
}

pub(crate) fn training_fingerprint(rows: &[Vec<f64>], ids: &[Id]) -> String {
    let mut hash = Sha256::new();
    for id in ids {
        hash.update((id.len() as u64).to_le_bytes());
        hash.update(id);
    }
    hash.update((rows.len() as u64).to_le_bytes());
    for row in rows {
        for value in row {
            hash.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}
