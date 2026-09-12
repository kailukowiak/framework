//! Fit during preparation, replicate the resulting fitted state. Replaying an
//! edit must not train independently on whichever data a replica sees today.
use crate::engine::ml::{ml_error, training_fingerprint};
use crate::*;
use framework_ml::{FitRequest, NumericDataset};
use sha2::{Digest, Sha256};

impl Document {
    pub(crate) fn prepare_model_operation(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        match operation {
            Operation::AddModel { name, spec, x, y } => {
                self.validate_model_spec(&spec)?;
                Ok(add_model(
                    ModelObject {
                        id: id(),
                        name,
                        spec: Some(spec),
                        fitted: None,
                        imported_input: None,
                    },
                    x,
                    y,
                ))
            }
            Operation::SetModelSpec { model_id, spec } => {
                self.model(&model_id)?;
                self.validate_model_spec(&spec)?;
                Ok(ReplicatedOperation::SetModelSpec { model_id, spec })
            }
            Operation::FitModel { model_id } => self.prepare_model_fit(model_id),
            operation @ (Operation::ImportModel { .. } | Operation::ImportOnnxModel { .. }) => {
                self.prepare_model_import(operation)
            }
            Operation::AddModelPredictions {
                model_id,
                source_frame_id,
                feature_column_ids,
                name,
                x,
                y,
            } => self.prepare_model_predictions(
                model_id,
                source_frame_id,
                feature_column_ids,
                name,
                x,
                y,
            ),
            Operation::AddModelSummary {
                model_id,
                kind,
                name,
                x,
                y,
            } => self.prepare_model_summary(model_id, kind, name, x, y),
            _ => Err(ml_error("Not a model operation")),
        }
    }

    pub(crate) fn validate_model_spec(&self, spec: &ModelSpec) -> Result<(), CoreError> {
        self.validate_model_spec_shape(spec)?;
        self.model_columns(&spec.source_frame_id, &spec.feature_column_ids)?;
        self.model_columns(
            &spec.source_frame_id,
            std::slice::from_ref(spec.target_column_id.as_ref().expect("validated shape")),
        )?;
        Ok(())
    }

    /// History may restore an honest missing-column state. The recipe shape
    /// must remain valid, but only a new authored edit requires live inputs.
    pub(crate) fn validate_model_spec_shape(&self, spec: &ModelSpec) -> Result<(), CoreError> {
        if spec.feature_column_ids.is_empty() || spec.feature_column_ids.len() > 256 {
            return Err(ml_error("Choose between one and 256 input features"));
        }
        let unique: std::collections::HashSet<_> = spec.feature_column_ids.iter().collect();
        if unique.len() != spec.feature_column_ids.len() {
            return Err(ml_error("A model feature is selected twice"));
        }
        let target = spec
            .target_column_id
            .as_ref()
            .ok_or_else(|| ml_error("Choose a target column"))?;
        if spec.feature_column_ids.contains(target) {
            return Err(ml_error("The target cannot also be an input feature"));
        }
        if !spec.confidence_level.is_finite()
            || !(0.0..1.0).contains(&spec.confidence_level)
            || spec.confidence_level == 0.0
        {
            return Err(ml_error("Confidence level must be between zero and one"));
        }
        if !spec.holdout_fraction.is_finite() || !(0.0..0.5).contains(&spec.holdout_fraction) {
            return Err(ml_error(
                "Holdout fraction must be at least zero and less than one half",
            ));
        }
        Ok(())
    }

    fn prepare_model_fit(&self, model_id: Id) -> Result<ReplicatedOperation, CoreError> {
        let model = self.model(&model_id)?;
        let spec = model
            .spec
            .as_ref()
            .ok_or_else(|| ml_error("An imported model cannot be retrained here"))?;
        self.validate_model_spec(spec)?;
        let mut columns = spec.feature_column_ids.clone();
        let target = spec
            .target_column_id
            .clone()
            .ok_or_else(|| ml_error("Choose a target column"))?;
        columns.push(target.clone());
        let rows = self.model_data(&spec.source_frame_id, &columns)?;
        let fingerprint = training_fingerprint(&rows, &columns);
        let split = split_rows(rows.len(), spec.holdout_fraction, spec.seed)?;
        let dataset = |indices: &[u32]| NumericDataset {
            rows: indices
                .iter()
                .map(|&i| rows[i as usize][..columns.len() - 1].to_vec())
                .collect(),
            targets: indices
                .iter()
                .map(|&i| rows[i as usize][columns.len() - 1])
                .collect(),
        };
        let request = FitRequest {
            method: spec.method,
            feature_names: self.model_columns(&spec.source_frame_id, &spec.feature_column_ids)?,
            target_name: self
                .model_columns(&spec.source_frame_id, &[target])?
                .remove(0),
            confidence_level: spec.confidence_level,
            covariance: spec.covariance,
            max_iterations: 100,
            forest: spec.forest.clone(),
        };
        let result =
            framework_ml::fit(&request, &dataset(&split.training_rows)).map_err(ml_error)?;
        let evaluation_metrics = if split.evaluation_rows.is_empty() {
            None
        } else {
            Some(
                framework_ml::evaluate(&result, &dataset(&split.evaluation_rows))
                    .map_err(ml_error)?,
            )
        };
        let fitted = ModelFit {
            id: id(),
            training_revision: self.revision,
            training_fingerprint: Some(fingerprint),
            spec: Some(spec.clone()),
            result,
            evaluation_metrics,
            split: Some(split),
        };
        Ok(ReplicatedOperation::SetModelFit {
            model_id,
            fitted: Some(fitted),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_model_predictions(
        &self,
        model_id: Id,
        source_frame_id: Id,
        feature_column_ids: Vec<Id>,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let model = self.model(&model_id)?;
        let fitted = model
            .fitted
            .as_ref()
            .ok_or_else(|| ml_error("Fit the model before predicting"))?;
        self.model_binding_columns(&source_frame_id, &feature_column_ids, &fitted.result)?;
        if feature_column_ids.len() != fitted.result.feature_names.len() {
            return Err(ml_error(
                "Select one source feature for each model input, in fitted order",
            ));
        }
        let columns = framework_ml::output_names(&fitted.result)
            .into_iter()
            .zip(framework_ml::output_types(&fitted.result))
            .map(|(name, dtype)| Column {
                id: column_id(&name),
                name,
                source_name: None,
                data_type: crate::engine::model_inputs::model_output_type(dtype),
                categories: Vec::new(),
                format: None,
                formula: None,
            })
            .collect::<Vec<_>>();
        let object_id = id();
        let output_column_ids = columns.iter().map(|c| c.id.clone()).collect();
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(FrameObject {
                id: object_id.clone(),
                name: self.unique_frame_name(&name, None),
                base_columns: columns.clone(),
                columns,
                prediction: Some(ModelPrediction {
                    model_id,
                    feature_column_ids,
                    output_column_ids,
                }),
                derivation: Some(FrameDerivation {
                    source_frame_id,
                    join: None,
                    steps: Vec::new(),
                }),
                ..FrameObject::default()
            }),
            view: model_view(object_id, x, y, 480.0, 360.0),
            container_id: None,
        })
    }
}

pub(crate) fn add_model(model: ModelObject, x: f64, y: f64) -> ReplicatedOperation {
    let height = model.fitted.as_ref().map_or(140.0, |fit| {
        let rows = fit.result.summary.coefficients.len();
        (140.0
            + if rows == 0 {
                0.0
            } else {
                (rows + 1) as f64 * 22.0
            })
        .min(600.0)
    });
    let view = model_view(model.id.clone(), x, y, 560.0, height);
    ReplicatedOperation::AddObject {
        object: DataObject::Model(Box::new(model)),
        view,
        container_id: None,
    }
}

pub(crate) fn model_view(object_id: Id, x: f64, y: f64, width: f64, height: f64) -> CanvasView {
    CanvasView {
        id: id(),
        object_id,
        x,
        y,
        width,
        height,
        collapsed: false,
        tab_object_ids: Vec::new(),
    }
}

fn split_rows(count: usize, fraction: f64, seed: u32) -> Result<ModelSplit, CoreError> {
    let test_count = (count as f64 * fraction).floor() as usize;
    if fraction > 0.0 && test_count == 0 {
        return Err(ml_error(
            "There are too few rows for that holdout fraction; use more data or training-only fit",
        ));
    }
    let mut ranked: Vec<_> = (0..count as u32)
        .map(|row| {
            let mut hash = Sha256::new();
            hash.update(seed.to_le_bytes());
            hash.update(row.to_le_bytes());
            (hash.finalize(), row)
        })
        .collect();
    ranked.sort_by_key(|a| a.0);
    let mut evaluation_rows: Vec<_> = ranked[..test_count].iter().map(|(_, row)| *row).collect();
    let mut training_rows: Vec<_> = ranked[test_count..].iter().map(|(_, row)| *row).collect();
    evaluation_rows.sort();
    training_rows.sort();
    Ok(ModelSplit {
        seed,
        training_rows,
        evaluation_rows,
    })
}
