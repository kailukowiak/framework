//! Dependency tracing for canvas values and results: "what does this number
//! depend on, and what is that worth right now."
//!
//! Frame-level "how did I get here" is answered separately, by
//! [`Store::sample_frame_step`] walking a wrangle chain one step at a time.
//! This is the other half — the reference graph between values, results,
//! and the frames they read — so a bad number can be walked back to its
//! inputs one hop at a time. A frame is a leaf in this graph on purpose: its
//! own steps are a different kind of path, with its own way of being
//! bisected, not something this walk reaches into.
//!
//! [`Store::sample_frame_step`]: crate::Store::sample_frame_step

use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use ts_rs::TS;

/// One stop on a dependency walk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DependencyNode {
    pub object_id: Id,
    pub name: String,
    pub kind: DependencyKind,
    /// The formula as written, for a result. `None` for anything else.
    pub formula: Option<String>,
    /// The current value, rendered the way the canvas shows it.
    pub display: Option<String>,
    pub error: Option<String>,
    pub children: Vec<DependencyNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DependencyKind {
    Value,
    Result,
    Series,
    Frame,
    /// Reached again on the branch currently being walked.
    ///
    /// Formulas are not supposed to be able to form a cycle, and nothing
    /// here can fully stand in for that being enforced on write — this
    /// walk's own display values come from [`Document::compute_results`],
    /// which assumes the same thing and is not guarded against a cycle
    /// either. This variant only covers a case this walk could otherwise
    /// still recurse forever on despite that: a shared reference reached a
    /// second time via a different path is not this — see the branch that
    /// removes from `visiting` again once a subtree finishes.
    Repeated,
    Other,
}

impl Document {
    /// Walks what `object_id` depends on, recursively, with each stop's
    /// current value attached.
    pub(crate) fn dependency_graph(&self, object_id: &str) -> Result<DependencyNode, CoreError> {
        let computed = self.compute_results();
        let mut visiting = HashSet::new();
        self.dependency_node(object_id, &computed, &mut visiting)
    }

    fn dependency_node(
        &self,
        object_id: &str,
        computed: &HashMap<Id, ComputedResult>,
        visiting: &mut HashSet<Id>,
    ) -> Result<DependencyNode, CoreError> {
        let object = self.object(object_id)?;

        // Only guards the branch currently on the stack — removed again
        // below, so the same object reached from two different branches
        // (a diamond, not a cycle) still shows in full both times.
        if !visiting.insert(object_id.to_string()) {
            return Ok(DependencyNode {
                object_id: object_id.to_string(),
                name: object.name().to_string(),
                kind: DependencyKind::Repeated,
                formula: None,
                display: None,
                error: None,
                children: Vec::new(),
            });
        }

        let node = match object {
            DataObject::Value(value) => DependencyNode {
                object_id: value.id.clone(),
                name: value.name.clone(),
                kind: DependencyKind::Value,
                formula: None,
                display: Some(value.raw.clone()),
                error: None,
                children: Vec::new(),
            },
            DataObject::Series(series) => DependencyNode {
                object_id: series.id.clone(),
                name: series.name.clone(),
                kind: DependencyKind::Series,
                formula: None,
                display: Some(format!(
                    "{} value{}",
                    series.values.len(),
                    if series.values.len() == 1 { "" } else { "s" }
                )),
                error: None,
                children: Vec::new(),
            },
            DataObject::Result(result) => {
                let mut value_ids = Vec::new();
                let mut series_ids = Vec::new();
                let mut frame_ids = Vec::new();
                collect_references(
                    &result.formula.expression,
                    &mut value_ids,
                    &mut series_ids,
                    &mut frame_ids,
                );

                let mut children = Vec::new();
                for id in value_ids.into_iter().chain(series_ids) {
                    children.push(self.dependency_node(&id, computed, visiting)?);
                }
                for frame_id in frame_ids {
                    if let Ok(frame) = self.object(&frame_id) {
                        children.push(DependencyNode {
                            object_id: frame_id,
                            name: frame.name().to_string(),
                            kind: DependencyKind::Frame,
                            formula: None,
                            display: None,
                            error: None,
                            children: Vec::new(),
                        });
                    }
                }

                let outcome = computed.get(&result.id);
                DependencyNode {
                    object_id: result.id.clone(),
                    name: result.name.clone(),
                    kind: DependencyKind::Result,
                    formula: Some(self.render_formula_scalar(&result.formula.expression)),
                    display: outcome.map(|value| value.cell.display.clone()),
                    error: outcome.and_then(|value| value.cell.error.clone()),
                    children,
                }
            }
            DataObject::Frame(frame) => DependencyNode {
                object_id: frame.id.clone(),
                name: frame.name.clone(),
                kind: DependencyKind::Frame,
                formula: None,
                display: None,
                error: None,
                children: Vec::new(),
            },
            DataObject::Text(text) => {
                let mut value_ids = Vec::new();
                let mut series_ids = Vec::new();
                let mut frame_ids = Vec::new();
                let mut formulas = Vec::new();
                for segment in text.effective_segments() {
                    if let TextSegment::Formula { formula } = segment {
                        formulas.push(self.render_formula_scalar(&formula.expression));
                        collect_references(
                            &formula.expression,
                            &mut value_ids,
                            &mut series_ids,
                            &mut frame_ids,
                        );
                    }
                }

                // One prose formula can name the same frame through several
                // columns. A diagnostic tree should show the dependency once,
                // not repeat the entire query plan for every mention.
                let mut seen = HashSet::new();
                let mut children = Vec::new();
                for id in value_ids.into_iter().chain(series_ids) {
                    if seen.insert(id.clone()) {
                        children.push(self.dependency_node(&id, computed, visiting)?);
                    }
                }
                for frame_id in frame_ids {
                    if !seen.insert(frame_id.clone()) {
                        continue;
                    }
                    if let Ok(frame) = self.object(&frame_id) {
                        children.push(DependencyNode {
                            object_id: frame_id,
                            name: frame.name().to_string(),
                            kind: DependencyKind::Frame,
                            formula: None,
                            display: None,
                            error: None,
                            children: Vec::new(),
                        });
                    }
                }

                DependencyNode {
                    object_id: text.id.clone(),
                    name: text.name.clone(),
                    kind: DependencyKind::Other,
                    formula: (formulas.len() == 1).then(|| formulas.remove(0)),
                    display: None,
                    error: None,
                    children,
                }
            }
            other => DependencyNode {
                object_id: object_id.to_string(),
                name: other.name().to_string(),
                kind: DependencyKind::Other,
                formula: None,
                display: None,
                error: None,
                children: Vec::new(),
            },
        };

        visiting.remove(object_id);
        Ok(node)
    }
}

impl Store {
    /// See [`Document::dependency_graph`].
    pub fn dependency_graph(&self, object_id: &str) -> Result<DependencyNode, CoreError> {
        self.document().dependency_graph(object_id)
    }
}

/// Every value, series, and foreign frame an expression names, in one pass.
///
/// `Expr`'s variant fields carry the same visibility as the enum itself —
/// Rust has no way to narrow a single variant's fields below that — so this
/// can match them directly rather than needing a walker exposed from
/// `formula::ast`.
fn collect_references(
    expression: &Expr,
    values: &mut Vec<Id>,
    series: &mut Vec<Id>,
    frames: &mut Vec<Id>,
) {
    match expression {
        Expr::Value { object_id } => values.push(object_id.clone()),
        Expr::Series { object_id } => series.push(object_id.clone()),
        Expr::ForeignColumn { frame_id, .. } => frames.push(frame_id.clone()),
        Expr::List { items } => {
            for item in items {
                collect_references(item, values, series, frames);
            }
        }
        Expr::Negate { expression } | Expr::Not { expression } => {
            collect_references(expression, values, series, frames);
        }
        Expr::Binary { left, right, .. } => {
            collect_references(left, values, series, frames);
            collect_references(right, values, series, frames);
        }
        Expr::PolarsCall {
            arguments,
            keyword_arguments,
            ..
        } => {
            for argument in arguments {
                collect_references(argument, values, series, frames);
            }
            for (_, argument) in keyword_arguments {
                collect_references(argument, values, series, frames);
            }
        }
        Expr::Method {
            input,
            arguments,
            keyword_arguments,
            ..
        } => {
            collect_references(input, values, series, frames);
            for argument in arguments {
                collect_references(argument, values, series, frames);
            }
            for (_, argument) in keyword_arguments {
                collect_references(argument, values, series, frames);
            }
        }
        Expr::Integer { .. }
        | Expr::Number { .. }
        | Expr::Percentage { .. }
        | Expr::Money { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Date { .. }
        | Expr::Duration { .. }
        | Expr::Null
        | Expr::Column { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_store() -> Store {
        Store::new(Document {
            id: "test".into(),
            name: "Test".into(),
            revision: 0,
            objects: Vec::new(),
            views: Vec::new(),
            frozen_values: Default::default(),
        })
    }

    /// A value, and somewhere for it to live.
    ///
    /// A value has no home on the bare canvas — a loose constant belongs on
    /// a line of a formula block — so these tests put theirs in a container,
    /// which is the other place one may sit.
    fn a_container(store: &mut Store) -> Id {
        if let Some(existing) = store
            .document()
            .objects
            .iter()
            .find(|object| object.name() == "Holder")
        {
            return existing.id().to_string();
        }
        store
            .apply(Operation::AddContainer {
                name: "Holder".into(),
                x: 0.0,
                y: 0.0,
                container_id: None,
            })
            .unwrap();
        object_id_named(store, "Holder")
    }

    fn add_value(store: &mut Store, name: &str, raw: &str) {
        let holder = a_container(store);
        store
            .apply(Operation::AddValue {
                name: name.into(),
                raw: raw.into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder),
            })
            .unwrap();
    }

    fn object_id_named(store: &Store, name: &str) -> Id {
        store
            .document()
            .objects
            .iter()
            .find(|object| object.name() == name)
            .unwrap()
            .id()
            .to_string()
    }

    #[test]
    fn walks_a_result_back_to_the_values_it_reads() {
        let mut store = blank_store();
        let holder = a_container(&mut store);
        add_value(&mut store, "Down payment", "50000");
        add_value(&mut store, "Purchase price", "200000");
        store
            .apply(Operation::AddResult {
                name: "Down payment percentage".into(),
                formula: "`Down payment` / `Purchase price`".into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder.clone()),
            })
            .unwrap();

        let result_id = object_id_named(&store, "Down payment percentage");
        let graph = store.dependency_graph(&result_id).unwrap();

        assert_eq!(graph.kind, DependencyKind::Result);
        assert_eq!(graph.display.as_deref(), Some("0.25"));
        let names: HashSet<_> = graph
            .children
            .iter()
            .map(|child| child.name.as_str())
            .collect();
        assert_eq!(names, HashSet::from(["Down payment", "Purchase price"]));
    }

    /// A diamond — two results that both read the same value — is not a
    /// cycle. The value should show in full down both branches rather than
    /// being flattened to "already shown" the second time.
    #[test]
    fn the_same_value_reached_by_two_branches_shows_in_both() {
        let mut store = blank_store();
        let holder = a_container(&mut store);
        add_value(&mut store, "Base rate", "10");
        store
            .apply(Operation::AddResult {
                name: "Doubled".into(),
                formula: "`Base rate` * 2".into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder.clone()),
            })
            .unwrap();
        store
            .apply(Operation::AddResult {
                name: "Tripled".into(),
                formula: "`Base rate` * 3".into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder.clone()),
            })
            .unwrap();
        let base_rate_id = object_id_named(&store, "Base rate");
        store
            .apply(Operation::AddResult {
                name: "Sum".into(),
                formula: format!(
                    "`Doubled` + `Tripled` + `{}`",
                    store
                        .document()
                        .objects
                        .iter()
                        .find(|object| object.id() == base_rate_id)
                        .unwrap()
                        .name()
                ),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder.clone()),
            })
            .unwrap();

        let sum_id = object_id_named(&store, "Sum");
        let graph = store.dependency_graph(&sum_id).unwrap();

        assert_eq!(graph.children.len(), 3);
        let base_rate_children: Vec<_> = graph
            .children
            .iter()
            .filter(|child| child.name == "Base rate")
            .collect();
        assert_eq!(base_rate_children.len(), 1);
        assert_eq!(base_rate_children[0].kind, DependencyKind::Value);

        let doubled = graph
            .children
            .iter()
            .find(|child| child.name == "Doubled")
            .unwrap();
        assert_eq!(doubled.children[0].name, "Base rate");
        assert_eq!(doubled.children[0].kind, DependencyKind::Value);
    }

    #[test]
    fn a_text_card_trace_reaches_the_values_and_frames_behind_its_holes() {
        let mut store = blank_store();
        add_value(&mut store, "Tax rate", "0.05");
        store
            .apply(Operation::AddFrame {
                name: "Sales".into(),
                grid: vec![vec!["Revenue".into()], vec!["10".into()]],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
        let text_id = store
            .document()
            .objects
            .iter()
            .find(|object| matches!(object, DataObject::Text(_)))
            .unwrap()
            .id()
            .to_string();
        store
            .apply(Operation::SetTextSource {
                object_id: text_id.clone(),
                source: "Tax {{`Tax rate`}} on {{`Sales`.`Revenue`.sum()}}".into(),
            })
            .unwrap();

        let graph = store.dependency_graph(&text_id).unwrap();
        let names: HashSet<_> = graph
            .children
            .iter()
            .map(|child| child.name.as_str())
            .collect();
        assert_eq!(names, HashSet::from(["Tax rate", "Sales"]));
        assert_eq!(graph.children.len(), 2);
    }
}
