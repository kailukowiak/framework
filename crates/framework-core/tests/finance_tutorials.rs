//! The three finance lessons under `tutorials/`. They live in their own
//! module rather than `tutorials.rs` so the finance branch does not edit a
//! file the ML batch is changing; the checks are the same two the bundled
//! tutorials get, plus one checkpoint per answer key so a regenerated
//! workbook cannot quietly drift from the README beside it.

use framework_core::{
    ComputedTextSegment, DataObject, FRAMEWORK_TUTORIAL_VERSION, Operation, Store,
};
use std::fs;
use std::path::PathBuf;

const FINANCE_TUTORIALS: [[&str; 2]; 6] = [
    ["price-a-deal", "price-a-deal-start.fw"],
    ["price-a-deal", "price-a-deal-finished.fw"],
    ["driver-forecast", "driver-forecast-start.fw"],
    ["driver-forecast", "driver-forecast-finished.fw"],
    [
        "scenarios-and-sensitivity",
        "scenarios-and-sensitivity-start.fw",
    ],
    [
        "scenarios-and-sensitivity",
        "scenarios-and-sensitivity-finished.fw",
    ],
];

fn tutorial_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    path.push("tutorials");
    for part in parts {
        path.push(part);
    }
    path
}

fn object_id(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap_or_else(|| panic!("tutorial object {name:?} exists"))
        .id()
        .to_string()
}

/// A block's answers by line name. Comment lines carry no answer and are
/// skipped, the way the canvas skips them.
fn block_answer(store: &Store, block: &str, line_name: &str) -> f64 {
    let id = object_id(store, block);
    let view = store.view();
    let line = view.computed_blocks[&id]
        .lines
        .iter()
        .find(|line| line.name == line_name)
        .unwrap_or_else(|| panic!("block {block:?} has a line named {line_name:?}"));
    assert!(
        line.cell.error.is_none(),
        "{block}.{line_name} errs: {:?}",
        line.cell.error
    );
    line.cell
        .display
        .replace(['$', ','], "")
        .parse()
        .unwrap_or_else(|_| panic!("{block}.{line_name} shows {:?}", line.cell.display))
}

fn cell(store: &Store, frame: &str, first_cell: &str, column: &str) -> f64 {
    let id = object_id(store, frame);
    let page = store.get_frame_page(&id, 0, 200).unwrap();
    let index = page
        .columns
        .iter()
        .position(|c| c.name == column)
        .unwrap_or_else(|| panic!("{frame} has a column {column:?}"));
    let row = page
        .rows
        .iter()
        .find(|row| row[0] == first_cell)
        .unwrap_or_else(|| panic!("{frame} has a row starting {first_cell:?}"));
    row[index]
        .parse()
        .unwrap_or_else(|_| panic!("{frame} {first_cell} {column} shows {:?}", row[index]))
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= (expected.abs() * 1e-6).max(0.011),
        "{what}: got {actual}, expected {expected}"
    );
}

fn computation_errors(store: &Store) -> Vec<String> {
    let view = store.view();
    let mut errors = Vec::new();
    for object in &view.document.objects {
        let name = object.name();
        match object {
            DataObject::Block(block) => {
                for line in &view.computed_blocks[&block.id].lines {
                    if line.text.trim_start().starts_with('#') {
                        continue;
                    }
                    if let Some(error) = &line.cell.error {
                        errors.push(format!("block {name:?}, line {:?}: {error}", line.text));
                    }
                }
            }
            DataObject::Text(text) => {
                for segment in &view.computed_texts[&text.id].segments {
                    match segment {
                        ComputedTextSegment::Value { cell, formula, .. } => {
                            if let Some(error) = &cell.error {
                                errors.push(format!("text {name:?}, formula {formula:?}: {error}"));
                            }
                        }
                        ComputedTextSegment::Broken { source, error } => {
                            errors.push(format!("text {name:?}, formula {source:?}: {error}"));
                        }
                        ComputedTextSegment::Literal { .. } => {}
                    }
                }
            }
            DataObject::Frame(frame) => {
                let computed = &view.computed_frames[&frame.id];
                for cell in computed.summaries.values() {
                    if let Some(error) = &cell.error {
                        errors.push(format!("frame {name:?}, summary: {error}"));
                    }
                }
                if let Err(error) = store.get_frame_page(&frame.id, 0, 200) {
                    errors.push(format!("frame {name:?} could not produce a page: {error}"));
                }
            }
            DataObject::CalculationMatrix(matrix) => {
                let computed = &view.computed_calculation_matrices[&matrix.id];
                if let Some(error) = &computed.error {
                    errors.push(format!("calculation matrix {name:?}: {error}"));
                }
                for cell in computed.cells.iter().flatten() {
                    if let Some(error) = &cell.error {
                        errors.push(format!("calculation matrix {name:?}: {error}"));
                    }
                }
            }
            _ => {}
        }
    }
    errors
}

#[test]
fn every_finance_tutorial_declares_the_current_version_and_renders_its_guide() {
    for parts in FINANCE_TUTORIALS {
        let path = tutorial_path(&parts);
        let serialized: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            serialized["tutorialVersion"],
            FRAMEWORK_TUTORIAL_VERSION,
            "{} has a stale tutorial version",
            path.display()
        );
        let store = Store::load(&path).unwrap();
        let walkthrough = object_id(&store, "Tutorial walkthrough");
        let rendered = &store.view().computed_texts[&walkthrough];
        assert!(
            rendered.source.contains("## 1."),
            "{} has no first step",
            path.display()
        );
        assert!(
            rendered
                .segments
                .iter()
                .all(|segment| !matches!(segment, ComputedTextSegment::Broken { .. })),
            "{} has a broken formula example in its walkthrough",
            path.display()
        );
    }
}

#[test]
fn every_finance_tutorial_loads_and_computes_without_errors() {
    for parts in FINANCE_TUTORIALS {
        let path = tutorial_path(&parts);
        let store = Store::load(&path).unwrap();
        let errors = computation_errors(&store);
        assert!(
            errors.is_empty(),
            "{} contains computation errors:\n{}",
            path.display(),
            errors.join("\n")
        );
    }
}

#[test]
fn price_a_deal_answer_key_prices_the_deal_without_a_solver() {
    let store = Store::load(&tutorial_path(&[
        "price-a-deal",
        "price-a-deal-finished.fw",
    ]))
    .unwrap();
    assert_close(
        block_answer(&store, "Loan terms", "payment"),
        7733.12,
        "payment",
    );
    assert_close(
        block_answer(&store, "Loan terms", "total interest"),
        63987.24,
        "total interest",
    );
    assert_close(
        cell(&store, "Schedule", "60", "Closing"),
        0.0,
        "final balance",
    );
    assert_close(block_answer(&store, "Deal", "npv"), 41581.08, "npv");
    // The scan brackets the IRR the reader is asked to eyeball.
    assert!(block_answer(&store, "Rate scan", "NPV at 20%") > 0.0);
    assert!(block_answer(&store, "Rate scan", "NPV at 25%") < 0.0);
}

#[test]
fn driver_forecast_answer_key_switches_from_actuals_to_drivers() {
    let store = Store::load(&tutorial_path(&[
        "driver-forecast",
        "driver-forecast-finished.fw",
    ]))
    .unwrap();
    assert_close(
        cell(&store, "Forecast", "2026-07-01", "Revenue"),
        146000.0,
        "last actual",
    );
    assert_close(
        cell(&store, "Forecast", "2026-08-01", "Revenue"),
        148920.0,
        "first forecast",
    );
    assert_close(
        cell(&store, "Forecast", "2027-01-01", "Revenue"),
        164419.71,
        "last forecast",
    );
    assert_close(
        cell(&store, "Forecast", "2026-01-01", "YTD"),
        1475000.0,
        "FY2026 total",
    );
    assert_close(
        cell(&store, "Forecast", "2026-02-01", "Fiscal year"),
        2027.0,
        "offset year",
    );
    assert_close(
        block_answer(&store, "Checks", "fy2027"),
        1706405.37,
        "fy2027",
    );
}

#[test]
fn scenarios_answer_key_moves_the_model_with_the_scenario() {
    let mut store = Store::load(&tutorial_path(&[
        "scenarios-and-sensitivity",
        "scenarios-and-sensitivity-finished.fw",
    ]))
    .unwrap();
    assert_close(
        block_answer(&store, "Model", "ebitda"),
        150000.0,
        "base ebitda",
    );
    let upside = store
        .document()
        .scenarios
        .iter()
        .find(|scenario| scenario.name == "Upside")
        .expect("the answer key carries Upside")
        .id
        .clone();
    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(upside),
        })
        .unwrap();
    assert_close(
        block_answer(&store, "Model", "ebitda"),
        272500.0,
        "upside ebitda",
    );
    store
        .apply(Operation::ActivateScenario { scenario_id: None })
        .unwrap();
    assert_close(
        block_answer(&store, "Model", "ebitda"),
        150000.0,
        "back on base",
    );
    let grid = object_id(&store, "EBITDA by price and units");
    let view = store.view();
    let centre = &view.computed_calculation_matrices[&grid].cells[2][2];
    assert_close(
        centre.display.replace(['$', ','], "").parse().unwrap(),
        150000.0,
        "grid centre equals the model",
    );
}
