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

#[test]
fn a_select_step_keeps_literal_columns_editable_and_paged() {
    // Rearranging or deleting columns is a Select step in the frame's own
    // chain. A row that lost a column is still the same row, so one drag to
    // tidy columns must not freeze every literal cell of a hand-entered
    // table -- the regression this test pins down.
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Tidy inputs".into(),
            grid: vec![
                vec!["Line".into(), "SKU".into(), "Note".into()],
                vec!["1".into(), "A-100".into(), "keep".into()],
                vec!["2".into(), "B-200".into(), "drop".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Tidy inputs").clone();
    let line = frame.columns[0].clone();
    let sku = frame.columns[1].clone();

    // Reorder and drop in one Select: SKU first, Line second, Note gone.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Select {
                column_ids: vec![sku.id.clone(), line.id.clone()],
            }],
        })
        .unwrap();

    let view = store.view();
    let computed = &view.computed_frames[&frame.id];
    assert!(
        computed.editing.cells,
        "a surviving literal column still takes typing through a Select"
    );
    assert!(
        computed.editing.rows,
        "the add-row line survives a column rearrange"
    );

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        page.rows[0].len(),
        2,
        "the page projects the columns the Select kept, not the stored schema"
    );

    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: sku.id.clone(),
            raw: "A-101".into(),
        })
        .unwrap();
    let edited = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert!(
        edited.rows[0].contains(&"A-101".to_string()),
        "the typed value lands in the stored row behind the Select"
    );
}

#[test]
fn an_own_chain_reshape_names_the_chain_not_an_import() {
    // A Summarize in a hand-entered frame's own chain shows chain output,
    // so cells rightly refuse typing -- but the reason must name the chain.
    // The old fallback called typed-in rows "the copy imported into this
    // document", which taught exactly the wrong remedy.
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Reshaped inputs".into(),
            grid: vec![
                vec!["Line".into(), "Amount".into()],
                vec!["1".into(), "10".into()],
                vec!["2".into(), "20".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Reshaped inputs").clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Summarize {
                group_keys: Vec::new(),
                aggregates: vec![ExistingFormulaInput {
                    output_column_id: framework_core::id(),
                    name: "total".into(),
                    formula: "`Amount`.sum()".into(),
                }],
                maintain_order: true,
            }],
        })
        .unwrap();

    let view = store.view();
    let computed = &view.computed_frames[&frame.id];
    assert!(!computed.editing.cells);
    let reason = computed.editing.reason.as_deref().unwrap_or_default();
    assert!(
        reason.contains("computed by the chain"),
        "the refusal names the chain, got: {reason}"
    );
    assert!(
        !reason.contains("imported"),
        "typed-in rows must not be called an import, got: {reason}"
    );
}
