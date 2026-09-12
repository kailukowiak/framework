//! The three finance lessons: each builds a real job with today's catalog and
//! asserts the checkpoints its README promises, so the workbook a reader
//! opens and the guide beside it cannot drift apart. The "after" half of each
//! README is the acceptance target for a phase of `docs/finance-build-plan.md`
//! and is deliberately not generated here until that phase lands.
//!
//! Expected numbers were computed independently in Python before these
//! workbooks existed; a passing run is evidence, not a tautology. Asserts
//! compare numerically: a page hands back raw values and a block its display
//! text, and neither spelling is the lesson's claim.

// Each lesson is one flat sequence of the gestures its README lists, in the
// README's order; splitting it by line count would hide that correspondence.
#![allow(clippy::too_many_lines)]

use framework_core::{
    ColumnFormat, ColumnFormatScale, ColumnFormatStyle, DataObject, DataType, Document,
    ExistingFormulaInput, FrameJoinType, FrameStepInput, JoinColumnInput, Operation, SortInput,
    Store, column_id,
};
use std::path::{Path, PathBuf};

fn frame(store: &Store, name: &str) -> framework_core::FrameObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == name => Some(frame.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial frame {name:?} exists"))
}

fn object_id_named(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap_or_else(|| panic!("tutorial object {name:?} exists"))
        .id()
        .to_string()
}

fn column_id_named(frame: &framework_core::FrameObject, name: &str) -> String {
    frame
        .columns
        .iter()
        .find(|column| column.name == name)
        .unwrap_or_else(|| panic!("tutorial column {name:?} exists in {:?}", frame.name))
        .id
        .clone()
}

fn view_id(store: &Store, object_id: &str) -> String {
    store
        .document()
        .views
        .iter()
        .find(|view| {
            view.object_id == object_id || view.tab_object_ids.iter().any(|id| id == object_id)
        })
        .expect("tutorial object has a view")
        .id
        .clone()
}

fn view_height(store: &Store, object_id: &str) -> f64 {
    store
        .document()
        .views
        .iter()
        .find(|view| {
            view.object_id == object_id || view.tab_object_ids.iter().any(|id| id == object_id)
        })
        .expect("tutorial object has a view")
        .height
}

fn money_format() -> ColumnFormat {
    ColumnFormat {
        style: ColumnFormatStyle::Accounting,
        decimals: Some(0),
        scale: ColumnFormatScale::Units,
        negative_parens: Some(true),
        zero_dash: Some(true),
        currency_code: Some("USD".into()),
        date_pattern: None,
    }
}

fn percent_format() -> ColumnFormat {
    ColumnFormat {
        style: ColumnFormatStyle::Percent,
        decimals: Some(1),
        scale: ColumnFormatScale::Units,
        negative_parens: Some(true),
        zero_dash: None,
        currency_code: None,
        date_pattern: None,
    }
}

fn grid(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect()
}

fn add_walkthrough(store: &mut Store, source: &str) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddText { x: 70.0, y: 70.0 })?;
    let guide_id = object_id_named(store, "Text");
    store.apply(Operation::RenameObject {
        object_id: guide_id.clone(),
        name: "Tutorial walkthrough".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: guide_id.clone(),
        source: source.into(),
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(store, &guide_id),
        width: 580.0,
        height: 820.0,
    })?;
    Ok(())
}

fn add_block(
    store: &mut Store,
    name: &str,
    x: f64,
    y: f64,
    source: &str,
) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddBlock {
        name: name.into(),
        x,
        y,
    })?;
    set_block(store, name, source)
}

fn set_block(store: &mut Store, name: &str, source: &str) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::SetBlockSource {
        block_id: object_id_named(store, name),
        source: source.into(),
        editing: None,
    })?;
    Ok(())
}

fn with_columns(columns: &[(&str, &str)]) -> FrameStepInput {
    FrameStepInput::WithColumns {
        columns: columns
            .iter()
            .map(|(name, formula)| ExistingFormulaInput {
                output_column_id: column_id(name),
                name: (*name).into(),
                formula: (*formula).into(),
            })
            .collect(),
    }
}

fn sort_by(frame: &framework_core::FrameObject, column: &str) -> FrameStepInput {
    FrameStepInput::Sort {
        keys: vec![SortInput {
            column_id: column_id_named(frame, column),
            descending: false,
        }],
    }
}

fn format_columns(
    store: &mut Store,
    frame_name: &str,
    names: &[&str],
    format: ColumnFormat,
) -> Result<(), framework_core::CoreError> {
    let frame = frame(store, frame_name);
    for name in names {
        store.apply(Operation::SetColumnFormat {
            frame_id: frame.id.clone(),
            column_id: column_id_named(&frame, name),
            format: Some(format.clone()),
        })?;
    }
    Ok(())
}

fn set_column_type(
    store: &mut Store,
    frame_name: &str,
    column: &str,
    data_type: DataType,
) -> Result<(), framework_core::CoreError> {
    let frame = frame(store, frame_name);
    store.apply(Operation::SetColumnType {
        frame_id: frame.id.clone(),
        column_id: column_id_named(&frame, column),
        data_type,
    })?;
    Ok(())
}

fn add_value_in(
    store: &mut Store,
    container: &str,
    name: &str,
    raw: &str,
) -> Result<(), framework_core::CoreError> {
    let container_id = object_id_named(store, container);
    store.apply(Operation::AddValue {
        name: name.into(),
        raw: raw.into(),
        x: 0.0,
        y: 0.0,
        container_id: Some(container_id),
    })?;
    Ok(())
}

fn scenario_id(store: &Store, name: &str) -> String {
    store
        .document()
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap_or_else(|| panic!("scenario {name:?} exists"))
        .id
        .clone()
}

fn add_scenario(
    store: &mut Store,
    name: &str,
    values: &[(&str, &str)],
) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddScenario {
        scenario_id: None,
        name: name.into(),
        copy_from: None,
    })?;
    let scenario_id = scenario_id(store, name);
    for (value, raw) in values {
        store.apply(Operation::SetScenarioValue {
            scenario_id: scenario_id.clone(),
            value_id: object_id_named(store, value),
            raw: Some((*raw).into()),
        })?;
    }
    Ok(())
}

fn add_matrix(
    store: &mut Store,
    name: &str,
    x: f64,
    y: f64,
    rows: &[(&str, &str)],
    columns: &[(&str, &str)],
    body: &str,
) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddCalculationMatrix {
        name: name.into(),
        x,
        y,
    })?;
    let axis = |axes: &[(&str, &str)]| {
        axes.iter()
            .map(
                |(name, formula)| framework_core::CalculationMatrixFormulaInput {
                    target_id: None,
                    id: None,
                    name: (*name).into(),
                    formula: (*formula).into(),
                },
            )
            .collect::<Vec<_>>()
    };
    store.apply(Operation::SetCalculationMatrix {
        object_id: object_id_named(store, name),
        rows: axis(rows),
        columns: axis(columns),
        body: body.into(),
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading the answer key back
// ---------------------------------------------------------------------------

fn page(store: &Store, frame_name: &str) -> framework_core::FramePage {
    store
        .get_frame_page(&frame(store, frame_name).id, 0, 200)
        .unwrap_or_else(|error| panic!("{frame_name} pages: {error}"))
}

fn cell<'a>(page: &'a framework_core::FramePage, first_cell: &str, column: &str) -> &'a str {
    let index = page
        .columns
        .iter()
        .position(|c| c.name == column)
        .unwrap_or_else(|| panic!("column {column:?} in page"));
    let row = page
        .rows
        .iter()
        .find(|row| row[0] == first_cell)
        .unwrap_or_else(|| panic!("row starting {first_cell:?} exists"));
    &row[index]
}

fn number(display: &str) -> f64 {
    display
        .replace(['$', ','], "")
        .parse()
        .unwrap_or_else(|_| panic!("{display:?} is a number"))
}

fn assert_close(actual: &str, expected: f64, what: &str) {
    let actual = number(actual);
    let tolerance = (expected.abs() * 1e-6).max(0.011);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: got {actual}, expected {expected}"
    );
}

fn assert_cell_close(page: &framework_core::FramePage, first: &str, column: &str, expected: f64) {
    assert_close(
        cell(page, first, column),
        expected,
        &format!("{first} {column}"),
    );
}

/// Every expression line's answer, in order, after checking none of them
/// errs. Comment and blank lines have no answer and are skipped.
fn block_answers(store: &Store, name: &str) -> Vec<String> {
    let id = object_id_named(store, name);
    store.view().computed_blocks[&id]
        .lines
        .iter()
        .filter(|line| {
            let text = line.text.trim();
            !text.is_empty() && !text.starts_with('#')
        })
        .inspect(|line| {
            assert!(
                line.cell.error.is_none(),
                "block {name:?} line {:?} errs: {}",
                line.text,
                line.cell.error.clone().unwrap_or_default()
            );
        })
        .filter(|line| !line.cell.display.is_empty())
        .map(|line| line.cell.display.clone())
        .collect()
}

fn assert_block_close(store: &Store, name: &str, expected: &[f64]) {
    let answers = block_answers(store, name);
    assert_eq!(
        answers.len(),
        expected.len(),
        "block {name:?} answers {answers:?}"
    );
    for (index, (answer, expected)) in answers.iter().zip(expected).enumerate() {
        assert_close(answer, *expected, &format!("block {name:?} line {index}"));
    }
}

fn matrix_cells(store: &Store, name: &str) -> Vec<Vec<String>> {
    let id = object_id_named(store, name);
    let computed = &store.view().computed_calculation_matrices[&id];
    assert!(
        computed.error.is_none(),
        "matrix {name:?} errs: {}",
        computed.error.clone().unwrap_or_default()
    );
    computed
        .cells
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    assert!(
                        cell.error.is_none(),
                        "matrix {name:?} cell errs: {}",
                        cell.error.clone().unwrap_or_default()
                    );
                    cell.display.clone()
                })
                .collect()
        })
        .collect()
}

fn assert_matrix_close(store: &Store, name: &str, expected: &[&[f64]]) {
    let cells = matrix_cells(store, name);
    assert_eq!(
        cells.len(),
        expected.len(),
        "matrix {name:?} rows {cells:?}"
    );
    for (r, (row, expected_row)) in cells.iter().zip(expected).enumerate() {
        assert_eq!(
            row.len(),
            expected_row.len(),
            "matrix {name:?} row {r} {row:?}"
        );
        for (c, (cell, expected)) in row.iter().zip(*expected_row).enumerate() {
            assert_close(cell, *expected, &format!("matrix {name:?} [{r}][{c}]"));
        }
    }
}

/// Everything the asserts look at, printed when `FINANCE_TUTORIALS_DUMP` is
/// set, so a README checkpoint can be read off one run.
fn dump(store: &Store, frames: &[&str], blocks: &[&str], matrices: &[&str]) {
    if std::env::var_os("FINANCE_TUTORIALS_DUMP").is_none() {
        return;
    }
    for name in frames {
        let page = page(store, name);
        println!(
            "frame {name}: {:?}",
            page.columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
        );
        for row in &page.rows {
            println!("  {row:?}");
        }
    }
    for name in blocks {
        let id = object_id_named(store, name);
        for line in &store.view().computed_blocks[&id].lines {
            println!(
                "block {name} | {} => {:?} {:?}",
                line.text, line.cell.display, line.cell.error
            );
        }
    }
    for name in matrices {
        let id = object_id_named(store, name);
        let computed = &store.view().computed_calculation_matrices[&id];
        println!("matrix {name}: error={:?}", computed.error);
        for row in &computed.cells {
            println!(
                "  {:?}",
                row.iter()
                    .map(|c| (c.display.as_str(), c.error.as_deref()))
                    .collect::<Vec<_>>()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Lesson 1: price a deal
// ---------------------------------------------------------------------------

#[path = "finance/price.rs"]
mod price;
use price::generate_price_a_deal;

// ---------------------------------------------------------------------------
// Lesson 2: driver-based forecast
// ---------------------------------------------------------------------------

const ACTUALS: &[(&str, &str)] = &[
    ("2025-02-01", "100000"),
    ("2025-03-01", "104000"),
    ("2025-04-01", "110000"),
    ("2025-05-01", "118000"),
    ("2025-06-01", "125000"),
    ("2025-07-01", "131000"),
    ("2025-08-01", "128000"),
    ("2025-09-01", "122000"),
    ("2025-10-01", "119000"),
    ("2025-11-01", "126000"),
    ("2025-12-01", "140000"),
    ("2026-01-01", "152000"),
    ("2026-02-01", "112000"),
    ("2026-03-01", "116000"),
    ("2026-04-01", "123000"),
    ("2026-05-01", "131000"),
    ("2026-06-01", "139000"),
    ("2026-07-01", "146000"),
];

fn generate_driver_forecast(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("Driver-based forecast"));
    add_walkthrough(
        &mut store,
        include_str!("../../../tutorials/driver-forecast/README.md"),
    )?;
    let mut rows: Vec<Vec<String>> = vec![vec!["Month".into(), "Revenue".into()]];
    rows.extend(
        ACTUALS
            .iter()
            .map(|(month, revenue)| vec![(*month).into(), (*revenue).into()]),
    );
    store.apply(Operation::AddFrame {
        name: "Actuals".into(),
        grid: rows,
        x: 720.0,
        y: 70.0,
    })?;
    // Currency rather than the inferred integer: the recurrence in section 3
    // seeds from this column and keeps its type.
    set_column_type(&mut store, "Actuals", "Revenue", DataType::Currency)?;
    format_columns(&mut store, "Actuals", &["Revenue"], money_format())?;
    store.apply(Operation::AddContainer {
        name: "Assumptions".into(),
        x: 1410.0,
        y: 70.0,
        container_id: None,
    })?;
    add_value_in(&mut store, "Assumptions", "Close date", "2026-07-01")?;
    add_value_in(&mut store, "Assumptions", "Growth", "0.02")?;
    let start = output.join("driver-forecast-start.fw");
    store.save(&start)?;

    // Section 1: the fragile positional prior.
    let actuals = frame(&store, "Actuals");
    store.apply(Operation::SetFramePipeline {
        frame_id: actuals.id.clone(),
        steps: vec![
            sort_by(&actuals, "Month"),
            with_columns(&[("Prior month", "`Revenue`.shift(1)")]),
            with_columns(&[("Change", "`Revenue` / `Prior month` - 1")]),
        ],
    })?;
    format_columns(&mut store, "Actuals", &["Prior month"], money_format())?;
    format_columns(&mut store, "Actuals", &["Change"], percent_format())?;

    // Section 2: the spine and the join.
    let assumptions_id = object_id_named(&store, "Assumptions");
    let months_y = 70.0 + view_height(&store, &assumptions_id) + 40.0;
    store.apply(Operation::AddGeneratorFrame {
        name: "Months".into(),
        formula: "sequence(2025-02-01, 2027-02-01, 1mo)".into(),
        column_name: Some("Month".into()),
        x: 1410.0,
        y: months_y,
    })?;
    let actuals = frame(&store, "Actuals");
    let months = frame(&store, "Months");
    store.apply(Operation::SetUniqueKey {
        frame_id: actuals.id.clone(),
        column_ids: vec![column_id_named(&actuals, "Month")],
        enabled: true,
    })?;
    store.apply(Operation::AddJoinFrame {
        primary_frame_id: months.id.clone(),
        lookup_frame_id: actuals.id.clone(),
        primary_key_column_ids: vec![column_id_named(&months, "Month")],
        lookup_key_column_ids: vec![column_id_named(&actuals, "Month")],
        join_type: FrameJoinType::Left,
        columns: vec![
            JoinColumnInput {
                source_frame_id: months.id.clone(),
                source_column_id: column_id_named(&months, "Month"),
                name: "Month".into(),
            },
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: column_id_named(&actuals, "Revenue"),
                name: "Actual".into(),
            },
        ],
        name: "Forecast".into(),
        x: 2100.0,
        y: 70.0,
    })?;

    // Sections 3 to 5: cutover, hand-written calendar, positional windows.
    let forecast = frame(&store, "Forecast");
    store.apply(Operation::SetFramePipeline {
        frame_id: forecast.id.clone(),
        steps: vec![
            sort_by(&forecast, "Month"),
            with_columns(&[(
                "Revenue",
                "recur(`Actual`, when(`Month` <= `Close date`).then(`Actual`).otherwise(previous() * (1 + `Growth`)))",
            )]),
            with_columns(&[
                (
                    "Fiscal year",
                    "when(`Month`.dt.month() >= 2).then(`Month`.dt.year() + 1).otherwise(`Month`.dt.year())",
                ),
                ("Fiscal quarter", "((`Month`.dt.month() + 10) % 12) // 3 + 1"),
            ]),
            with_columns(&[
                ("Prior month", "`Revenue`.shift(1)"),
                ("YTD", "`Revenue`.cum_sum(False).over(`Fiscal year`)"),
                ("Last year", "`Revenue`.shift(12)"),
            ]),
            with_columns(&[("YoY", "`Revenue` / `Last year` - 1")]),
        ],
    })?;
    format_columns(
        &mut store,
        "Forecast",
        &["Actual", "Revenue", "Prior month", "YTD", "Last year"],
        money_format(),
    )?;
    format_columns(&mut store, "Forecast", &["YoY"], percent_format())?;

    // Section 6: the quarter summary and the checks.
    let forecast = frame(&store, "Forecast");
    store.apply(Operation::AddDerivedFrame {
        source_frame_id: forecast.id.clone(),
        name: "By quarter".into(),
        group_keys: vec![
            framework_core::NamedFormulaInput {
                name: "Fiscal year".into(),
                formula: "`Fiscal year`".into(),
            },
            framework_core::NamedFormulaInput {
                name: "Fiscal quarter".into(),
                formula: "`Fiscal quarter`".into(),
            },
        ],
        aggregates: vec![framework_core::NamedFormulaInput {
            name: "Revenue".into(),
            formula: "`Revenue`.sum()".into(),
        }],
        maintain_order: true,
        x: 2100.0,
        y: 70.0 + view_height(&store, &forecast.id) + 40.0,
    })?;
    format_columns(&mut store, "By quarter", &["Revenue"], money_format())?;
    let quarter_id = frame(&store, "By quarter").id;
    let checks_y =
        70.0 + view_height(&store, &forecast.id) + 40.0 + view_height(&store, &quarter_id) + 40.0;
    add_block(
        &mut store,
        "Checks",
        2100.0,
        checks_y,
        "fy2026 = `Forecast`.`Revenue`.filter(`Forecast`.`Fiscal year` == 2026).sum()\nfy2027 = `Forecast`.`Revenue`.filter(`Forecast`.`Fiscal year` == 2027).sum()\nforecast months = `Forecast`.`Actual`.null_count()",
    )?;

    let finished = output.join("driver-forecast-finished.fw");
    store.save(&finished)?;

    let reloaded = Store::load(&finished)?;
    dump(
        &reloaded,
        &["Actuals", "Forecast", "By quarter"],
        &["Checks"],
        &[],
    );
    let actuals = page(&reloaded, "Actuals");
    assert_cell_close(&actuals, "2025-03-01", "Prior month", 100000.0);
    assert_cell_close(&actuals, "2025-03-01", "Change", 0.04);
    let forecast = page(&reloaded, "Forecast");
    assert_eq!(forecast.total_rows, 24);
    assert_cell_close(&forecast, "2026-07-01", "Revenue", 146000.0);
    assert_cell_close(&forecast, "2026-08-01", "Revenue", 148920.0);
    assert_cell_close(&forecast, "2027-01-01", "Revenue", 164419.71);
    assert_eq!(cell(&forecast, "2026-01-01", "Fiscal year"), "2026");
    assert_eq!(cell(&forecast, "2026-01-01", "Fiscal quarter"), "4");
    assert_eq!(cell(&forecast, "2026-02-01", "Fiscal year"), "2027");
    assert_eq!(cell(&forecast, "2026-02-01", "Fiscal quarter"), "1");
    assert_cell_close(&forecast, "2026-01-01", "YTD", 1475000.0);
    assert_cell_close(&forecast, "2026-07-01", "YTD", 767000.0);
    assert_cell_close(&forecast, "2026-08-01", "YoY", 0.163437);
    assert_cell_close(&forecast, "2027-01-01", "YoY", 0.081709);
    let quarters = page(&reloaded, "By quarter");
    let expected = [
        ("2026", "1", 314000.0),
        ("2026", "2", 374000.0),
        ("2026", "3", 369000.0),
        ("2026", "4", 418000.0),
        ("2027", "1", 351000.0),
        ("2027", "2", 416000.0),
        ("2027", "3", 455754.77),
        ("2027", "4", 483650.61),
    ];
    assert_eq!(quarters.rows.len(), expected.len());
    for (row, (year, quarter, revenue)) in quarters.rows.iter().zip(expected) {
        assert_eq!(row[0], year);
        assert_eq!(row[1], quarter);
        assert_close(&row[2], revenue, &format!("FY{year} Q{quarter}"));
    }
    assert_block_close(&reloaded, "Checks", &[1475000.0, 1706405.37, 6.0]);
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Lesson 3: scenarios, sensitivity and goal seek
// ---------------------------------------------------------------------------

fn generate_scenarios(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("Scenarios, sensitivity and goal seek"));
    add_walkthrough(
        &mut store,
        include_str!("../../../tutorials/scenarios-and-sensitivity/README.md"),
    )?;
    store.apply(Operation::AddContainer {
        name: "Assumptions".into(),
        x: 720.0,
        y: 70.0,
        container_id: None,
    })?;
    for (name, raw) in [
        ("Price", "120"),
        ("Annual units", "8000"),
        ("Unit cost", "70"),
        ("Fixed costs", "250000"),
    ] {
        add_value_in(&mut store, "Assumptions", name, raw)?;
    }
    add_scenario(
        &mut store,
        "Upside",
        &[("Price", "125"), ("Annual units", "9500")],
    )?;
    add_scenario(
        &mut store,
        "Downside",
        &[("Price", "115"), ("Annual units", "6500")],
    )?;
    let assumptions_id = object_id_named(&store, "Assumptions");
    let plan_y = 70.0 + view_height(&store, &assumptions_id) + 40.0;
    store.apply(Operation::AddFrame {
        name: "Plan".into(),
        grid: grid(&[
            &["Month", "Weight"],
            &["Jan", "6"],
            &["Feb", "6"],
            &["Mar", "7"],
            &["Apr", "8"],
            &["May", "9"],
            &["Jun", "9"],
            &["Jul", "9"],
            &["Aug", "9"],
            &["Sep", "8"],
            &["Oct", "9"],
            &["Nov", "10"],
            &["Dec", "10"],
        ]),
        x: 720.0,
        y: plan_y,
    })?;
    let start = output.join("scenarios-and-sensitivity-start.fw");
    store.save(&start)?;

    // Section 1: the model, partly in a table on purpose.
    let plan = frame(&store, "Plan");
    store.apply(Operation::SetFramePipeline {
        frame_id: plan.id.clone(),
        steps: vec![
            with_columns(&[("Units", "`Annual units` * `Weight` / 100")]),
            with_columns(&[("Revenue", "`Units` * `Price`")]),
            with_columns(&[("Cost", "`Units` * `Unit cost`")]),
        ],
    })?;
    format_columns(&mut store, "Plan", &["Revenue", "Cost"], money_format())?;
    add_block(
        &mut store,
        "Model",
        1410.0,
        70.0,
        "revenue = `Plan`.`Revenue`.sum()\ngross = revenue - `Plan`.`Cost`.sum()\nebitda = gross - `Fixed costs`\nmargin = ebitda / revenue",
    )?;

    // Section 3: the grid, with the model retyped as its body.
    let model_id = object_id_named(&store, "Model");
    let axes_y = 70.0 + view_height(&store, &model_id) + 40.0;
    store.apply(Operation::AddVariable {
        name: "Price axis".into(),
        formula: "[100, 110, 120, 130, 140]".into(),
        x: 1410.0,
        y: axes_y,
    })?;
    store.apply(Operation::AddVariable {
        name: "Units axis".into(),
        formula: "[6000, 7000, 8000, 9000, 10000]".into(),
        x: 1410.0,
        y: axes_y + 70.0,
    })?;
    add_matrix(
        &mut store,
        "EBITDA by price and units",
        1410.0,
        axes_y + 140.0,
        &[("Price axis", "`Price axis`")],
        &[("Units axis", "`Units axis`")],
        "(`Price axis` - `Unit cost`) * `Units axis` - `Fixed costs`",
    )?;

    // Section 4: the scan that stands in for goal seek.
    store.apply(Operation::AddVariable {
        name: "Price scan".into(),
        formula: "sequence(100, 145, 5)".into(),
        x: 2100.0,
        y: 70.0,
    })?;
    store.apply(Operation::AddVariable {
        name: "Answer".into(),
        formula: "[\"EBITDA\"]".into(),
        x: 2100.0,
        y: 140.0,
    })?;
    add_matrix(
        &mut store,
        "EBITDA by price",
        2100.0,
        210.0,
        &[("Price scan", "`Price scan`")],
        &[("Answer", "`Answer`")],
        "(`Price scan` - `Unit cost`) * `Annual units` - `Fixed costs`",
    )?;

    let finished = output.join("scenarios-and-sensitivity-finished.fw");
    store.save(&finished)?;

    let mut reloaded = Store::load(&finished)?;
    dump(
        &reloaded,
        &["Plan"],
        &["Model"],
        &["EBITDA by price and units", "EBITDA by price"],
    );
    let base = [960000.0, 400000.0, 150000.0, 0.15625];
    assert_block_close(&reloaded, "Model", &base);
    for (name, expected) in [
        ("Upside", [1187500.0, 522500.0, 272500.0, 0.229474]),
        ("Downside", [747500.0, 292500.0, 42500.0, 0.056856]),
    ] {
        let id = scenario_id(&reloaded, name);
        reloaded.apply(Operation::ActivateScenario {
            scenario_id: Some(id),
        })?;
        assert_block_close(&reloaded, "Model", &expected);
    }
    reloaded.apply(Operation::ActivateScenario { scenario_id: None })?;
    assert_block_close(&reloaded, "Model", &base);
    assert_matrix_close(
        &reloaded,
        "EBITDA by price and units",
        &[
            &[-70000.0, -40000.0, -10000.0, 20000.0, 50000.0],
            &[-10000.0, 30000.0, 70000.0, 110000.0, 150000.0],
            &[50000.0, 100000.0, 150000.0, 200000.0, 250000.0],
            &[110000.0, 170000.0, 230000.0, 290000.0, 350000.0],
            &[170000.0, 240000.0, 310000.0, 380000.0, 450000.0],
        ],
    );
    assert_matrix_close(
        &reloaded,
        "EBITDA by price",
        &[
            &[-10000.0],
            &[30000.0],
            &[70000.0],
            &[110000.0],
            &[150000.0],
            &[190000.0],
            &[230000.0],
            &[270000.0],
            &[310000.0],
        ],
    );
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

type Lesson = fn(&Path) -> Result<(), Box<dyn std::error::Error>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let lessons: &[(&str, Lesson)] = &[
        ("price-a-deal", generate_price_a_deal),
        ("driver-forecast", generate_driver_forecast),
        ("scenarios-and-sensitivity", generate_scenarios),
    ];
    let chosen = std::env::args().nth(1);
    for (name, generate) in lessons {
        if chosen.as_deref().is_none_or(|wanted| wanted == *name) {
            generate(&workspace.join("tutorials").join(name))?;
        }
    }
    Ok(())
}
