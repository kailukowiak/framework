//! Evaluation rules that belong to Scratchwork rather than to frames.
//!
//! A frame may only inline a value backed by live frame data after that value
//! has been recorded; otherwise a frame could reach back through the value and
//! into itself. Scratchwork has a different contract: its lines are live views
//! and already have a top-to-bottom, cycle-checked dependency graph. When one
//! line reads another line whose aggregate touches a live frame, evaluate that
//! upstream line first and carry its answer into the downstream calculation as
//! a temporary literal. Nothing is persisted or frozen, and frame compilation
//! keeps its stricter cycle boundary.

use crate::*;
use polars::prelude::Series;
use std::collections::HashSet;

impl Document {
    /// Evaluate one Scratchwork expression with live upstream lines.
    pub(crate) fn evaluate_scratchwork_series(
        &self,
        expression: &Expr,
    ) -> Result<(DataType, Series), String> {
        if !self.has_live_line_dependency(expression, &mut HashSet::new()) {
            return self.evaluate_to_series(expression);
        }

        let mut evaluation = self.clone();
        prepare_live_line_dependencies(
            &mut evaluation,
            expression,
            &mut HashSet::new(),
            &mut HashSet::new(),
        )?;
        evaluation.evaluate_to_series(expression)
    }

    fn has_live_line_dependency(&self, expression: &Expr, seen: &mut HashSet<Id>) -> bool {
        let mut found = false;
        expression.walk_values(&mut |object_id| {
            if found || !seen.insert(object_id.to_string()) {
                return;
            }
            if self.frozen_values.contains_key(object_id) {
                return;
            }
            let Some((block, index)) = self.block_line(object_id) else {
                return;
            };
            let Some(dependency) = block.lines[index].expression() else {
                return;
            };
            found = self.first_live_frame(dependency).is_some()
                || self.has_live_line_dependency(dependency, seen);
        });
        found
    }
}

fn prepare_live_line_dependencies(
    document: &mut Document,
    expression: &Expr,
    visiting: &mut HashSet<Id>,
    prepared: &mut HashSet<Id>,
) -> Result<(), String> {
    let mut object_ids = Vec::new();
    expression.walk_values(&mut |object_id| object_ids.push(object_id.to_string()));
    for object_id in object_ids {
        if prepared.contains(&object_id) {
            continue;
        }
        if document.frozen_values.contains_key(&object_id) {
            prepared.insert(object_id);
            continue;
        }
        let Some((block, index)) = document.block_line(&object_id) else {
            continue;
        };
        let Some(dependency) = block.lines[index].expression().cloned() else {
            continue;
        };
        if !visiting.insert(object_id.clone()) {
            return Err("That Scratchwork formula creates a circular dependency".into());
        }
        prepare_live_line_dependencies(document, &dependency, visiting, prepared)?;
        visiting.remove(&object_id);

        if document.first_live_frame(&dependency).is_some() {
            let (data_type, series) = document.evaluate_to_series(&dependency)?;
            let series_id = format!("__scratchwork_live_{object_id}");
            let values = (0..series.len())
                .map(|index| crate::polars_value_at(&series, index))
                .map(|value| value.map(crate::engine::values::scalar_value_to_raw))
                .collect::<Result<Vec<_>, _>>()?;
            document.objects.push(DataObject::Series(SeriesObject {
                id: series_id.clone(),
                name: series_id.clone(),
                data_type,
                values,
            }));
            let line = document
                .objects
                .iter_mut()
                .find_map(|object| match object {
                    DataObject::Block(block) => {
                        block.lines.iter_mut().find(|line| line.id == object_id)
                    }
                    _ => None,
                })
                .ok_or_else(|| "Scratchwork line not found".to_string())?;
            line.formula = Some(Formula {
                expression: Expr::Series {
                    object_id: series_id,
                },
            });
        }
        prepared.insert(object_id);
    }
    Ok(())
}
