//! Header filters author the same data chain as Wrangle.
use crate::*;

impl Document {
    pub(crate) fn prepare_set_frame_display_filter(
        &self,
        frame_id: Id,
        filters: Vec<String>,
        filter_match_all: bool,
    ) -> Result<ReplicatedOperation, CoreError> {
        let frame = self.frame(&frame_id)?;
        let mut parsed = Vec::new();
        for formula in filters {
            let expression = self.prepare_formula_for_frame(&frame.id, &formula)?;
            let data_type = frame
                .infer_polars_expression_type(self, &expression)
                .map_err(CoreError::Formula)?;
            if data_type != DataType::Boolean {
                return Err(CoreError::InvalidOperation(
                    "A display filter must produce true or false".into(),
                ));
            }
            parsed.push(Formula { expression });
        }
        let mut frame = frame.clone();
        let steps = if let Some(derivation) = &mut frame.derivation {
            &mut derivation.steps
        } else {
            &mut frame.steps
        };
        // The legacy command name remains a wire-compatible alias. A header
        // filter is now data, just like a header sort; keep earlier filters
        // in place because calculations may depend on their position.
        let insertion = steps.len().saturating_sub(usize::from(matches!(
            steps.last(),
            Some(FrameStep::Sort { .. })
        )));
        let index = if insertion > 0 && matches!(steps[insertion - 1], FrameStep::Filter { .. }) {
            steps.remove(insertion - 1);
            insertion - 1
        } else {
            insertion
        };
        if !parsed.is_empty() {
            steps.insert(
                index,
                FrameStep::Filter {
                    predicates: parsed
                        .into_iter()
                        .map(|formula| formula.expression)
                        .collect(),
                    match_all: filter_match_all,
                },
            );
        }
        frame.display.set_filter(Vec::new(), true);
        Ok(ReplicatedOperation::RestoreFrame { frame })
    }
}
