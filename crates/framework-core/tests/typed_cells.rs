use crate::common::*;
use framework_core::*;
use std::collections::BTreeMap;

fn sales_store() -> (Store, FrameObject) {
    let mut store = Store::new(Document::blank("Typed"));
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Region".into(), "Revenue".into(), "Sold on".into()],
                vec!["East".into(), "142000".into(), "2026-01-15".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Sales").clone();
    (store, frame)
}

fn set_cell(frame: &FrameObject, column: usize, raw: &str) -> Operation {
    Operation::SetCell {
        frame_id: frame.id.clone(),
        row_id: frame.rows[0].id.clone(),
        column_id: frame.columns[column].id.clone(),
        raw: raw.into(),
    }
}

/// A typed column refuses text it cannot hold, naming the column and the
/// type it wanted, on every path a value arrives by: one cell, a range, a
/// paste, a new row. Storing the text and showing a null was the silent
/// alternative, and the one a spreadsheet hand least expects.
#[test]
fn typed_columns_refuse_text_they_cannot_parse() {
    let (mut store, frame) = sales_store();
    let revenue = &frame.columns[1];
    assert_eq!(revenue.data_type, DataType::Integer);
    assert_eq!(frame.columns[2].data_type, DataType::Date);

    let refused = store
        .apply(set_cell(&frame, 1, "= 12*100"))
        .expect_err("text in an integer column is refused");
    assert_eq!(
        refused.to_string(),
        "'= 12*100' is not a valid integer for column 'Revenue'; use a whole number"
    );
    assert_eq!(
        frame_named(store.document(), "Sales").rows[0].cells[&revenue.id].raw,
        "142000"
    );

    // What evaluation reads as a number is accepted as one, blanks included.
    store.apply(set_cell(&frame, 1, "151,000")).unwrap();
    store.apply(set_cell(&frame, 1, "")).unwrap();
    store
        .apply(set_cell(&frame, 0, "= anything goes in text"))
        .unwrap();

    let refused = store
        .apply(set_cell(&frame, 2, "soon"))
        .expect_err("a date column wants a date");
    assert!(
        refused
            .to_string()
            .ends_with("for column 'Sold on'; use YYYY-MM-DD"),
        "{refused}"
    );

    let refused = store
        .apply(Operation::SetCells {
            frame_id: frame.id.clone(),
            cells: vec![CellUpdate {
                row_id: frame.rows[0].id.clone(),
                column_id: revenue.id.clone(),
                raw: "n/a".into(),
            }],
        })
        .expect_err("a range write with text in it is refused");
    assert!(
        refused.to_string().contains("'n/a' is not a valid integer"),
        "{refused}"
    );

    let refused = store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: revenue.id.clone(),
            grid: vec![vec!["1000".into()], vec!["abc".into()]],
        })
        .expect_err("a paste with text in a numeric column is refused whole");
    assert!(
        refused.to_string().contains("'abc' is not a valid integer"),
        "{refused}"
    );
    assert_eq!(frame_named(store.document(), "Sales").rows.len(), 1);

    let refused = store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::from([(revenue.id.clone(), "lots".to_string())]),
        })
        .expect_err("a new row with text in a numeric column is refused");
    assert!(
        refused
            .to_string()
            .contains("'lots' is not a valid integer"),
        "{refused}"
    );
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::from([(revenue.id.clone(), "1200".to_string())]),
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Sales").rows.len(), 2);
}
