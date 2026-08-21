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

use framework_core::{DataObject, Document, DocumentView, Operation, Store};
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
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let output = workspace.join("src/test/fixtures");
    std::fs::create_dir_all(&output)?;

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

    std::fs::write(output.join("README.md"), README)?;

    // Loading each file back through serde_json — as the exact `DocumentView`
    // type, not just a generic JSON value — catches malformed output and
    // schema drift the same way a frontend fixture import would.
    for path in [&blank_path, &before_path, &after_path] {
        let text = std::fs::read_to_string(path)?;
        let _: DocumentView = serde_json::from_str(&text)?;
        println!("wrote {}", path.display());
    }
    println!("wrote {}", output.join("README.md").display());
    Ok(())
}
