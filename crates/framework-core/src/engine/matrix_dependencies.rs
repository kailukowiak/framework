//! A matrix cell may name a live result rather than copy its expression.
//! Resolve live scalar dependencies in the private evaluation document before
//! handing the body to the frame compiler. Frozen answers deliberately remain
//! frozen; sensitivity is not permission to refresh a recorded fact.
use crate::*;
use std::collections::HashSet;

pub(super) fn prepare(
    document: &mut Document,
    expression: &Expr,
    visiting: &mut HashSet<Id>,
) -> Result<(), String> {
    let mut ids = Vec::new();
    expression.walk_values(&mut |id| ids.push(id.to_string()));
    for id in ids {
        if document.frozen_values.contains_key(&id) {
            continue;
        }
        let dependency = match document.object(&id) {
            Ok(DataObject::Result(result)) => Some(result.formula.expression.clone()),
            _ => document
                .block_line(&id)
                .and_then(|(block, index)| block.lines[index].expression().cloned()),
        };
        let Some(dependency) = dependency else {
            continue;
        };
        if !visiting.insert(id.clone()) {
            return Err("Circular sensitivity dependency".into());
        }
        prepare(document, &dependency, visiting)?;
        visiting.remove(&id);
        if document.first_live_frame(&dependency).is_none() {
            continue;
        }
        let (data_type, series) = document.evaluate_scratchwork_series(&dependency)?;
        let items = (0..series.len())
            .map(|i| expression_value_at(&series, data_type, i))
            .collect::<Result<Vec<_>, _>>()?;
        let expression = if items.len() == 1 {
            items[0].clone()
        } else {
            Expr::List { items }
        };
        if let Ok(DataObject::Result(result)) = document.object_mut(&id) {
            result.formula = Formula { expression };
        } else {
            for object in &mut document.objects {
                if let DataObject::Block(block) = object
                    && let Some(line) = block.lines.iter_mut().find(|line| line.id == id)
                {
                    line.formula = Some(Formula {
                        expression: expression.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}
