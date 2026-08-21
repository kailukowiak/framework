//! Keeping a linked frame's hidden identity projection aligned with its source.
//!
//! A link owns its output column ids, so it begins with a projection from the
//! source ids and a select that adopts the outputs. That pair is plumbing and
//! the inspector carries it invisibly whenever it edits the authored steps.
//! If the source later gains a calculated column, an old copy of the plumbing
//! must learn about it before completion or parsing walks the authored chain.

use crate::*;
use std::collections::HashSet;

impl Document {
    /// Add newly published source columns to an existing pass-through prefix.
    ///
    /// This runs while walking a draft, so completion sees the addition before
    /// anything is saved. The same walk prepares `SetFramePipeline`; its minted
    /// output id is then carried by the replicated operation and becomes the
    /// linked frame's durable identity for the new column.
    pub(crate) fn reconcile_pass_through_inputs(
        &self,
        frame: &FrameObject,
        input_columns: &[Column],
        mut steps: Vec<FrameStepInput>,
    ) -> Vec<FrameStepInput> {
        let Some(derivation) = &frame.derivation else {
            return steps;
        };
        if derivation.join.is_some() {
            return steps;
        }
        let [
            FrameStep::WithColumns {
                columns: stored_columns,
            },
            FrameStep::Select {
                column_ids: stored_ids,
            },
            ..,
        ] = derivation.steps.as_slice()
        else {
            return steps;
        };
        let pass_through = stored_columns
            .iter()
            .all(|column| matches!(column.expression, Expr::Column { .. }))
            && stored_ids.len() == stored_columns.len()
            && stored_ids
                .iter()
                .zip(stored_columns)
                .all(|(selected, column)| selected == &column.output_column_id);
        if !pass_through {
            return steps;
        }

        let [
            FrameStepInput::WithColumns { columns },
            FrameStepInput::Select { column_ids },
            ..,
        ] = steps.as_mut_slice()
        else {
            return steps;
        };
        let projected_sources: HashSet<&str> = stored_columns
            .iter()
            .filter_map(|column| match &column.expression {
                Expr::Column { column_id } => Some(column_id.as_str()),
                _ => None,
            })
            .collect();
        let mut used_names: HashSet<String> =
            columns.iter().map(|column| column.name.clone()).collect();

        for source in input_columns {
            if projected_sources.contains(source.id.as_str())
                || columns
                    .iter()
                    .any(|column| column.formula == formula_name(&source.name))
            {
                continue;
            }
            let name = unique_pass_through_name(&source.name, &mut used_names);
            let output_column_id = column_id(&name);
            columns.push(ExistingFormulaInput {
                output_column_id: output_column_id.clone(),
                name,
                formula: formula_name(&source.name),
            });
            column_ids.push(output_column_id);
        }
        steps
    }
}

fn unique_pass_through_name(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("the numeric suffix space is unbounded")
}
