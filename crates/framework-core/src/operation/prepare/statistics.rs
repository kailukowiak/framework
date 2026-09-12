//! Statistical reports are explicit snapshots. Their values live in an ordinary
//! frame so formulas can reference them without a second summary-only engine.
use crate::engine::ml::ml_error;
use crate::*;

impl Document {
    pub(crate) fn prepare_statistical_analysis(
        &self,
        operation: Operation,
    ) -> Result<ReplicatedOperation, CoreError> {
        let Operation::AddStatisticalAnalysis {
            name,
            source_frame_id,
            x_column_id,
            y_column_id,
            request,
            x,
            y,
        } = operation
        else {
            return Err(ml_error("Not a statistical analysis operation"));
        };
        let mut ids = vec![x_column_id];
        if let Some(y) = y_column_id {
            ids.push(y);
        }
        let names = self.model_columns(&source_frame_id, &ids)?;
        let data = self.model_data(&source_frame_id, &ids)?;
        let first: Vec<_> = data.iter().map(|row| row[0]).collect();
        let second: Vec<_> = if ids.len() == 2 {
            data.iter().map(|row| row[1]).collect()
        } else {
            Vec::new()
        };
        let result = framework_ml::statistics(
            &request,
            &first,
            (ids.len() == 2).then_some(second.as_slice()),
        )
        .map_err(ml_error)?;
        let values = [
            ("Estimate", result.estimate),
            ("Standard error", result.standard_error),
            ("Statistic", result.statistic),
            ("Degrees of freedom", result.degrees_of_freedom),
            ("P value", result.p_value),
            ("Confidence lower", result.confidence_lower),
            ("Confidence upper", result.confidence_upper),
            ("N x", Some(result.n_x as f64)),
            ("N y", result.n_y.map(|n| n as f64)),
        ];
        let mut grid = vec![vec!["Statistic".into(), "Value".into()]];
        grid.extend(
            values
                .into_iter()
                .filter_map(|(name, value)| value.map(|v| vec![name.into(), v.to_string()])),
        );
        let (mut frame, view) = Self::build_frame_with_types(
            self.unique_frame_name(&name, None),
            grid,
            vec![DataType::String, DataType::Number],
            x,
            y,
        );
        frame.comment = Some(format!(
            "Editable {} snapshot of {}: {} at document revision {}, confidence {}%. {}",
            match result.method {
                framework_ml::StatsMethod::MeanConfidence => "Mean confidence interval",
                framework_ml::StatsMethod::PearsonCorrelation => "Pearson correlation",
                framework_ml::StatsMethod::WelchDifference => "Welch difference of means",
            },
            self.frame(&source_frame_id)?.name,
            names.join(", "),
            self.revision,
            request.confidence_level * 100.0,
            result.warnings.join(" ")
        ));
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }
}
