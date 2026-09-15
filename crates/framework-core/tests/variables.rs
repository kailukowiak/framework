#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Variables".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
        calendars: Vec::new(),
        default_calendar_id: None,
    })
}

fn variable<'a>(store: &'a Store, name: &str) -> &'a ResultObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Result(result) if result.variable && result.name == name => Some(result),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_compact_variable_may_hold_a_scalar_date_or_vector_formula() {
    let mut store = blank_store();
    store
        .apply(Operation::AddVariable {
            name: "`my variable name`".into(),
            formula: "2".into(),
            x: 40.0,
            y: 60.0,
        })
        .unwrap();

    let id = variable(&store, "my variable name").id.clone();
    let view = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == id)
        .unwrap();
    assert_eq!((view.width, view.height), (320.0, 46.0));
    let computed = &store.view().computed_results[&id];
    assert_eq!(computed.cell.value, Some(2.0));
    assert_eq!(computed.value_count, 1);

    store
        .apply(Operation::SetResultFormula {
            object_id: id.clone(),
            formula: "[1, 2, 3]".into(),
        })
        .unwrap();
    let computed = &store.view().computed_results[&id];
    assert_eq!(computed.value_count, 3);
    assert_eq!(computed.cell.display, "[1, 2, 3]");

    store
        .apply(Operation::SetResultFormula {
            object_id: id.clone(),
            formula: "\"2024-10-10\"d".into(),
        })
        .unwrap();
    let computed = &store.view().computed_results[&id];
    assert_eq!(computed.data_type, DataType::Date);
    assert_eq!(computed.cell.display, "2024-10-10");
    assert_eq!(computed.value_count, 1);

    store
        .apply(Operation::RenameObject {
            object_id: id,
            name: "`forecast date`".into(),
        })
        .unwrap();
    assert_eq!(variable(&store, "forecast date").name, "forecast date");
}

#[test]
fn a_variable_keeps_one_identity_when_its_formula_changes_shape() {
    let mut store = blank_store();
    store
        .apply(Operation::AddVariable {
            name: "Factors".into(),
            formula: "[1, 2, 3]".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let id = variable(&store, "Factors").id.clone();

    store
        .apply(Operation::AddBlock {
            name: "Work".into(),
            x: 0.0,
            y: 80.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::SetBlockSource {
            block_id,
            source: "scaled = `Factors` * 2".into(),
            editing: None,
        })
        .unwrap();
    assert_eq!(
        store.view().computed_blocks.values().next().unwrap().lines[0].value_count,
        3
    );

    store
        .apply(Operation::SetResultFormula {
            object_id: id.clone(),
            formula: "5".into(),
        })
        .unwrap();
    assert_eq!(variable(&store, "Factors").id, id);
    let view = store.view();
    let line = &view.computed_blocks.values().next().unwrap().lines[0];
    assert_eq!(line.value_count, 1);
    assert_eq!(line.cell.value, Some(10.0));
}
