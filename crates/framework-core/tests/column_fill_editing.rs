use crate::common::*;
use framework_core::*;
use std::collections::BTreeMap;

#[test]
fn a_series_fill_keeps_other_columns_and_row_entry_editable() {
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Launch inputs".into(),
            grid: vec![
                vec!["Line".into(), "SKU".into()],
                vec!["3".into(), "C-300".into()],
                vec!["1".into(), "A-100".into()],
                vec!["2".into(), "B-200".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Launch inputs").clone();
    let line = frame.columns[0].clone();
    let sku = frame.columns[1].clone();
    let original_row_for_one = frame.rows[1].id.clone();
    let formula = "sequence(1, 1 + 1 * frame.len(), step=1)";

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: line.id.clone(),
                        descending: false,
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: line.id.clone(),
                        name: line.name.clone(),
                        formula: formula.into(),
                    }],
                },
            ],
        })
        .unwrap();

    let view = store.view();
    let computed = &view.computed_frames[&frame.id];
    assert!(
        computed.editing.cells,
        "an unaffected input column still takes typing"
    );
    assert!(
        computed.editing.rows,
        "the empty add-row line remains available"
    );
    assert_eq!(computed.formulas[&line.id], formula);

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(page.rows[0], ["1", "A-100"]);
    assert_eq!(page.row_ids[0], original_row_for_one);

    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: sku.id.clone(),
            raw: "A-101".into(),
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().rows[0],
        ["1", "A-101"]
    );

    let filled_edit = store.apply(Operation::SetCell {
        frame_id: frame.id.clone(),
        row_id: page.row_ids[0].clone(),
        column_id: line.id.clone(),
        raw: "99".into(),
    });
    assert!(
        filled_edit
            .unwrap_err()
            .to_string()
            .contains("calculated by Wrangle")
    );

    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::from([(sku.id.clone(), "D-400".into())]),
        })
        .unwrap();
    let grown = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(grown.rows.len(), 4);
    assert_eq!(grown.rows[3], ["4", "D-400"]);
}
