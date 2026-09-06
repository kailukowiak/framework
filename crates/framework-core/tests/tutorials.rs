use framework_core::{
    ComputedTextSegment, DataObject, DataType, ExistingFormulaInput, FRAMEWORK_TUTORIAL_VERSION,
    FrameObject, FrameStepInput, Operation, Store, inspect_excel_workbook,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const BUNDLED_TUTORIALS: [[&str; 2]; 12] = [
    ["grand-tour", "grand-tour-start.fw"],
    ["grand-tour", "grand-tour-finished.fw"],
    ["first-workbook", "first-workbook-start.fw"],
    ["first-workbook", "first-workbook-finished.fw"],
    ["excel-import", "excel-import-start.fw"],
    ["excel-import", "excel-import-finished.fw"],
    ["formula-clicks", "formula-clicks-start.fw"],
    ["formula-clicks", "formula-clicks-finished.fw"],
    ["month-end-close", "month-end-close-start.fw"],
    ["month-end-close", "month-end-close-finished.fw"],
    ["vectors-and-joins", "vectors-and-joins-start.fw"],
    ["vectors-and-joins", "vectors-and-joins-finished.fw"],
];

fn tutorial_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    path.push("tutorials");
    for part in parts {
        path.push(part);
    }
    path
}

fn frame_named<'a>(store: &'a Store, name: &str) -> &'a FrameObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == name => Some(frame),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial frame {name:?} exists"))
}

fn block_answers(store: &Store, name: &str) -> Vec<String> {
    let id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == name => Some(block.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial block {name:?} exists"));
    store.view().computed_blocks[&id]
        .lines
        .iter()
        .map(|line| line.cell.display.clone())
        .collect()
}

fn add_seventh_launch_row(store: &mut Store, source: &FrameObject) {
    let mut values = BTreeMap::new();
    for (name, value) in [("Line", "7"), ("SKU", "C-300"), ("Units", "50")] {
        let column = source
            .columns
            .iter()
            .find(|column| column.name == name)
            .unwrap_or_else(|| panic!("launch input column {name:?} exists"));
        values.insert(column.id.clone(), value.into());
    }
    store
        .apply(Operation::AddRow {
            frame_id: source.id.clone(),
            values,
        })
        .unwrap();
}

fn tutorial_computation_errors(store: &Store) -> Vec<String> {
    let view = store.view();
    let mut errors = Vec::new();
    for object in &view.document.objects {
        let name = object.name();
        match object {
            DataObject::Result(result) => {
                if let Some(error) = &view.computed_results[&result.id].cell.error {
                    errors.push(format!("result {name:?}: {error}"));
                }
            }
            DataObject::Block(block) => {
                for line in &view.computed_blocks[&block.id].lines {
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
                for error in computed.style_rule_errors.values() {
                    errors.push(format!("frame {name:?}, conditional format: {error}"));
                }
                for cell in computed.summaries.values() {
                    if let Some(error) = &cell.error {
                        errors.push(format!("frame {name:?}, summary: {error}"));
                    }
                }
                if let Err(error) = store.get_frame_page(&frame.id, 0, 1_000) {
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
            DataObject::Value(_)
            | DataObject::Series(_)
            | DataObject::Container(_)
            | DataObject::Plot(_) => {}
        }
    }
    errors
}

#[test]
fn every_bundled_tutorial_declares_the_current_tutorial_version() {
    for parts in BUNDLED_TUTORIALS {
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
        let walkthrough = store
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Text(text) if text.name == "Tutorial walkthrough" => {
                    Some(text.id.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{} has no rendered walkthrough", path.display()));
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
fn every_bundled_tutorial_loads_and_computes_without_errors() {
    for parts in BUNDLED_TUTORIALS {
        let path = tutorial_path(&parts);
        let store = Store::load(&path).unwrap();
        let errors = tutorial_computation_errors(&store);
        assert!(
            errors.is_empty(),
            "{} contains computation errors:\n{}",
            path.display(),
            errors.join("\n")
        );
    }
}

#[test]
fn excel_import_tutorial_ships_sources_and_a_real_imported_answer() {
    let start = Store::load(&tutorial_path(&["excel-import", "excel-import-start.fw"])).unwrap();
    assert_eq!(start.document().objects.len(), 1);
    assert!(matches!(start.document().objects[0], DataObject::Text(_)));

    let finished = Store::load(&tutorial_path(&[
        "excel-import",
        "excel-import-finished.fw",
    ]))
    .unwrap();
    for (name, rows) in [
        ("Customers", 7),
        ("Inventory", 6),
        ("Suppliers", 4),
        ("Orders", 20),
        ("Adjustments", 8),
        ("Targets", 10),
    ] {
        let frame = frame_named(&finished, name);
        assert!(frame.artifact.is_some(), "{name} is an imported artifact");
        assert_eq!(
            finished
                .get_frame_page(&frame.id, 0, 50)
                .unwrap()
                .total_rows,
            rows
        );
    }

    let simple = inspect_excel_workbook(&tutorial_path(&[
        "excel-import",
        "source",
        "simple-customers.xlsx",
    ]))
    .unwrap();
    assert_eq!(simple.sheets.len(), 1);
    assert_eq!(simple.tables.len(), 1);
    let complex = inspect_excel_workbook(&tutorial_path(&[
        "excel-import",
        "source",
        "multi-table-operations.xlsx",
    ]))
    .unwrap();
    assert_eq!(complex.sheets.len(), 2);
    assert_eq!(complex.tables.len(), 3);
    assert_eq!(
        complex
            .suggested_regions
            .iter()
            .map(|region| (region.sheet_name.as_str(), region.cell_range.as_str()))
            .collect::<Vec<_>>(),
        [("Operations", "A15:D23"), ("Sales", "P15:S25")]
    );
}

#[test]
fn first_workbook_tutorial_keeps_a_rebuildable_start_and_checked_answer() {
    let start = Store::load(&tutorial_path(&[
        "first-workbook",
        "first-workbook-start.fw",
    ]))
    .unwrap();
    let empty = frame_named(&start, "Monthly sales");
    assert_eq!(empty.columns.len(), 2);
    assert!(empty.rows.iter().all(|row| {
        row.cells
            .values()
            .all(|cell| cell.raw.is_empty() && cell.override_formula.is_none())
    }));

    let finished = Store::load(&tutorial_path(&[
        "first-workbook",
        "first-workbook-finished.fw",
    ]))
    .unwrap();
    let sales = frame_named(&finished, "Monthly sales");
    let page = finished.get_frame_page(&sales.id, 0, 20).unwrap();
    assert_eq!(page.total_rows, 6);
    assert_eq!(
        page.columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Month", "Region", "Revenue", "Cost", "Profit"]
    );
    assert_eq!(
        page.rows[0],
        vec!["2026-01", "East", "118000", "76000", "42000"]
    );
    assert_eq!(
        finished
            .get_frame_page(&frame_named(&finished, "East only").id, 0, 20)
            .unwrap()
            .total_rows,
        4
    );
    assert!(
        finished.document().objects.iter().any(
            |object| matches!(object, DataObject::Plot(plot) if plot.name == "Profit by month")
        )
    );
    let narrative = finished
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Text(text) if text.name == "Sales narrative" => Some(text.id.clone()),
            _ => None,
        })
        .expect("finished tutorial includes the live narrative");
    assert_eq!(
        finished.view().computed_texts[&narrative]
            .segments
            .iter()
            .filter_map(|segment| match segment {
                ComputedTextSegment::Value { cell, .. } => Some(cell.display.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["839000", "308000"]
    );
}

#[test]
fn formula_clicks_tutorial_still_has_the_intermediate_answer_key() {
    let start = Store::load(&tutorial_path(&[
        "formula-clicks",
        "formula-clicks-start.fw",
    ]))
    .unwrap();
    let starting_sales = frame_named(&start, "Monthly sales");
    assert!(starting_sales.steps.is_empty());
    assert_eq!(
        starting_sales
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Month", "Region", "Revenue", "Cost"]
    );
    assert_eq!(
        start
            .get_frame_page(&starting_sales.id, 0, 20)
            .unwrap()
            .total_rows,
        6
    );

    let finished = Store::load(&tutorial_path(&[
        "formula-clicks",
        "formula-clicks-finished.fw",
    ]))
    .unwrap();
    let sales = frame_named(&finished, "Monthly sales");
    let page = finished.get_frame_page(&sales.id, 0, 20).unwrap();
    assert_eq!(page.total_rows, 6);
    assert_eq!(page.rows[1][4], "118000");
    assert_eq!(page.rows[1][5], "6000");
    assert!(finished.document().frozen_values.is_empty());
    assert_eq!(block_answers(&finished, "Checks"), ["839000", "168000"]);
}

#[test]
fn formula_clicks_start_can_calculate_an_existing_blank_column() {
    let mut store = Store::load(&tutorial_path(&[
        "formula-clicks",
        "formula-clicks-start.fw",
    ]))
    .unwrap();
    let sales = frame_named(&store, "Monthly sales").clone();
    store
        .apply(Operation::AddColumn {
            frame_id: sales.id.clone(),
            name: "New column".into(),
            data_type: DataType::String,
            after_column_id: sales.columns.last().map(|column| column.id.clone()),
        })
        .unwrap();
    let calculated_id = frame_named(&store, "Monthly sales")
        .columns
        .last()
        .unwrap()
        .id
        .clone();

    store
        .apply(Operation::SetFramePipeline {
            frame_id: sales.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: calculated_id,
                    name: "New column".into(),
                    formula: "`Revenue` - `Cost`".into(),
                }],
            }],
        })
        .unwrap();

    let page = store.get_frame_page(&sales.id, 0, 20).unwrap();
    assert_eq!(page.total_rows, 6);
    // The calculated column takes the type its formula produces, not the
    // String the blank column was declared as. Which type that is follows
    // its operands: this workbook's Revenue and Cost are money, and money
    // minus money is money -- the notation-propagation rule, which is why
    // this asserts against Revenue's own type rather than naming one. A
    // regenerated workbook that types those columns differently should move
    // this answer with them rather than fail.
    let revenue = page
        .columns
        .iter()
        .find(|column| column.name == "Revenue")
        .unwrap()
        .data_type;
    assert_eq!(page.columns.last().unwrap().data_type, revenue);
    assert_ne!(page.columns.last().unwrap().data_type, DataType::String);
    assert_eq!(page.rows[0].last().unwrap(), "51000");
}

#[test]
fn month_end_close_tutorial_reconciles_every_output() {
    let start = Store::load(&tutorial_path(&[
        "month-end-close",
        "month-end-close-start.fw",
    ]))
    .unwrap();
    assert_eq!(frame_named(&start, "Actuals").rows.len(), 12);
    assert_eq!(frame_named(&start, "Budget").rows.len(), 12);

    let finished = Store::load(&tutorial_path(&[
        "month-end-close",
        "month-end-close-finished.fw",
    ]))
    .unwrap();
    let analysis = frame_named(&finished, "Actuals vs budget");
    let analysis_page = finished.get_frame_page(&analysis.id, 0, 30).unwrap();
    assert_eq!(analysis_page.total_rows, 12);
    assert_eq!(
        analysis_page.rows[0],
        vec![
            "2026-01",
            "East",
            "118000",
            "76000",
            "120000",
            "42000",
            "-2000",
            "-0.016666666666666666"
        ]
    );

    let summary = frame_named(&finished, "Regional summary");
    let summary_page = finished.get_frame_page(&summary.id, 0, 10).unwrap();
    assert_eq!(summary_page.total_rows, 2);
    assert_eq!(
        summary_page.rows[0],
        vec!["East", "843000", "822000", "21000", "310000"]
    );
    assert_eq!(
        summary_page.rows[1],
        vec!["West", "808000", "793000", "15000", "290000"]
    );

    let pivot = frame_named(&finished, "Revenue by month");
    let pivot_page = finished.get_frame_page(&pivot.id, 0, 10).unwrap();
    assert_eq!(pivot_page.total_rows, 2);
    assert_eq!(pivot_page.columns.len(), 7);

    let exceptions = frame_named(&finished, "Below budget");
    assert_eq!(
        finished
            .get_frame_page(&exceptions.id, 0, 10)
            .unwrap()
            .total_rows,
        2
    );
    assert!(finished.document().frozen_values.is_empty());
    assert_eq!(
        block_answers(&finished, "Close checks"),
        ["1651000", "1615000"]
    );
}

#[test]
fn vectors_and_joins_tutorial_grows_dates_and_lookup_results_from_its_source() {
    let start = Store::load(&tutorial_path(&[
        "vectors-and-joins",
        "vectors-and-joins-start.fw",
    ]))
    .unwrap();
    let walkthrough = start
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Text(text) if text.name == "Tutorial walkthrough" => Some(text.id.clone()),
            _ => None,
        })
        .expect("the tutorial renders its walkthrough inside the workbook");
    let rendered_walkthrough = &start.view().computed_texts[&walkthrough];
    assert!(
        rendered_walkthrough
            .source
            .contains("## 1. Continue the date pattern")
    );
    assert!(
        rendered_walkthrough
            .source
            .contains("## 3. Build a Calculation Matrix")
    );
    assert!(
        rendered_walkthrough
            .source
            .contains("## 4. Bring catalog columns over")
    );
    assert_eq!(frame_named(&start, "Launch inputs").rows.len(), 6);
    assert!(
        frame_named(&start, "Launch plan")
            .derivation
            .as_ref()
            .is_some_and(|derivation| derivation.join.is_none())
    );

    let mut finished = Store::load(&tutorial_path(&[
        "vectors-and-joins",
        "vectors-and-joins-finished.fw",
    ]))
    .unwrap();
    let source = frame_named(&finished, "Launch inputs").clone();
    let launch = frame_named(&finished, "Launch plan").clone();
    let scheduled = frame_named(&finished, "Scheduled launches").clone();
    let catalog = frame_named(&finished, "Product catalog");
    let matrix = finished
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::CalculationMatrix(matrix) if matrix.name == "Scenario × Quarter" => {
                Some(matrix)
            }
            _ => None,
        })
        .expect("the finished tutorial contains its Calculation Matrix");
    assert_eq!(matrix.rows.len(), 2);
    assert_eq!(matrix.rows[0].name, "Scenario");
    assert_eq!(matrix.rows[0].source, "`Scenario`");
    assert_eq!(matrix.rows[1].name, "Multiplier");
    assert_eq!(matrix.rows[1].source, "`Multiplier`");
    assert_eq!(matrix.columns.len(), 2);
    assert_eq!(matrix.columns[0].name, "Quarter");
    assert_eq!(matrix.columns[0].source, "`Quarter`");
    assert_eq!(matrix.columns[1].name, "Base revenue");
    assert_eq!(matrix.columns[1].source, "`Base revenue`");
    assert_eq!(matrix.body.source, "`Base revenue` * `Multiplier`");
    assert!(matrix.body.error.is_none());
    let matrix_id = matrix.id.clone();
    let view = finished.view();
    let computed_matrix = &view.computed_calculation_matrices[&matrix_id];
    assert_eq!(computed_matrix.row_tuples.len(), 3);
    assert_eq!(computed_matrix.column_tuples.len(), 4);
    assert_eq!(computed_matrix.row_tuples[0].values, ["Base", "1"]);
    assert_eq!(computed_matrix.column_tuples[0].values, ["Q1", "100"]);
    assert_eq!(computed_matrix.cells.len(), 3);
    assert!(computed_matrix.cells.iter().all(|row| row.len() == 4));
    assert_eq!(computed_matrix.cells[0][0].display, "100.00");
    assert_eq!(computed_matrix.cells[1][3].display, "149.50");
    assert_eq!(computed_matrix.cells[2][1].display, "93.50");
    assert_eq!(computed_matrix.output.as_ref().unwrap().rows.len(), 12);
    for (name, count) in [
        ("Scenario", 3),
        ("Multiplier", 3),
        ("Quarter", 4),
        ("Base revenue", 4),
    ] {
        let id = finished
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Result(result) if result.variable && result.name == name => {
                    Some(result.id.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the finished tutorial contains the {name} variable"));
        assert_eq!(view.computed_results[&id].value_count, count);
    }
    assert_eq!(catalog.unique_keys.len(), 1);

    let launch_page = finished.get_frame_page(&launch.id, 0, 20).unwrap();
    assert_eq!(launch_page.total_rows, 6);
    assert_eq!(launch_page.rows[0][3], "2026-09-01");
    assert_eq!(launch_page.rows[5][3], "2027-02-01");
    assert_eq!(
        finished.get_frame_page(&scheduled.id, 0, 20).unwrap().rows[0][7],
        "30000"
    );
    assert_eq!(block_answers(&finished, "Checks"), ["6", "137600"]);

    add_seventh_launch_row(&mut finished, &source);

    let grown_launch = finished.get_frame_page(&launch.id, 0, 20).unwrap();
    assert_eq!(grown_launch.total_rows, 7);
    assert_eq!(grown_launch.rows[6][3], "2027-03-01");
    let grown_join = finished.get_frame_page(&scheduled.id, 0, 20).unwrap();
    assert_eq!(grown_join.total_rows, 7);
    assert_eq!(grown_join.rows[6][4], "Cedar");
    assert_eq!(grown_join.rows[6][7], "16000");
    assert_eq!(block_answers(&finished, "Checks"), ["7", "153600"]);
}

#[test]
fn grand_tour_switches_assumptions_without_copying_the_model() {
    let start = Store::load(&tutorial_path(&["grand-tour", "grand-tour-start.fw"])).unwrap();
    let empty = frame_named(&start, "Monthly sales");
    assert_eq!(empty.columns.len(), 2);
    assert!(empty.rows.iter().all(|row| {
        row.cells
            .values()
            .all(|cell| cell.raw.is_empty() && cell.override_formula.is_none())
    }));
    assert_eq!(frame_named(&start, "Budget").rows.len(), 6);
    // Section 5 asks the reader to press ⌘J, which is the gesture that
    // creates the block. Shipping one in the Start workbook would quietly
    // replace the thing the section is teaching.
    assert!(
        !start
            .document()
            .objects
            .iter()
            .any(|object| matches!(object, DataObject::Block(_))),
        "the tour's Start workbook leaves Scratchwork to ⌘J"
    );

    let mut finished =
        Store::load(&tutorial_path(&["grand-tour", "grand-tour-finished.fw"])).unwrap();
    let sales = frame_named(&finished, "Monthly sales");
    let sales_page = finished.get_frame_page(&sales.id, 0, 20).unwrap();
    assert_eq!(sales_page.total_rows, 6);
    assert_eq!(
        sales_page
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Month", "Region", "Revenue", "Cost", "Profit", "Forecast"]
    );
    assert_eq!(
        sales_page.rows[0],
        vec!["2026-01", "East", "118000", "76000", "42000", "127440"]
    );

    let analysis = frame_named(&finished, "Sales vs budget");
    let analysis_page = finished.get_frame_page(&analysis.id, 0, 20).unwrap();
    assert_eq!(analysis_page.total_rows, 6);
    assert_eq!(
        analysis_page.rows[0],
        vec![
            "2026-01", "East", "118000", "76000", "42000", "120000", "-2000"
        ]
    );
    assert_eq!(analysis_page.rows[5].last().unwrap(), "8000");
    assert_eq!(frame_named(&finished, "Budget").unique_keys.len(), 1);

    let summary = frame_named(&finished, "By region");
    let summary_page = finished.get_frame_page(&summary.id, 0, 10).unwrap();
    assert_eq!(
        summary_page.rows,
        vec![
            vec!["East", "573000", "560000", "13000"],
            vec!["West", "266000", "258000", "8000"],
        ]
    );

    assert!(
        finished.document().objects.iter().any(
            |object| matches!(object, DataObject::Plot(plot) if plot.name == "Revenue by month")
        )
    );
    assert!(finished.document().frozen_values.is_empty());
    assert_eq!(
        block_answers(&finished, "Scratchwork"),
        ["839000", "21000", "906120.00"]
    );

    // The tour ends on the claim that one model answers three questions.
    // Switching is therefore behavior to assert, not a stored formula: the
    // same three lines are read under each set of assumptions and again
    // after switching back.
    for (name, forecast) in [("Upside", "964850.00"), ("Downside", "797050.00")] {
        let scenario_id = finished
            .document()
            .scenarios
            .iter()
            .find(|scenario| scenario.name == name)
            .unwrap_or_else(|| panic!("the answer key carries the {name} scenario"))
            .id
            .clone();
        finished
            .apply(Operation::ActivateScenario {
                scenario_id: Some(scenario_id),
            })
            .unwrap();
        assert_eq!(
            block_answers(&finished, "Scratchwork"),
            ["839000", "21000", forecast]
        );
    }
    finished
        .apply(Operation::ActivateScenario { scenario_id: None })
        .unwrap();
    assert_eq!(
        block_answers(&finished, "Scratchwork"),
        ["839000", "21000", "906120.00"]
    );
}
