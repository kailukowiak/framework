//! What a hand-entered frame still accepts once it carries a calculated
//! column.
//!
//! The "Add calculated column" gesture appends a `withColumns` step to the
//! frame's Wrangle chain. `FrameObject::is_computed()` (engine/frame.rs)
//! treats any step other than Filter / Sort / Comment as making the frame
//! "computed", and the column lifecycle used to refuse every computed frame
//! with `DerivedFrameReadOnly` while the cell paths had been taught a finer
//! rule. The guards now ask `FrameObject::owns_its_columns` — the document
//! holds the rows and the chain only adds calculations — and report it as
//! `editing.columns`. These tests pin both halves, and that a chain which
//! reshapes still fixes the schema.

use crate::common::*;
use framework_core::*;

/// A literal frame plus a calculated column added the way the app does it:
/// a fresh output id, the blank `None.cast("number")` formula.
fn frame_with_calculated_column() -> (Store, Id, Vec<Column>) {
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Inputs".into(),
            grid: vec![
                vec!["Qty".into(), "Price".into()],
                vec!["2".into(), "5".into()],
                vec!["3".into(), "7".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inputs").clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "calc-column-1".into(),
                    name: "Column 1".into(),
                    formula: r#"None.cast("number")"#.into(),
                }],
            }],
        })
        .unwrap();
    let literal = frame_named(store.document(), "Inputs").columns.clone();
    (store, frame.id, literal)
}

#[test]
fn cell_and_row_editing_survive_a_calculated_column() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let qty = columns.iter().find(|c| c.name == "Qty").unwrap().clone();
    // The context menu offers Add column / Column type / Delete column when
    // `editing.rows` is true (ContextMenuFrameItems.tsx), so this is the
    // flag the menu is reading while the column operations refuse.
    let view = store.view();
    assert!(view.computed_frames[&frame_id].editing.rows);
    assert!(view.computed_frames[&frame_id].editing.cells);
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: qty.id.clone(),
            raw: "9".into(),
        })
        .expect("SetCell on a literal column still works");
    let mut values = std::collections::BTreeMap::new();
    values.insert(qty.id.clone(), "4".into());
    store
        .apply(Operation::AddRow {
            frame_id,
            values,
            before_row_id: None,
        })
        .expect("AddRow still works");
}

#[test]
fn add_column_after_a_calculated_column_should_succeed() {
    let (mut store, frame_id, _) = frame_with_calculated_column();
    let result = store.apply(Operation::AddColumn {
        frame_id,
        name: "Discount".into(),
        data_type: DataType::Number,
        after_column_id: None,
        before_column_id: None,
        values: Default::default(),
    });
    assert!(result.is_ok(), "got {:?}", result.err());
}

#[test]
fn delete_literal_column_after_a_calculated_column_should_succeed() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let price = columns.iter().find(|c| c.name == "Price").unwrap();
    let result = store.apply(Operation::DeleteColumn {
        frame_id,
        column_id: price.id.clone(),
    });
    assert!(result.is_ok(), "got {:?}", result.err());
}

#[test]
fn retype_literal_column_after_a_calculated_column_should_succeed() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let qty = columns.iter().find(|c| c.name == "Qty").unwrap();
    let result = store.apply(Operation::SetColumnType {
        frame_id,
        column_id: qty.id.clone(),
        data_type: DataType::String,
        scale: None,
    });
    assert!(result.is_ok(), "got {:?}", result.err());
}

/// A chain that reshapes still fixes the schema: a Select decides which
/// columns the frame shows, so a literal column added underneath it would be
/// invisible, and the column lifecycle stays refused exactly as before.
#[test]
fn a_reshaping_chain_still_refuses_the_column_lifecycle() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let qty = columns.iter().find(|c| c.name == "Qty").unwrap().clone();
    let price = columns.iter().find(|c| c.name == "Price").unwrap().clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::Select {
                column_ids: vec![qty.id.clone(), price.id.clone()],
            }],
        })
        .unwrap();
    assert!(!store.view().computed_frames[&frame_id].editing.columns);
    let add = store.apply(Operation::AddColumn {
        frame_id: frame_id.clone(),
        name: "Discount".into(),
        data_type: DataType::Number,
        after_column_id: None,
        before_column_id: None,
        values: Default::default(),
    });
    let del = store.apply(Operation::DeleteColumn {
        frame_id: frame_id.clone(),
        column_id: price.id.clone(),
    });
    let ty = store.apply(Operation::SetColumnType {
        frame_id,
        column_id: qty.id,
        data_type: DataType::String,
        scale: None,
    });
    assert!(
        matches!(add, Err(CoreError::DerivedFrameReadOnly)),
        "{add:?}"
    );
    assert!(
        matches!(del, Err(CoreError::DerivedFrameReadOnly)),
        "{del:?}"
    );
    assert!(matches!(ty, Err(CoreError::DerivedFrameReadOnly)), "{ty:?}");
}

/// The flag the frontend gates its column menu on says the same thing the
/// operations do, on every kind of frame.
#[test]
fn editing_columns_reports_whether_the_schema_is_the_documents() {
    let (store, frame_id, _) = frame_with_calculated_column();
    let view = store.view();
    assert!(view.computed_frames[&frame_id].editing.columns);
    let customers = frame_named(store.document(), "Customers").id.clone();
    assert!(view.computed_frames[&customers].editing.columns);

    let dir = temporary_test_directory("calculated-column-editing-columns");
    let path = dir.join("stock.csv");
    std::fs::write(&path, b"SKU,Count\nA,1\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let stock = frame_named(store.document(), "Stock").id.clone();
    assert!(!store.view().computed_frames[&stock].editing.columns);
}

/// An added literal column reaches the page with the calculated column still
/// computing beside it, and one undo takes it back out of both schemas.
#[test]
fn an_added_literal_column_is_an_input_the_chain_can_read() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let price = columns.iter().find(|c| c.name == "Price").unwrap().clone();
    store
        .apply(Operation::AddColumn {
            frame_id: frame_id.clone(),
            name: "Discount".into(),
            data_type: DataType::Number,
            after_column_id: None,
            before_column_id: Some(price.id.clone()),
            values: Default::default(),
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inputs").clone();
    let names: Vec<_> = frame.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Qty", "Discount", "Price", "Column 1"]);
    let inputs: Vec<_> = frame.base_columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(inputs, ["Qty", "Discount", "Price"]);
    let discount = frame.columns[1].id.clone();

    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: discount.clone(),
            raw: "1".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "calc-column-1".into(),
                    name: "Net".into(),
                    formula: "`Price` - `Discount`".into(),
                }],
            }],
        })
        .unwrap();
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    let net = page.columns.iter().position(|c| c.name == "Net").unwrap();
    assert_eq!(page.rows[0][net], "4");

    // Undo the chain edit, the cell, then the column.
    store.undo();
    store.undo();
    store.undo();
    let frame = frame_named(store.document(), "Inputs");
    assert!(frame.columns.iter().all(|c| c.id != discount));
    assert!(frame.base_columns.iter().all(|c| c.id != discount));
}

/// Deleting a literal column the chain reads is refused in words, the way a
/// column another formula reads always has been; deleting one it does not
/// read takes it out of the input schema too.
#[test]
fn deleting_a_literal_column_a_chain_formula_reads_is_refused() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let qty = columns.iter().find(|c| c.name == "Qty").unwrap().clone();
    let price = columns.iter().find(|c| c.name == "Price").unwrap().clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "calc-column-1".into(),
                    name: "Doubled".into(),
                    formula: "`Qty` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let error = store
        .apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: qty.id.clone(),
        })
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("reads ‘Qty’, so it cannot be deleted"),
        "{error}"
    );
    store
        .apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: price.id.clone(),
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inputs");
    assert!(frame.base_columns.iter().all(|c| c.id != price.id));
    let page = store.get_frame_page(&frame_id, 0, 10).unwrap();
    assert_eq!(page.rows[0], ["2", "4"]);
}

/// A calculated output is its formula's, not the column lifecycle's.
#[test]
fn a_calculated_column_cannot_be_deleted_or_retyped_as_if_literal() {
    let (mut store, frame_id, _) = frame_with_calculated_column();
    let delete = store
        .apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: "calc-column-1".into(),
        })
        .unwrap_err();
    assert!(
        delete.to_string().contains("calculated by Wrangle"),
        "{delete}"
    );
    let retype = store
        .apply(Operation::SetColumnType {
            frame_id,
            column_id: "calc-column-1".into(),
            data_type: DataType::String,
            scale: None,
        })
        .unwrap_err();
    assert!(
        retype.to_string().contains("calculated by Wrangle"),
        "{retype}"
    );
}

/// Renaming or retyping a literal column reaches the input schema the chain
/// is rendered against, so the chain's formulas say the new name.
#[test]
fn a_renamed_literal_column_is_renamed_where_the_chain_reads_it() {
    let (mut store, frame_id, columns) = frame_with_calculated_column();
    let qty = columns.iter().find(|c| c.name == "Qty").unwrap().clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "calc-column-1".into(),
                    name: "Doubled".into(),
                    formula: "`Qty` * 2".into(),
                }],
            }],
        })
        .unwrap();
    store
        .apply(Operation::RenameColumn {
            frame_id: frame_id.clone(),
            column_id: qty.id.clone(),
            name: "Units".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetColumnType {
            frame_id: frame_id.clone(),
            column_id: qty.id.clone(),
            data_type: DataType::Integer,
            scale: None,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inputs");
    let input = frame.base_columns.iter().find(|c| c.id == qty.id).unwrap();
    assert_eq!(input.name, "Units");
    assert_eq!(input.data_type, DataType::Integer);
    let rendered = format!("{:?}", store.view().computed_frames[&frame_id].steps);
    assert!(rendered.contains("`Units` * 2"), "{rendered}");
}
