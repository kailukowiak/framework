//! `AddColumn` carrying initial values: typing into the column that does not
//! exist yet is one edit, and so one undo step.

use crate::common::*;
use framework_core::*;
use std::{collections::BTreeMap, fs};

fn inputs_store() -> (Store, FrameObject) {
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Inputs".into(),
            grid: vec![
                vec!["Qty".into()],
                vec!["2".into()],
                vec!["3".into()],
                vec!["4".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inputs").clone();
    (store, frame)
}

fn add(frame: &FrameObject, data_type: DataType, values: BTreeMap<Id, String>) -> Operation {
    Operation::AddColumn {
        frame_id: frame.id.clone(),
        name: "Extra".into(),
        data_type,
        after_column_id: None,
        before_column_id: None,
        values,
    }
}

/// The new column's raw values top to bottom, or `None` when it is absent.
fn extra(store: &Store) -> Option<Vec<String>> {
    let frame = frame_named(store.document(), "Inputs");
    let column = frame.columns.iter().find(|column| column.name == "Extra")?;
    Some(
        frame
            .rows
            .iter()
            .map(|row| row.cells[&column.id].raw.clone())
            .collect(),
    )
}

#[test]
fn values_land_in_the_named_rows_and_the_rest_stay_blank() {
    let (mut store, frame) = inputs_store();
    let values = BTreeMap::from([(frame.rows[1].id.clone(), "42".to_string())]);
    store.apply(add(&frame, DataType::Number, values)).unwrap();
    assert_eq!(extra(&store).unwrap(), ["", "42", ""]);
}

#[test]
fn undo_removes_the_column_and_redo_restores_its_values_exactly() {
    let (mut store, frame) = inputs_store();
    let before = store.document().clone();
    let values = BTreeMap::from([
        (frame.rows[0].id.clone(), "1".to_string()),
        (frame.rows[2].id.clone(), "3".to_string()),
    ]);
    store.apply(add(&frame, DataType::Number, values)).unwrap();
    let after = store.document().clone();
    store.undo();
    assert_eq!(extra(&store), None);
    assert_eq!(store.document().objects, before.objects);
    store.redo();
    assert_eq!(extra(&store).unwrap(), ["1", "", "3"]);
    assert_eq!(store.document().objects, after.objects);
}

#[test]
fn an_unknown_row_refuses_and_leaves_nothing() {
    let (mut store, frame) = inputs_store();
    let before = store.document().clone();
    let values = BTreeMap::from([("no-such-row".to_string(), "1".to_string())]);
    let err = store
        .apply(add(&frame, DataType::Number, values))
        .unwrap_err();
    assert!(matches!(err, CoreError::RowNotFound), "{err:?}");
    assert_eq!(extra(&store), None);
    assert_eq!(store.document().objects, before.objects);
}

#[test]
fn a_value_the_type_refuses_refuses_the_whole_add() {
    let (mut store, frame) = inputs_store();
    let before = store.document().clone();
    let values = BTreeMap::from([
        (frame.rows[0].id.clone(), "5".to_string()),
        (frame.rows[1].id.clone(), "not a number".to_string()),
    ]);
    assert!(store.apply(add(&frame, DataType::Number, values)).is_err());
    assert_eq!(extra(&store), None);
    assert_eq!(store.document().objects, before.objects);
}

#[test]
fn values_are_refused_on_a_frame_that_does_not_own_its_rows() {
    let dir = temporary_test_directory("add-column-values-file");
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: Some(path.display().to_string()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert!(!page.rows.is_empty());
    let values = BTreeMap::from([("0".to_string(), "x".to_string())]);
    let operation = Operation::AddColumn {
        frame_id: frame.id.clone(),
        name: "Extra".into(),
        data_type: DataType::String,
        after_column_id: None,
        before_column_id: None,
        values,
    };
    assert!(store.apply(operation).is_err());
    assert!(
        !frame_named(store.document(), "Stock")
            .columns
            .iter()
            .any(|column| column.name == "Extra")
    );
}
