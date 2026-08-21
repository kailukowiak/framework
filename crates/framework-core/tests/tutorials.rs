use framework_core::{
    ComputedTextSegment, DataObject, DataType, ExistingFormulaInput, FRAMEWORK_TUTORIAL_VERSION,
    FrameObject, FrameStepInput, Operation, Store, inspect_excel_workbook,
};
use std::fs;
use std::path::PathBuf;

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

#[test]
fn every_bundled_tutorial_declares_the_current_tutorial_version() {
    for parts in [
        ["first-workbook", "first-workbook-start.fw"],
        ["first-workbook", "first-workbook-finished.fw"],
        ["excel-import", "excel-import-start.fw"],
        ["excel-import", "excel-import-finished.fw"],
        ["formula-clicks", "formula-clicks-start.fw"],
        ["formula-clicks", "formula-clicks-finished.fw"],
        ["month-end-close", "month-end-close-start.fw"],
        ["month-end-close", "month-end-close-finished.fw"],
    ] {
        let path = tutorial_path(&parts);
        let serialized: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            serialized["tutorialVersion"],
            FRAMEWORK_TUTORIAL_VERSION,
            "{} has a stale tutorial version",
            path.display()
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
    let checks = finished
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == "Checks" => Some(block.id.clone()),
            _ => None,
        })
        .expect("finished tutorial keeps its Scratchwork checks");
    assert_eq!(
        finished
            .view()
            .computed_blocks
            .get(&checks)
            .unwrap()
            .lines
            .iter()
            .map(|line| line.cell.display.as_str())
            .collect::<Vec<_>>(),
        ["839000", "168000"]
    );
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
    let checks = finished
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == "Close checks" => Some(block.id.clone()),
            _ => None,
        })
        .expect("finished close tutorial keeps its live control block");
    assert_eq!(
        finished
            .view()
            .computed_blocks
            .get(&checks)
            .unwrap()
            .lines
            .iter()
            .map(|line| line.cell.display.as_str())
            .collect::<Vec<_>>(),
        ["1651000", "1615000"]
    );
}
