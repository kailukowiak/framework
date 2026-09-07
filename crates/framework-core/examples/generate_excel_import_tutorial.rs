use framework_core::{DataObject, Document, Operation, Store, create_excel_range_artifact};
use std::fs;
use std::path::{Path, PathBuf};

const LESSON: &str = include_str!("../../../tutorials/excel-import/README.md");

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

/// The height the engine actually gave the named frame's card, so the next
/// row can be placed below it instead of at a guessed offset — cards size
/// to their row count now (`frame_card_height` in `engine/build.rs`), so a
/// fixed gap between rows silently overlaps once a card grows past ~450px.
fn frame_view_height(store: &Store, name: &str) -> f64 {
    let object_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == name => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("frame {name} exists"));
    store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == object_id)
        .unwrap_or_else(|| panic!("frame {name} has a view"))
        .height
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
        name: "Tutorial walkthrough".into(),
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
    let row1_y = 880.0;
    import_range(
        &mut store,
        &simple,
        &data,
        ImportSpec {
            name: "Customers",
            sheet: "Customers",
            range: "A4:D11",
            x: 60.0,
            y: row1_y,
        },
    )?;
    for (name, sheet, range, x, y) in [
        ("Inventory", "Operations", "A4:F10", 650.0, row1_y),
        ("Suppliers", "Operations", "H4:L8", 1240.0, row1_y),
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
    // The second row starts below the tallest card the first row actually
    // got, not a guessed offset: frame cards now size to their row count.
    let row1_height = ["Customers", "Inventory", "Suppliers"]
        .into_iter()
        .map(|name| frame_view_height(&store, name))
        .fold(0.0_f64, f64::max);
    let row2_y = row1_y + row1_height + 40.0;
    for (name, sheet, range, x) in [
        ("Orders", "Sales", "B5:I25", 60.0),
        ("Adjustments", "Operations", "A15:D23", 650.0),
        ("Targets", "Sales", "P15:S25", 1240.0),
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
                y: row2_y,
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
