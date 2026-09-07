//! Generate the dictionary lesson from public operations, including its
//! answer key. The source table and dictionary stay small enough to inspect
//! together, and the guide lives in the workbook rather than only on disk.
use framework_core::*;
use std::path::PathBuf;

fn object(store: &Store, name: &str) -> DataObject {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .clone()
}
fn frame(store: &Store, name: &str) -> FrameObject {
    match object(store, name) {
        DataObject::Frame(frame) => frame,
        _ => unreachable!(),
    }
}
fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|value| value.to_string()).collect())
        .collect()
}
fn prepare() -> Result<Store, CoreError> {
    let mut store = Store::new_tutorial(Document::blank("Dictionaries and value mapping"));
    store.apply(Operation::AddFrame {
        name: "Expenses".into(),
        x: 590.0,
        y: 70.0,
        grid: grid(&[
            &["Category", "Amount"],
            &["Offce supplies", "100"],
            &["Travel & meals", "50"],
            &["Software", "30"],
            &["Offce supplies", "20"],
        ]),
    })?;
    store.apply(Operation::AddFrame {
        name: "Category fixes".into(),
        x: 1000.0,
        y: 70.0,
        grid: grid(&[
            &["Key", "Value"],
            &["Offce supplies", "Office supplies"],
            &["Travel & meals", "Travel"],
        ]),
    })?;
    store.apply(Operation::AddBlock {
        name: "Checks".into(),
        x: 590.0,
        y: 370.0,
    })?;
    store.apply(Operation::SetBlockSource {
        block_id: object(&store, "Checks").id().to_string(),
        source: "total = `Expenses`.`Amount`.sum()".into(),
        editing: None,
    })?;
    store.apply(Operation::AddText { x: 40.0, y: 70.0 })?;
    let guide = object(&store, "Text").id().to_string();
    store.apply(Operation::RenameObject {
        object_id: guide.clone(),
        name: "Tutorial walkthrough".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: guide.clone(),
        source: include_str!("../../../tutorials/dictionaries/README.md").into(),
    })?;
    let view = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == guide)
        .unwrap()
        .id
        .clone();
    store.apply(Operation::ResizeView {
        view_id: view,
        width: 510.0,
        height: 850.0,
    })?;
    Ok(store)
}
fn finish(store: &mut Store) -> Result<(), CoreError> {
    let dictionary = frame(store, "Category fixes");
    store.apply(Operation::SetUniqueKey {
        frame_id: dictionary.id,
        column_ids: vec![dictionary.columns[0].id.clone()],
        enabled: true,
    })?;
    let expenses = frame(store, "Expenses");
    store.apply(Operation::SetFramePipeline {
        frame_id: expenses.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: expenses.columns[0].id.clone(),
                name: "Category".into(),
                formula: "map_values(`Category`, `Category fixes`.`Key`, `Category fixes`.`Value`)"
                    .into(),
            }],
        }],
    })?;
    store.apply(Operation::SetBlockSource { block_id: object(store, "Checks").id().to_string(), source: "total = `Expenses`.`Amount`.sum()\nlabel = lookup(\"Offce supplies\", `Category fixes`.`Key`, `Category fixes`.`Value`)\nmissing = lookup(\"Software\", `Category fixes`.`Key`, `Category fixes`.`Value`, \"Not mapped\")".into(), editing: None })?;
    assert_eq!(
        store.get_frame_page(&expenses.id, 0, 10)?.rows[0][0],
        "Office supplies"
    );
    let view = store.view();
    let checks = &view.computed_blocks[object(store, "Checks").id()];
    assert_eq!(checks.lines[0].cell.value, Some(200.0));
    assert_eq!(checks.lines[1].cell.display, "Office supplies");
    assert_eq!(checks.lines[2].cell.display, "Not mapped");
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tutorials/dictionaries");
    std::fs::create_dir_all(&output)?;
    let mut store = prepare()?;
    store.save(&output.join("dictionaries-start.fw"))?;
    finish(&mut store)?;
    store.save(&output.join("dictionaries-finished.fw"))?;
    Ok(())
}
