//! Generates `DocumentView` JSON fixtures for frontend interaction tests.
//!
//! `DocumentView` is exactly what the frontend receives over Tauri IPC —
//! the document plus everything computed from it, `Serialize`d with
//! camelCase fields. A frontend test that wants to render a frame or a
//! block without launching the desktop app, the Rust engine, and a real
//! IPC round trip needs a `DocumentView` to render, and hand-writing one as
//! a TypeScript literal drifts from the real shape the moment a field is
//! added or renamed on the Rust side. Generating it here instead means the
//! fixture is the real struct, serialized the real way.
//!
//! Every fixture below is built exclusively through public `Operation`s
//! applied to a `Store` — the same mutation boundary MCP and the desktop
//! app use, rather than a hand-assembled `Document` literal. That keeps
//! these files honest: a fixture only a privileged constructor could reach,
//! but no real operation sequence could, is a fixture that silently drifts
//! from what the app is actually capable of producing.

use framework_core::{
    ArtifactFormat, ConnectorRecipe, DataArtifact, DataObject, Document, DocumentView,
    ExistingFormulaInput, FrameStepInput, Operation, Store,
};
use polars::prelude as pl;
#[path = "generate_ui_fixtures/parameters.rs"]
mod parameters;
#[path = "generate_ui_fixtures/sensitivity.rs"]
mod sensitivity;
use std::path::{Path, PathBuf};

/// Finds a block's id by name. `AddBlock` mints an id nothing else in this
/// file learns, and a name lookup stays correct regardless of what other
/// operations ran before it — the same pattern the tutorial generators use.
fn block_id(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == name => Some(block.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture block {name:?} exists"))
}

/// The frame and one of its columns, both by name — ids re-mint every run.
fn frame_column_id(store: &Store, frame_name: &str, column_name: &str) -> (String, String) {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == frame_name => Some((
                frame.id.clone(),
                frame
                    .columns
                    .iter()
                    .find(|column| column.name == column_name)
                    .unwrap_or_else(|| panic!("fixture column {column_name:?} exists"))
                    .id
                    .clone(),
            )),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture frame {frame_name:?} exists"))
}

/// Writes `store`'s current `DocumentView` as pretty-printed JSON: the same
/// struct, serialized the same way, that lands in the frontend on every
/// IPC round trip.
fn write_view(store: &Store, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&store.view())?;
    std::fs::write(path, json)?;
    Ok(())
}

/// The "Monthly sales" grid from `generate_formula_click_tutorial.rs`,
/// reused verbatim. Sharing the exact numbers means a fixture and its
/// tutorial counterpart describe the same starting workbook if someone
/// ever needs to compare them.
fn monthly_sales_grid() -> Vec<Vec<String>> {
    vec![
        vec!["Month", "Region", "Revenue", "Cost"],
        vec!["2026-04", "West", "142000", "91000"],
        vec!["2026-01", "East", "118000", "76000"],
        vec!["2026-06", "East", "168000", "104000"],
        vec!["2026-03", "East", "136000", "85000"],
        vec!["2026-02", "West", "124000", "79000"],
        vec!["2026-05", "East", "151000", "96000"],
    ]
    .into_iter()
    .map(|row| row.into_iter().map(str::to_string).collect())
    .collect()
}

const README: &str = "\
# UI fixtures

Generated `DocumentView` JSON — the exact struct the frontend receives over
Tauri IPC, camelCase fields and all. Never hand-edit these files; regenerate
them with:

    cargo run -p framework-core --example generate_ui_fixtures

Every id in these files (document, frame, column, block, view) is a UUID or
a `column_id` random suffix minted fresh each time the example runs, so ids
are not stable across regeneration. Tests must select fixture data by name
— a frame's `name`, a block line's `name` — never by id.

- `blank.json` — `Document::blank(\"Fixture\")`, a brand-new workbook with
  nothing on the canvas.
- `sales-before-formula.json` — a \"Monthly sales\" frame plus an empty
  \"Checks\" block, before any formula has been written.
- `sales-with-formula.json` — the same document after `SetBlockSource`
  writes a formula into Checks, so `computedBlocks` carries a real computed
  answer rather than an empty line.
- `sales-with-margin.json` — the same document again after
  `SetFramePipeline` adds a calculated Margin column, so the frame holds a
  literal column beside a chain-calculated one and `editing` reports the
  per-frame answer the grid gates edits on.
- `sales-margin-delete-region.json` — one more `SetFramePipeline` on the
  same store, whose Select step drops Region. Generated from the same
  document so every id matches `sales-with-margin.json`: a test can render
  one view and re-render with the other to stand in for an undo or a chain
  edit arriving from outside the component.
- `imported-sales.json` — a separate document holding a linked import: an
  artifact-paged frame with a file connector, which the engine reports as
  not cell-editable (`editing.cells` false, with the reason text). The
  parquet it points at is `imported-sales.parquet` beside these fixtures,
  written by this generator; the artifact path is stored relative to the
  workspace root, which is where this example sets its working directory.
";

/// `sales-with-margin.json` and `sales-margin-delete-region.json`: two more
/// chain saves on the caller's store. The frame first gains a calculated
/// Margin column beside its literal inputs — the shape the grid's
/// per-column edit gate has to answer for: typing lands in Revenue, never
/// in Margin. Then a Select drops Region. Both are written from the same
/// document deliberately, so every id matches across the pair — a test can
/// render one view and re-render with the other to play an undo, or a
/// chain edit landing from outside the component, without ids re-minting
/// under it.
fn write_chain_fixtures(
    store: &mut Store,
    output: &Path,
) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let (sales_frame_id, revenue_id) = frame_column_id(store, "Monthly sales", "Revenue");
    let (_, cost_id) = frame_column_id(store, "Monthly sales", "Cost");
    let margin_step = FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: "margin~fixture".into(),
            name: "Margin".into(),
            formula: "`Revenue` - `Cost`".into(),
        }],
    };
    store.apply(Operation::SetFramePipeline {
        frame_id: sales_frame_id.clone(),
        steps: vec![margin_step.clone()],
    })?;
    let margin_path = output.join("sales-with-margin.json");
    write_view(store, &margin_path)?;

    let (_, month_id) = frame_column_id(store, "Monthly sales", "Month");
    store.apply(Operation::SetFramePipeline {
        frame_id: sales_frame_id,
        steps: vec![
            margin_step,
            FrameStepInput::Select {
                column_ids: vec![month_id, revenue_id, cost_id, "margin~fixture".into()],
            },
        ],
    })?;
    let delete_region_path = output.join("sales-margin-delete-region.json");
    write_view(store, &delete_region_path)?;
    Ok((margin_path, delete_region_path))
}

/// `imported-sales.json`: a linked import — an artifact-paged frame with a
/// file connector, which the engine reports as not cell-editable, with the
/// reason text. The fixture for "typing here must not open an editor". The
/// parquet is real because preparing the import reads its schema; it is
/// written beside the fixtures and addressed relative to the workspace
/// root so the committed JSON carries no machine-specific path.
fn write_imported_fixture(
    workspace: &Path,
    output: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let parquet_relative = "src/test/fixtures/imported-sales.parquet";
    let mut source_frame = pl::df! {
        "Month" => ["2026-04", "2026-01", "2026-06", "2026-03", "2026-02", "2026-05"],
        "Region" => ["West", "East", "East", "East", "West", "East"],
        "Revenue" => [142_000i64, 118_000, 168_000, 136_000, 124_000, 151_000],
        "Cost" => [91_000i64, 76_000, 104_000, 85_000, 79_000, 96_000],
    }?;
    let parquet_file = std::fs::File::create(workspace.join(parquet_relative))?;
    pl::ParquetWriter::new(parquet_file).finish(&mut source_frame)?;
    let mut imported_store = Store::new(Document::blank("Fixture"));
    imported_store.apply(Operation::ImportFrameFromArtifact {
        name: "Imported sales".into(),
        artifact: DataArtifact {
            id: "imported-sales-fixture".into(),
            path: parquet_relative.into(),
            // Corrected from the parquet itself while the import prepares.
            row_count: 0,
            format: ArtifactFormat::Parquet,
            source_name: "monthly-sales.csv".into(),
        },
        connector: Some(ConnectorRecipe::File {
            source_path: "monthly-sales.csv".into(),
        }),
        file_origin: None,
        x: 80.0,
        y: 80.0,
    })?;
    let imported_path = output.join("imported-sales.json");
    write_view(&imported_store, &imported_path)?;
    Ok(imported_path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    // The imported-frame fixture below stores its artifact path relative to
    // the workspace root, so the committed JSON is identical on every
    // machine. Running from the workspace root makes that relative path
    // resolvable while the import is prepared (the engine opens the parquet
    // to read its schema); pinning the working directory here means the
    // command works the same however cargo was invoked.
    std::env::set_current_dir(&workspace)?;
    let output = workspace.join("src/test/fixtures");
    std::fs::create_dir_all(&output)?;
    if std::env::args().any(|arg| arg == "--sensitivity") { return sensitivity::write(&output); }
    if std::env::args().any(|arg| arg == "--parameters") { return parameters::write(&output); }
    parameters::write(&output)?;
    sensitivity::write(&output)?;

    // 1. A blank document: the state a brand-new workbook opens into, and
    // the simplest possible `DocumentView` a test can assert against.
    let blank_store = Store::new(Document::blank("Fixture"));
    let blank_path = output.join("blank.json");
    write_view(&blank_store, &blank_path)?;

    // 2. The same "Monthly sales" frame and empty "Checks" block the
    // formula click tutorial starts from, before any formula exists to
    // compute an answer. Frontend tests that assert on an empty Checks
    // block, or that drive typing a formula into it themselves, start here.
    let mut store = Store::new(Document::blank("Fixture"));
    store.apply(Operation::AddFrame {
        name: "Monthly sales".into(),
        grid: monthly_sales_grid(),
        x: 80.0,
        y: 80.0,
    })?;
    store.apply(Operation::AddBlock {
        name: "Checks".into(),
        x: 80.0,
        y: 430.0,
    })?;
    let before_path = output.join("sales-before-formula.json");
    write_view(&store, &before_path)?;

    // 3. The same document after `SetBlockSource` writes a formula into
    // Checks, so `computedBlocks` in the view carries a real computed
    // answer. Frontend tests that assert on a rendered answer start here
    // instead of replaying step 2's typing themselves.
    store.apply(Operation::SetBlockSource {
        block_id: block_id(&store, "Checks"),
        source: "Total revenue = `Monthly sales`.`Revenue`.sum()".into(),
        editing: None,
    })?;
    let after_path = output.join("sales-with-formula.json");
    write_view(&store, &after_path)?;

    // 4 and 5. Chain fixtures on the same store, so ids line up across the
    // pair; see `write_chain_fixtures`.
    let (margin_path, delete_region_path) = write_chain_fixtures(&mut store, &output)?;

    // 6. A linked import; see `write_imported_fixture`.
    let imported_path = write_imported_fixture(&workspace, &output)?;

    std::fs::write(output.join("README.md"), README)?;

    // Loading each file back through serde_json — as the exact `DocumentView`
    // type, not just a generic JSON value — catches malformed output and
    // schema drift the same way a frontend fixture import would.
    for path in [
        &blank_path,
        &before_path,
        &after_path,
        &margin_path,
        &delete_region_path,
        &imported_path,
    ] {
        let text = std::fs::read_to_string(path)?;
        let _: DocumentView = serde_json::from_str(&text)?;
        println!("wrote {}", path.display());
    }
    println!("wrote {}", output.join("README.md").display());
    Ok(())
}
