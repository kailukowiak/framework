use crate::engine::ml::ml_error;
use crate::*;

impl Document {
    pub(crate) fn apply_model_spec(
        &mut self,
        model_id: Id,
        spec: ModelSpec,
    ) -> Result<(), CoreError> {
        self.validate_model_spec_shape(&spec)?;
        let DataObject::Model(model) = self.object_mut(&model_id)? else {
            return Err(ml_error("That is not a model"));
        };
        if model.spec.is_none() {
            return Err(ml_error(
                "An imported model has no editable training specification",
            ));
        }
        model.spec = Some(spec);
        Ok(())
    }
    pub(crate) fn apply_model_fit(
        &mut self,
        model_id: Id,
        fitted: Option<ModelFit>,
    ) -> Result<(), CoreError> {
        if let Some(fit) = &fitted {
            framework_ml::validate_fitted(&fit.result).map_err(ml_error)?;
        }
        let DataObject::Model(model) = self.object_mut(&model_id)? else {
            return Err(ml_error("That is not a model"));
        };
        if let (Some(before), Some(after)) = (&model.fitted, &fitted)
            && before.id == after.id
            && before != after
        {
            return Err(ml_error(
                "A fitted revision is immutable; use a new fitted revision ID",
            ));
        }
        model.fitted = fitted;
        Ok(())
    }
    pub(crate) fn validate_model_object(&self, object: &DataObject) -> Result<(), CoreError> {
        match object {
            DataObject::Model(model) => {
                if let Some(spec) = &model.spec {
                    self.validate_model_spec_shape(spec)?;
                }
                if let Some(fitted) = &model.fitted {
                    framework_ml::validate_fitted(&fitted.result).map_err(ml_error)?;
                }
            }
            DataObject::Frame(frame) if frame.prediction.is_some() => {
                let binding = frame.prediction.as_ref().expect("guarded");
                let source = frame
                    .derivation
                    .as_ref()
                    .ok_or_else(|| ml_error("A prediction frame needs a source"))?;
                if source.source_frame_id == frame.id {
                    return Err(ml_error("A prediction frame cannot read itself"));
                }
                let inputs: std::collections::HashSet<_> =
                    binding.feature_column_ids.iter().collect();
                let outputs: std::collections::HashSet<_> =
                    binding.output_column_ids.iter().collect();
                if inputs.len() != binding.feature_column_ids.len()
                    || outputs.len() != binding.output_column_ids.len()
                {
                    return Err(ml_error("Prediction bindings cannot repeat column IDs"));
                }
                let model = self.model(&binding.model_id)?;
                let fit = model
                    .fitted
                    .as_ref()
                    .ok_or_else(|| ml_error("Fit the model before predicting"))?;
                if binding.feature_column_ids.len() != fit.result.feature_names.len()
                    || binding.output_column_ids.len()
                        != framework_ml::output_names(&fit.result).len()
                {
                    return Err(ml_error(
                        "Prediction binding does not match the fitted model",
                    ));
                }
                if !frame.rows.is_empty()
                    || frame.source_file.is_some()
                    || frame.artifact.is_some()
                    || frame.generator.is_some()
                {
                    return Err(ml_error(
                        "A prediction frame cannot also own or import rows",
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
