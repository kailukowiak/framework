//! The exact amount: `Accounting` is Decimal128 at a declared scale, so
//! cents foot and a sum ties, and it presents in accounting style without
//! being asked.

use crate::common::frame_named;
use framework_core::*;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Ledger".into(),
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

/// A ledger whose `Amount` column has been called accounting, with the
/// three amounts every float gets wrong.
fn ledger(amounts: &[&str]) -> (Store, FrameObject) {
    let mut store = blank_store();
    let mut grid = vec![vec!["Amount".to_string(), "Rate".to_string()]];
    for amount in amounts {
        grid.push(vec![amount.to_string(), "8.25%".into()]);
    }
    store
        .apply(Operation::AddFrame {
            name: "Ledger".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: frame.columns[0].id.clone(),
            data_type: DataType::Accounting,
            scale: None,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    (store, frame)
}

fn column<'a>(frame: &'a FrameObject, name: &str) -> &'a Column {
    frame
        .columns
        .iter()
        .find(|column| column.name == name)
        .unwrap()
}

fn add_column(
    store: &mut Store,
    frame_id: &str,
    name: &str,
    formula: &str,
) -> Result<(), CoreError> {
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.into(),
            name: name.into(),
            formula: formula.into(),
            after_column_id: None,
        })
        .map(|_| ())
}

fn cell(store: &Store, frame: &FrameObject, row: usize, name: &str) -> ComputedCell {
    let frame = frame_named(store.document(), &frame.name);
    store.view().computed_frames[&frame.id].rows[&frame.rows[row].id][&column(frame, name).id]
        .clone()
}

#[test]
fn an_accounting_column_is_exact_where_a_float_is_not() {
    let (mut store, frame) = ledger(&["0.10", "0.20", "0.30"]);
    let amount = column(&frame, "Amount");
    assert_eq!(amount.data_type, DataType::Accounting);
    assert_eq!(
        amount.scale,
        Some(2),
        "an amount gets the default scale when none is asked for"
    );

    store
        .apply(Operation::AddSummary {
            frame_id: frame.id.clone(),
            column_id: amount.id.clone(),
            operation: SummaryOperation::Sum,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    let total = &store.view().computed_frames[&frame.id].summaries[&frame.summaries[0].id];
    // 0.1 + 0.2 + 0.3 is 0.6000000000000001 in a float. Not here.
    assert_eq!(total.typed_value, ScalarValue::Number(0.6));
    assert!(total.error.is_none());

    // The arithmetic stays exact too, and so does what it is called.
    add_column(&mut store, &frame.id, "Triple", "`Amount` * 3").unwrap();
    add_column(&mut store, &frame.id, "Tax", "`Amount` * `Rate`").unwrap();
    add_column(&mut store, &frame.id, "Third", "`Amount` / 3").unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Triple").data_type, DataType::Accounting);
    assert_eq!(column(&frame, "Triple").scale, Some(2));
    assert_eq!(
        cell(&store, &frame, 0, "Triple").typed_value,
        ScalarValue::Number(0.3)
    );
    // A rate column is a float; it is brought to decimal with nine places
    // rather than the amount being dropped to a float.
    assert_eq!(column(&frame, "Tax").data_type, DataType::Accounting);
    assert_eq!(column(&frame, "Tax").scale, Some(9));
    assert_eq!(
        cell(&store, &frame, 0, "Tax").typed_value,
        ScalarValue::Number(0.00825)
    );
    // Division is at the amount's scale, not a float's.
    assert_eq!(column(&frame, "Third").scale, Some(2));
    assert_eq!(
        cell(&store, &frame, 2, "Third").typed_value,
        ScalarValue::Number(0.1)
    );
}

#[test]
fn a_written_number_brings_its_own_places() {
    let (mut store, frame) = ledger(&["100.00"]);
    add_column(&mut store, &frame.id, "Levy", "`Amount` * 0.0825").unwrap();
    add_column(&mut store, &frame.id, "Half", "`Amount` * 1.5").unwrap();
    add_column(&mut store, &frame.id, "Plus", "`Amount` + 0.005").unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Levy").scale, Some(4));
    assert_eq!(
        cell(&store, &frame, 0, "Levy").typed_value,
        ScalarValue::Number(8.25)
    );
    assert_eq!(column(&frame, "Half").scale, Some(2));
    assert_eq!(
        cell(&store, &frame, 0, "Half").typed_value,
        ScalarValue::Number(150.0)
    );
    assert_eq!(column(&frame, "Plus").scale, Some(3));
    assert_eq!(
        cell(&store, &frame, 0, "Plus").typed_value,
        ScalarValue::Number(100.005)
    );
}

#[test]
fn amounts_are_read_the_way_accountants_write_them() {
    let (mut store, frame) = ledger(&["(12.50)", "$1,234.56", "-0.05", "7"]);
    let amount = column(&frame, "Amount");
    assert_eq!(
        cell(&store, &frame, 0, "Amount").typed_value,
        ScalarValue::Number(-12.5)
    );
    assert_eq!(
        cell(&store, &frame, 1, "Amount").typed_value,
        ScalarValue::Number(1234.56)
    );
    assert_eq!(
        cell(&store, &frame, 2, "Amount").typed_value,
        ScalarValue::Number(-0.05)
    );
    assert_eq!(
        cell(&store, &frame, 3, "Amount").typed_value,
        ScalarValue::Number(7.0)
    );

    // A cell edit is held to the same reading.
    let row_id = frame.rows[3].id.clone();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: row_id.clone(),
            column_id: amount.id.clone(),
            raw: "(1,000.25)".into(),
        })
        .unwrap();
    assert_eq!(
        cell(&store, &frame, 3, "Amount").typed_value,
        ScalarValue::Number(-1000.25)
    );
    let refused = store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id,
            column_id: amount.id.clone(),
            raw: "twelve".into(),
        })
        .unwrap_err();
    assert!(refused.to_string().contains("not a valid"), "{refused}");
}

#[test]
fn the_scale_is_declared_on_the_column_and_survives_undo() {
    let (mut store, frame) = ledger(&["1.23456"]);
    // The default scale rounds what it holds.
    assert_eq!(
        cell(&store, &frame, 0, "Amount").typed_value,
        ScalarValue::Number(1.23)
    );
    let amount_id = column(&frame, "Amount").id.clone();
    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: amount_id.clone(),
            data_type: DataType::Accounting,
            scale: Some(4),
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Amount").scale, Some(4));
    assert_eq!(
        cell(&store, &frame, 0, "Amount").typed_value,
        ScalarValue::Number(1.2346)
    );

    store.undo();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Amount").scale, Some(2));

    // The scale goes when the type does.
    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: amount_id.clone(),
            data_type: DataType::Number,
            scale: None,
        })
        .unwrap();
    assert_eq!(
        column(&frame_named(store.document(), "Ledger"), "Amount").scale,
        None
    );

    // And only an amount has one.
    let refused = store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: amount_id,
            data_type: DataType::Number,
            scale: Some(3),
        })
        .unwrap_err();
    assert!(
        refused.to_string().contains("accounting column"),
        "{refused}"
    );
}

#[test]
fn money_and_an_amount_do_not_meet_without_a_cast() {
    let (mut store, frame) = ledger(&["10.00"]);
    add_column(&mut store, &frame.id, "Price", "$2.50").unwrap();
    let refused = add_column(&mut store, &frame.id, "Bad", "`Amount` + `Price`").unwrap_err();
    assert!(
        refused.to_string().contains("cast one side first"),
        "{refused}"
    );

    add_column(
        &mut store,
        &frame.id,
        "Exact",
        "`Amount` + `Price`.cast(\"accounting\")",
    )
    .unwrap();
    add_column(
        &mut store,
        &frame.id,
        "Model",
        "`Amount`.cast(\"currency\") + `Price`",
    )
    .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Exact").data_type, DataType::Accounting);
    assert_eq!(
        cell(&store, &frame, 0, "Exact").typed_value,
        ScalarValue::Number(12.5)
    );
    assert_eq!(column(&frame, "Model").data_type, DataType::Currency);
}

#[test]
fn cast_names_the_scale_and_text_reads_the_exact_digits() {
    let (mut store, frame) = ledger(&["12.5"]);
    add_column(
        &mut store,
        &frame.id,
        "Fine",
        "`Rate`.cast(\"accounting\", 4)",
    )
    .unwrap();
    add_column(
        &mut store,
        &frame.id,
        "Coarse",
        "`Rate`.cast(\"accounting\")",
    )
    .unwrap();
    add_column(&mut store, &frame.id, "Text", "\"Owed \" + `Amount`").unwrap();
    add_column(&mut store, &frame.id, "Whole", "`Amount`.cast(\"integer\")").unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Fine").scale, Some(4));
    assert_eq!(
        cell(&store, &frame, 0, "Fine").typed_value,
        ScalarValue::Number(0.0825)
    );
    assert_eq!(column(&frame, "Coarse").scale, Some(2));
    assert_eq!(
        cell(&store, &frame, 0, "Coarse").typed_value,
        ScalarValue::Number(0.08)
    );
    assert_eq!(
        cell(&store, &frame, 0, "Text").typed_value,
        ScalarValue::String("Owed 12.50".into())
    );
    assert_eq!(column(&frame, "Whole").data_type, DataType::Integer);

    let refused = add_column(
        &mut store,
        &frame.id,
        "Bad",
        "`Rate`.cast(\"accounting\", \"two\")",
    )
    .unwrap_err();

    // Text read from a file is an amount by the same rules as a cell.
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Imported".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["(1,250.75)".into()],
                vec!["$3.5".into()],
                vec!["note".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let imported = frame_named(store.document(), "Imported").clone();
    assert_eq!(column(&imported, "Amount").data_type, DataType::String);
    add_column(
        &mut store,
        &imported.id,
        "Read",
        "`Amount`.cast(\"accounting\")",
    )
    .unwrap();
    let imported = frame_named(store.document(), "Imported").clone();
    assert_eq!(column(&imported, "Read").data_type, DataType::Accounting);
    assert_eq!(
        cell(&store, &imported, 0, "Read").typed_value,
        ScalarValue::Number(-1250.75)
    );
    assert_eq!(
        cell(&store, &imported, 1, "Read").typed_value,
        ScalarValue::Number(3.5)
    );
    assert_eq!(
        cell(&store, &imported, 2, "Read").typed_value,
        ScalarValue::Null
    );
    assert!(refused.to_string().contains("decimal places"), "{refused}");
}

#[test]
fn a_derived_step_keeps_the_amount_exact_and_the_scale_with_it() {
    let (mut store, frame) = ledger(&["0.10", "0.20", "0.30"]);
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: frame.columns[0].id.clone(),
                        descending: false,
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: uuid::Uuid::new_v4().to_string(),
                        name: "Running".into(),
                        formula: "`Amount`.cum_sum(False)".into(),
                    }],
                },
            ],
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    let running = column(&frame, "Running");
    assert_eq!(running.data_type, DataType::Accounting);
    assert_eq!(running.scale, Some(2));
}
