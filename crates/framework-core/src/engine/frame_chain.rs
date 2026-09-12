//! Render active and retained recipes independently of evaluating their data.
use crate::*;

impl FrameObject {
    pub(crate) fn render_active_chain(
        &self,
        document: &Document,
    ) -> (Vec<RenderedFrameStep>, usize) {
        match &self.derivation {
            Some(derivation) => {
                let chain = derivation.steps();
                if self.prediction.is_some() {
                    (self.render_steps(document, &self.base_columns, &chain), 0)
                } else if derivation.join.is_some() {
                    // The join is configured in its own compact summary and
                    // is the fixed input to Wrangle. Only the steps after it
                    // are editable there, starting from the join's retained
                    // output schema.
                    let editable = chain
                        .strip_prefix(&[FrameStep::Join {
                            join: derivation.join.clone().expect("checked above"),
                        }])
                        .unwrap_or(&chain);
                    let input = if self.base_columns.is_empty() {
                        &self.columns
                    } else {
                        &self.base_columns
                    };
                    (
                        self.render_steps(document, input, editable),
                        pass_through_prefix(editable),
                    )
                } else {
                    let rendered = document
                        .frame(&derivation.source_frame_id)
                        .map(|source| self.render_steps(document, &source.columns, &chain))
                        .unwrap_or_default();
                    (rendered, pass_through_prefix(&chain))
                }
            }
            // A source frame's chain is only ever what someone wrote in it.
            None => (
                self.render_steps(document, self.input_columns(), &self.steps),
                0,
            ),
        }
    }
}

/// How many leading steps exist only so a derived frame owns its column
/// ids: a projection of bare column references, and the select that adopts
/// them as the frame's schema.
///
/// This is the shape `AddLinkedFrame` and `BranchFrame` produce, and the
/// shape `FrameDerivation::steps` synthesizes from the legacy `projections`
/// field. It carries no transformation — every output is one input column,
/// unchanged — so presenting it as something the user wrote is noise. It is
/// still load-bearing: it is what records this frame's dependency on each
/// source column, which is how deleting one out from under it is refused.
fn pass_through_prefix(steps: &[FrameStep]) -> usize {
    let [
        FrameStep::WithColumns { columns },
        FrameStep::Select { column_ids },
        ..,
    ] = steps
    else {
        return 0;
    };
    let renames_only = columns
        .iter()
        .all(|column| matches!(column.expression, Expr::Column { .. }));
    let adopts_them = column_ids.len() == columns.len()
        && column_ids
            .iter()
            .zip(columns)
            .all(|(selected, column)| *selected == column.output_column_id);
    if renames_only && adopts_them { 2 } else { 0 }
}
