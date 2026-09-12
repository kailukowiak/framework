use crate::engine::ml::ml_error;
use crate::*;

impl Document {
    pub(crate) fn prepare_model_summary(
        &self,
        model_id: Id,
        kind: ModelSummaryKind,
        name: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let model = self.model(&model_id)?;
        let fit = model
            .fitted
            .as_ref()
            .ok_or_else(|| ml_error("Fit the model before copying its summary"))?;
        let (grid, types) =
            match kind {
                ModelSummaryKind::Coefficients => {
                    let mut grid = vec![
                        vec![
                            "Term",
                            "Estimate",
                            "Standard error",
                            "Statistic",
                            "P value",
                            "Lower",
                            "Upper",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    ];
                    for coefficient in &fit.result.summary.coefficients {
                        grid.push(vec![
                            coefficient.term.clone(),
                            raw(coefficient.estimate),
                            raw(coefficient.standard_error),
                            raw(coefficient.statistic),
                            raw(coefficient.p_value),
                            raw(coefficient.confidence_lower),
                            raw(coefficient.confidence_upper),
                        ]);
                    }
                    if grid.len() == 1 {
                        return Err(ml_error("This model does not have a coefficient table"));
                    }
                    (
                        grid,
                        [vec![DataType::String], vec![DataType::Number; 6]].concat(),
                    )
                }
                ModelSummaryKind::TrainingMetrics | ModelSummaryKind::EvaluationMetrics => {
                    let metrics = if matches!(kind, ModelSummaryKind::TrainingMetrics) {
                        &fit.result.summary.training_metrics
                    } else {
                        fit.evaluation_metrics
                            .as_ref()
                            .ok_or_else(|| ml_error("This fit has no held-out evaluation"))?
                    };
                    let values = [
                        ("Rows", Some(metrics.rows as f64)),
                        ("RMSE", metrics.rmse),
                        ("MAE", metrics.mae),
                        ("R squared", metrics.r_squared),
                        ("Accuracy", metrics.accuracy),
                        ("Log loss", metrics.log_loss),
                        ("Baseline RMSE", metrics.baseline_rmse),
                        ("Baseline accuracy", metrics.baseline_accuracy),
                        ("Baseline log loss", metrics.baseline_log_loss),
                    ];
                    let mut grid = vec![vec!["Metric".into(), "Value".into()]];
                    grid.extend(values.into_iter().filter_map(|(name, value)| {
                        value.map(|v| vec![name.into(), v.to_string()])
                    }));
                    (grid, vec![DataType::String, DataType::Number])
                }
            };
        let (mut frame, view) =
            Self::build_frame_with_types(self.unique_frame_name(&name, None), grid, types, x, y);
        frame.comment = Some(format!(
            "Editable summary snapshot of {} (fitted revision {}). This table does not retrain or change when its source model changes.",
            model.name, fit.id
        ));
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }
}

fn raw(value: Option<f64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}
