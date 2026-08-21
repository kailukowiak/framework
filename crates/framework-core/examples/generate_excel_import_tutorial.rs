use framework_core::{DataObject, Document, Operation, Store, create_excel_range_artifact};
use std::fs;
use std::path::{Path, PathBuf};

const LESSON: &str = r#"# Importing Excel data

Two example workbooks are beside this FrameWork file. Excel is an interchange source here: FrameWork imports cached cell values from one explicit table or range. It does not copy Excel formulas, formatting, or workbook logic.

## 1. Import one clean table

1. Open **Data Library** and choose **Excel range…**.
2. Choose `simple-customers.xlsx` beside this file.
3. Select `CustomersTable` — or enter `Customers!A4:D11` with **First row contains headers** enabled.
4. Preview the values, name the table `Customers`, and import it.

## 2. Import several defined tables from one workbook

Choose `multi-table-operations.xlsx`. Import each named Excel Table separately:

- `InventoryTable` from `Operations!A4:F10`;
- `SuppliersTable` from `Operations!H4:L8`;
- `OrdersTable` from `Sales!B5:I25`.

## 3. Import pasted ranges without Excel Tables

The same workbook also contains two ordinary rectangular blocks. They are not defined Excel Tables. Choose them under **Suggested region**:

- `Operations!A15:D23`, named `Adjustments`;
- `Sales!P15:S25`, named `Targets`.

The note in `Operations!N2` is intentionally ignored. Suggestions are conservative starting points: preview them before importing, and type a manual range when a workbook is too irregular.

## Checkpoint

You should have six static FrameWork tables: `Customers`, `Inventory`, `Suppliers`, `Orders`, `Adjustments`, and `Targets`. `Orders.Revenue` contains cached numeric results; its Excel formulas were not imported.
"#;

fn text_id(store: &Store) -> String {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Text(text) => Some(text.id.clone()),
            _ => None,
        })
        .expect("tutorial instructions exist")
}

fn view_id(store: &Store, object_id: &str) -> String {
    store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == object_id)
        .expect("tutorial instructions have a view")
        .id
        .clone()
}

fn set_instructions(store: &mut Store, source: String) -> Result<(), framework_core::CoreError> {
    let id = text_id(store);
    store.apply(Operation::RenameObject {
        object_id: id.clone(),
        name: "Import steps".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: id.clone(),
        source,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(store, &id),
        width: 920.0,
        height: 770.0,
    })?;
    Ok(())
}

struct ImportSpec<'a> {
    name: &'a str,
    sheet: &'a str,
    range: &'a str,
    x: f64,
    y: f64,
}

fn import_range(
    store: &mut Store,
    source: &Path,
    data_directory: &Path,
    spec: ImportSpec<'_>,
) -> Result<(), framework_core::CoreError> {
    let (artifact, _) =
        create_excel_range_artifact(source, data_directory, spec.sheet, spec.range, true)?;
    store.apply(Operation::ImportFrameFromArtifact {
        name: spec.name.into(),
        artifact,
        connector: None,
        x: spec.x,
        y: spec.y,
    })?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let output = workspace.join("tutorials/excel-import");
    let sources = output.join("source");
    let data = output.join("finished-data");
    if data.exists() {
        fs::remove_dir_all(&data)?;
    }
    fs::create_dir_all(&data)?;

    let mut store = Store::new_tutorial(Document::blank("Importing an Excel workbook"));
    store.apply(Operation::AddText { x: 60.0, y: 50.0 })?;
    set_instructions(&mut store, LESSON.into())?;
    store.save(&output.join("excel-import-start.fw"))?;

    let simple = sources.join("simple-customers.xlsx");
    let complex = sources.join("multi-table-operations.xlsx");
    import_range(
        &mut store,
        &simple,
        &data,
        ImportSpec {
            name: "Customers",
            sheet: "Customers",
            range: "A4:D11",
            x: 60.0,
            y: 880.0,
        },
    )?;
    for (name, sheet, range, x, y) in [
        ("Inventory", "Operations", "A4:F10", 650.0, 880.0),
        ("Suppliers", "Operations", "H4:L8", 1240.0, 880.0),
        ("Orders", "Sales", "B5:I25", 60.0, 1270.0),
        ("Adjustments", "Operations", "A15:D23", 650.0, 1270.0),
        ("Targets", "Sales", "P15:S25", 1240.0, 1270.0),
    ] {
        import_range(
            &mut store,
            &complex,
            &data,
            ImportSpec {
                name,
                sheet,
                range,
                x,
                y,
            },
        )?;
    }
    set_instructions(
        &mut store,
        format!(
            "{LESSON}\n\n## Answer key\n\nThe six completed imports are laid out below. They are static, artifact-backed tables—the same result produced by the Excel range dialog."
        ),
    )?;
    store.save(&output.join("excel-import-finished.fw"))?;
    Ok(())
}
