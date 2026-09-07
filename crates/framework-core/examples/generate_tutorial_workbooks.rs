use framework_core::{
    CalculationMatrixFormulaInput, ColumnFormat, ColumnFormatScale, ColumnFormatStyle, DataObject,
    DerivedSort, Document, ExistingFormulaInput, FrameJoinType, FrameStepInput, JoinColumnInput,
    Operation, PivotAggregate, Store, column_id,
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

fn block_id(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == name => Some(block.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial block {name:?} exists"))
}

fn assert_block_answers(store: &Store, name: &str, expected: &[&str]) {
    let id = block_id(store, name);
    assert_eq!(
        store.view().computed_blocks[&id]
            .lines
            .iter()
            .map(|line| line.cell.display.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn add_seventh_launch_row(
    store: &mut Store,
    source: &framework_core::FrameObject,
) -> Result<(), framework_core::CoreError> {
    let mut values = std::collections::BTreeMap::new();
    for (name, value) in [("Line", "7"), ("SKU", "C-300"), ("Units", "50")] {
        values.insert(column_id_named(source, name), value.into());
    }
    store.apply(Operation::AddRow {
        frame_id: source.id.clone(),
        values,
    })?;
    Ok(())
}

fn text_id(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Text(text) if text.name == name => Some(text.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial text {name:?} exists"))
}

fn calculation_matrix_id(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::CalculationMatrix(matrix) if matrix.name == name => Some(matrix.id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("tutorial calculation matrix {name:?} exists"))
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

/// The height the engine actually gave an object's card, so the next card
/// in a column can be placed below it instead of at a guessed offset —
/// frame cards size to their row count now (`frame_card_height` in
/// `engine/build.rs`), so a fixed gap between rows silently overlaps once a
/// card grows past the old ~300-450px guesses.
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
        zero_dash: Some(true),
        currency_code: None,
        date_pattern: None,
    }
}

/// Put the lesson beside its workbook, not in a repository the learner may
/// never see. Existing cards move as a group so the walkthrough owns the
/// opening edge of the canvas without covering the spreadsheet it explains.
fn add_tutorial_walkthrough(
    store: &mut Store,
    source: &str,
) -> Result<(), framework_core::CoreError> {
    let existing = store
        .document()
        .views
        .iter()
        .map(|view| (view.id.clone(), view.x, view.y))
        .collect::<Vec<_>>();
    for (view_id, x, y) in existing {
        store.apply(Operation::MoveView {
            view_id,
            x: x + 650.0,
            y,
        })?;
    }
    store.apply(Operation::AddText { x: 70.0, y: 70.0 })?;
    let guide_id = text_id(store, "Text");
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

fn branch_frame_mut(
    store: &mut Store,
    source_name: &str,
    branch_name: &str,
) -> Result<framework_core::FrameObject, framework_core::CoreError> {
    let source = frame(store, source_name);
    let existing_ids = store
        .document()
        .objects
        .iter()
        .map(|object| object.id().to_string())
        .collect::<std::collections::HashSet<_>>();
    store.apply(Operation::BranchFrame {
        view_id: view_id(store, &source.id),
        frame_id: source.id.clone(),
    })?;
    let branch = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if !existing_ids.contains(&frame.id) => Some(frame.clone()),
            _ => None,
        })
        .expect("branch operation adds a frame");
    store.apply(Operation::RenameObject {
        object_id: branch.id.clone(),
        name: branch_name.into(),
    })?;
    Ok(frame(store, branch_name))
}

fn pass_through_steps(branch: &framework_core::FrameObject) -> Vec<FrameStepInput> {
    vec![
        FrameStepInput::WithColumns {
            columns: branch
                .columns
                .iter()
                .map(|column| ExistingFormulaInput {
                    output_column_id: column.id.clone(),
                    name: column.name.clone(),
                    formula: format!("`{}`", column.name.replace('`', "``")),
                })
                .collect(),
        },
        FrameStepInput::Select {
            column_ids: branch
                .columns
                .iter()
                .map(|column| column.id.clone())
                .collect(),
        },
    ]
}

fn add_sales_narrative(store: &mut Store) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddText {
        x: 1270.0,
        y: 669.0,
    })?;
    let narrative_id = text_id(store, "Text");
    store.apply(Operation::RenameObject {
        object_id: narrative_id.clone(),
        name: "Sales narrative".into(),
    })?;
    store.apply(Operation::SetTextSource {
        object_id: narrative_id.clone(),
        source: "## Monthly sales\n\nRevenue is {{`Monthly sales`.`Revenue`.sum()}} and profit is {{`Monthly sales`.`Profit`.sum()}}.".into(),
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(store, &narrative_id),
        width: 520.0,
        height: 240.0,
    })?;
    Ok(())
}

/// The tour's Start workbook: the empty table its first section pastes into,
/// and the budget its join reads. Everything else in the lesson is something
/// the reader makes, including the Scratchwork block — ⌘J writes that one, and
/// shipping it pre-made would hide the gesture the section exists to teach.
fn grand_tour_start(store: &mut Store) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddFrame {
        name: "Monthly sales".into(),
        grid: vec![
            vec!["Column 1".into(), "Column 2".into()],
            vec![String::new(), String::new()],
            vec![String::new(), String::new()],
        ],
        x: 70.0,
        y: 70.0,
    })?;
    store.apply(Operation::AddFrame {
        name: "Budget".into(),
        grid: vec![
            vec!["Month", "Budget"],
            vec!["2026-01", "120000"],
            vec!["2026-02", "120000"],
            vec!["2026-03", "130000"],
            vec!["2026-04", "138000"],
            vec!["2026-05", "150000"],
            vec!["2026-06", "160000"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 70.0,
        y: 500.0,
    })?;
    add_tutorial_walkthrough(
        store,
        include_str!("../../../tutorials/grand-tour/README.md"),
    )
}

/// Sections 1, 3, 4 and 8: the pasted rows, the two calculated columns, and
/// the declared order between them. Written as one pipeline because that is
/// what the three gestures leave behind — `SetFrameDisplaySort` is the header
/// control, and it appends the trailing Sort the lesson tells the reader to
/// find in Wrangle.
fn grand_tour_sales(store: &mut Store) -> Result<(), framework_core::CoreError> {
    let sales_id = frame(store, "Monthly sales").id;
    store.apply(Operation::SetFrameFromPastedText {
        frame_id: sales_id.clone(),
        text: "Month\tRegion\tRevenue\tCost\n2026-04\tWest\t142000\t91000\n2026-01\tEast\t118000\t76000\n2026-06\tEast\t168000\t104000\n2026-03\tEast\t136000\t85000\n2026-02\tWest\t124000\t79000\n2026-05\tEast\t151000\t96000\n".into(),
    })?;
    store.apply(Operation::SetFramePipeline {
        frame_id: sales_id.clone(),
        steps: vec![
            FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: column_id("Profit"),
                    name: "Profit".into(),
                    formula: "`Revenue` - `Cost`".into(),
                }],
            },
            FrameStepInput::Sort {
                keys: vec![framework_core::SortInput {
                    column_id: column_id_named(&frame(store, "Monthly sales"), "Month"),
                    descending: false,
                }],
            },
            FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: column_id("Forecast"),
                    name: "Forecast".into(),
                    formula: "(`Revenue` * `Growth`).round()".into(),
                }],
            },
        ],
    })?;
    let sales = frame(store, "Monthly sales");
    for name in ["Revenue", "Cost", "Profit", "Forecast"] {
        store.apply(Operation::SetColumnFormat {
            frame_id: sales.id.clone(),
            column_id: column_id_named(&sales, name),
            format: Some(money_format()),
        })?;
    }
    Ok(())
}

/// Section 8's assumption: one value in a group, and the two scenarios that
/// disagree with it. The answer key is saved on the base so its checkpoints
/// are the numbers a reader sees on opening it.
fn grand_tour_assumptions(store: &mut Store) -> Result<(), framework_core::CoreError> {
    store.apply(Operation::AddContainer {
        name: "Assumptions".into(),
        x: 1990.0,
        y: 500.0,
        container_id: None,
    })?;
    let container_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Container(container) if container.name == "Assumptions" => {
                Some(container.id.clone())
            }
            _ => None,
        })
        .expect("the tour's assumptions group exists");
    store.apply(Operation::AddValue {
        name: "Growth".into(),
        raw: "1.08".into(),
        x: 0.0,
        y: 0.0,
        container_id: Some(container_id),
    })?;
    let growth_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Value(value) if value.name == "Growth" => Some(value.id.clone()),
            _ => None,
        })
        .expect("the tour's growth assumption exists");
    for (name, raw) in [("Upside", "1.15"), ("Downside", "0.95")] {
        store.apply(Operation::AddScenario {
            scenario_id: None,
            name: name.into(),
            copy_from: None,
        })?;
        let scenario_id = store
            .document()
            .scenarios
            .iter()
            .find(|scenario| scenario.name == name)
            .expect("the scenario just added exists")
            .id
            .clone();
        store.apply(Operation::SetScenarioValue {
            scenario_id,
            value_id: growth_id.clone(),
            raw: Some(raw.into()),
        })?;
    }
    Ok(())
}

/// Section 6 and 7: the dragged join, the variance beside it, and the branch
/// that summarizes the whole thing by region.
fn grand_tour_analysis(store: &mut Store) -> Result<(), framework_core::CoreError> {
    let sales = frame(store, "Monthly sales");
    let budget = frame(store, "Budget");
    let budget_month = column_id_named(&budget, "Month");
    store.apply(Operation::SetUniqueKey {
        frame_id: budget.id.clone(),
        column_ids: vec![budget_month.clone()],
        enabled: true,
    })?;
    store.apply(Operation::AddJoinFrame {
        primary_frame_id: sales.id.clone(),
        lookup_frame_id: budget.id.clone(),
        primary_key_column_ids: vec![column_id_named(&sales, "Month")],
        lookup_key_column_ids: vec![budget_month],
        join_type: FrameJoinType::Left,
        columns: ["Month", "Region", "Revenue", "Cost", "Profit"]
            .into_iter()
            .map(|name| JoinColumnInput {
                source_frame_id: sales.id.clone(),
                source_column_id: column_id_named(&sales, name),
                name: name.into(),
            })
            .chain(std::iter::once(JoinColumnInput {
                source_frame_id: budget.id.clone(),
                source_column_id: column_id_named(&budget, "Budget"),
                name: "Budget".into(),
            }))
            .collect(),
        name: "Sales vs budget".into(),
        x: 1290.0,
        y: 70.0,
    })?;
    let analysis = frame(store, "Sales vs budget");
    store.apply(Operation::SetFramePipeline {
        frame_id: analysis.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: column_id("Variance"),
                name: "Variance".into(),
                formula: "`Revenue` - `Budget`".into(),
            }],
        }],
    })?;
    let analysis = frame(store, "Sales vs budget");
    for name in ["Revenue", "Cost", "Profit", "Budget", "Variance"] {
        store.apply(Operation::SetColumnFormat {
            frame_id: analysis.id.clone(),
            column_id: column_id_named(&analysis, name),
            format: Some(money_format()),
        })?;
    }

    let summary = branch_frame_mut(store, "Sales vs budget", "By region")?;
    let mut summary_steps = pass_through_steps(&summary);
    summary_steps.push(FrameStepInput::Summarize {
        group_keys: vec![ExistingFormulaInput {
            output_column_id: column_id("Region"),
            name: "Region".into(),
            formula: "`Region`".into(),
        }],
        aggregates: ["Revenue", "Budget", "Variance"]
            .into_iter()
            .map(|name| ExistingFormulaInput {
                output_column_id: column_id(&format!("Total {name}")),
                name: format!("Total {name}"),
                formula: format!("`{name}`.sum()"),
            })
            .collect(),
        maintain_order: true,
    });
    store.apply(Operation::SetFramePipeline {
        frame_id: summary.id,
        steps: summary_steps,
    })?;
    Ok(())
}

/// Section 9, plus the layout the answer key opens on. The chart shares the
/// analysis card's tab strip because the lesson's claim is that a plot lives
/// beside the table it explains rather than in a chart sheet of its own.
fn grand_tour_layout(store: &mut Store) -> Result<(), framework_core::CoreError> {
    let analysis = frame(store, "Sales vs budget");
    let analysis_view = view_id(store, &analysis.id);
    store.apply(Operation::ResizeView {
        view_id: analysis_view.clone(),
        width: 1080.0,
        height: 430.0,
    })?;
    store.apply(Operation::AddPlot {
        name: "Revenue by month".into(),
        source_frame_id: analysis.id.clone(),
        spec: serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v6.json",
            "mark": {"type": "bar", "tooltip": true},
            "encoding": {
                "x": {
                    "field": column_id_named(&analysis, "Month"),
                    "type": "nominal",
                    "title": "Month",
                    "sort": null
                },
                "y": {
                    "field": column_id_named(&analysis, "Revenue"),
                    "type": "quantitative",
                    "title": "Revenue"
                },
                "color": {
                    "field": column_id_named(&analysis, "Region"),
                    "type": "nominal",
                    "title": "Region"
                }
            },
            "title": "Revenue by month"
        }),
        x: 0.0,
        y: 0.0,
        view_id: Some(analysis_view.clone()),
    })?;
    store.apply(Operation::SetActiveTab {
        view_id: analysis_view.clone(),
        object_id: analysis.id.clone(),
    })?;
    // One column of work beside the walkthrough, in the order the tour builds
    // it: the source table, then the analysis card — which carries the join,
    // the regional summary and the chart as tabs, because a branch is a tab
    // rather than a card of its own — then the answers. The inputs the reader
    // does not edit again sit to the right of that column.
    store.apply(Operation::MoveView {
        view_id: analysis_view,
        x: 720.0,
        y: 490.0,
    })?;
    for (object_id, x, y, width, height) in [
        (frame(store, "Monthly sales").id, 720.0, 70.0, 1080.0, 380.0),
        (block_id(store, "Scratchwork"), 720.0, 960.0, 620.0, 220.0),
        (frame(store, "Budget").id, 1840.0, 70.0, 380.0, 330.0),
    ] {
        let view = view_id(store, &object_id);
        store.apply(Operation::MoveView {
            view_id: view.clone(),
            x,
            y,
        })?;
        store.apply(Operation::ResizeView {
            view_id: view,
            width,
            height,
        })?;
    }
    let assumptions = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Container(container) if container.name == "Assumptions" => {
                Some(container.id.clone())
            }
            _ => None,
        })
        .expect("the tour's assumptions group exists");
    store.apply(Operation::MoveView {
        view_id: view_id(store, &assumptions),
        x: 1840.0,
        y: 440.0,
    })?;
    Ok(())
}

fn generate_grand_tour(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("The FrameWork tour"));
    grand_tour_start(&mut store)?;
    let start = output.join("grand-tour-start.fw");
    store.save(&start)?;

    // Order matters here the way it matters in the lesson: the assumption
    // exists before the Forecast column that reads it, and the join copies
    // the source columns that exist when it is made.
    grand_tour_assumptions(&mut store)?;
    grand_tour_sales(&mut store)?;
    grand_tour_analysis(&mut store)?;
    store.apply(Operation::AddBlock {
        name: "Scratchwork".into(),
        x: 720.0,
        y: 500.0,
    })?;
    store.apply(Operation::SetBlockSource {
        block_id: block_id(&store, "Scratchwork"),
        source: "Total revenue = `Monthly sales`.`Revenue`.sum()\nTotal variance = `Sales vs budget`.`Variance`.sum()\nForecast total = `Monthly sales`.`Forecast`.sum()".into(),
        editing: None,
    })?;
    grand_tour_layout(&mut store)?;

    let finished = output.join("grand-tour-finished.fw");
    store.save(&finished)?;

    let mut reloaded = Store::load(&finished)?;
    let sales_id = frame(&reloaded, "Monthly sales").id;
    let sales_page = reloaded.get_frame_page(&sales_id, 0, 20)?;
    assert_eq!(sales_page.total_rows, 6);
    assert_eq!(
        sales_page.rows[0],
        vec!["2026-01", "East", "118000", "76000", "42000", "127440"]
    );
    let analysis_id = frame(&reloaded, "Sales vs budget").id;
    let analysis_page = reloaded.get_frame_page(&analysis_id, 0, 20)?;
    assert_eq!(analysis_page.total_rows, 6);
    assert_eq!(
        analysis_page.rows[0],
        vec![
            "2026-01", "East", "118000", "76000", "42000", "120000", "-2000"
        ]
    );
    let summary_page = reloaded.get_frame_page(&frame(&reloaded, "By region").id, 0, 10)?;
    assert_eq!(
        summary_page.rows,
        vec![
            vec!["East", "573000", "560000", "13000"],
            vec!["West", "266000", "258000", "8000"],
        ]
    );
    assert_block_answers(&reloaded, "Scratchwork", &["839000", "21000", "906120.00"]);

    // The tour's closing claim is that switching assumptions moves the model
    // rather than a copy of it, so the generator proves it the way the reader
    // does: switch, read the same three answers, switch back.
    for (name, forecast) in [("Upside", "964850.00"), ("Downside", "797050.00")] {
        let scenario_id = reloaded
            .document()
            .scenarios
            .iter()
            .find(|scenario| scenario.name == name)
            .unwrap_or_else(|| panic!("the answer key carries the {name} scenario"))
            .id
            .clone();
        reloaded.apply(Operation::ActivateScenario {
            scenario_id: Some(scenario_id),
        })?;
        assert_block_answers(&reloaded, "Scratchwork", &["839000", "21000", forecast]);
    }
    reloaded.apply(Operation::ActivateScenario { scenario_id: None })?;
    assert_block_answers(&reloaded, "Scratchwork", &["839000", "21000", "906120.00"]);

    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

fn generate_basic(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("Your first FrameWork workbook"));
    store.apply(Operation::AddFrame {
        name: "Monthly sales".into(),
        grid: vec![
            vec!["Column 1".into(), "Column 2".into()],
            vec![String::new(), String::new()],
            vec![String::new(), String::new()],
        ],
        x: 70.0,
        y: 70.0,
    })?;
    store.apply(Operation::AddBlock {
        name: "Assumptions".into(),
        x: 70.0,
        y: 420.0,
    })?;
    add_tutorial_walkthrough(
        &mut store,
        include_str!("../../../tutorials/first-workbook/README.md"),
    )?;
    let start = output.join("first-workbook-start.fw");
    store.save(&start)?;

    let sales_id = frame(&store, "Monthly sales").id;
    store.apply(Operation::SetFrameFromPastedText {
        frame_id: sales_id.clone(),
        text: "Month\tRegion\tRevenue\tCost\n2026-04\tWest\t142000\t91000\n2026-01\tEast\t118000\t76000\n2026-06\tEast\t168000\t104000\n2026-03\tEast\t136000\t85000\n2026-02\tWest\t124000\t79000\n2026-05\tEast\t151000\t96000\n".into(),
    })?;
    let sales = frame(&store, "Monthly sales");
    let month_id = column_id_named(&sales, "Month");
    let revenue_id = column_id_named(&sales, "Revenue");
    let cost_id = column_id_named(&sales, "Cost");
    store.apply(Operation::AddComputedColumn {
        frame_id: sales.id.clone(),
        name: "Profit".into(),
        formula: "`Revenue` - `Cost`".into(),
        after_column_id: Some(cost_id.clone()),
    })?;
    let sales = frame(&store, "Monthly sales");
    let profit_id = column_id_named(&sales, "Profit");
    for column_id in [&revenue_id, &cost_id, &profit_id] {
        store.apply(Operation::SetColumnFormat {
            frame_id: sales.id.clone(),
            column_id: column_id.clone(),
            format: Some(money_format()),
        })?;
    }
    store.apply(Operation::SetFrameDisplaySort {
        frame_id: sales.id.clone(),
        keys: vec![DerivedSort {
            column_id: month_id.clone(),
            descending: false,
        }],
    })?;
    let east = branch_frame_mut(&mut store, "Monthly sales", "East only")?;
    let mut east_steps = pass_through_steps(&east);
    east_steps.push(FrameStepInput::Filter {
        predicates: vec!["`Region` == \"East\"".into()],
        match_all: true,
    });
    store.apply(Operation::SetFramePipeline {
        frame_id: east.id,
        steps: east_steps,
    })?;
    store.apply(Operation::SetBlockSource {
        block_id: block_id(&store, "Assumptions"),
        source: "Target margin = 30%\nJanuary profit = $118000 - $76000".into(),
        editing: None,
    })?;
    add_sales_narrative(&mut store)?;
    store.apply(Operation::AddPlot {
        name: "Profit by month".into(),
        source_frame_id: sales.id.clone(),
        spec: serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v6.json",
            "mark": {"type": "line", "tooltip": true, "point": true},
            "encoding": {
                "x": {"field": month_id, "type": "nominal", "title": "Month", "sort": null},
                "y": {"field": profit_id, "type": "quantitative", "title": "Profit"},
                "color": {"field": column_id_named(&sales, "Region"), "type": "nominal", "title": "Region"}
            }
        }),
        x: 1020.0,
        y: 70.0,
        view_id: None,
    })?;
    let plot_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Plot(plot) if plot.name == "Profit by month" => Some(plot.id.clone()),
            _ => None,
        })
        .expect("tutorial plot exists");
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &sales.id),
        width: 910.0,
        height: 464.0,
    })?;
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &plot_id),
        x: 1626.0,
        y: 57.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &plot_id),
        width: 360.0,
        height: 330.0,
    })?;
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &block_id(&store, "Assumptions")),
        x: 727.0,
        y: 669.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &block_id(&store, "Assumptions")),
        width: 520.0,
        height: 180.0,
    })?;
    let finished = output.join("first-workbook-finished.fw");
    store.save(&finished)?;
    let reloaded = Store::load(&finished)?;
    let page = reloaded.get_frame_page(&sales.id, 0, 20)?;
    assert_eq!(page.total_rows, 6);
    assert_eq!(page.rows[0][0], "2026-01");
    assert_eq!(page.rows[0][4], "42000");
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

fn generate_advanced(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank("Month-end close tutorial"));
    store.apply(Operation::AddFrame {
        name: "Actuals".into(),
        grid: vec![
            vec!["Key", "Month", "Region", "Revenue", "Cost"],
            vec!["2026-01-East", "2026-01", "East", "118000", "76000"],
            vec!["2026-01-West", "2026-01", "West", "110000", "72000"],
            vec!["2026-02-East", "2026-02", "East", "125000", "80000"],
            vec!["2026-02-West", "2026-02", "West", "124000", "79000"],
            vec!["2026-03-East", "2026-03", "East", "136000", "85000"],
            vec!["2026-03-West", "2026-03", "West", "129000", "83000"],
            vec!["2026-04-East", "2026-04", "East", "145000", "92000"],
            vec!["2026-04-West", "2026-04", "West", "142000", "91000"],
            vec!["2026-05-East", "2026-05", "East", "151000", "96000"],
            vec!["2026-05-West", "2026-05", "West", "148000", "94000"],
            vec!["2026-06-East", "2026-06", "East", "168000", "104000"],
            vec!["2026-06-West", "2026-06", "West", "155000", "99000"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 70.0,
        y: 70.0,
    })?;
    let actuals_start_y = 70.0;
    let budget_start_y = actuals_start_y + view_height(&store, &frame(&store, "Actuals").id) + 40.0;
    store.apply(Operation::AddFrame {
        name: "Budget".into(),
        grid: vec![
            vec!["Key", "Month", "Region", "Budget"],
            vec!["2026-01-East", "2026-01", "East", "120000"],
            vec!["2026-01-West", "2026-01", "West", "112000"],
            vec!["2026-02-East", "2026-02", "East", "122000"],
            vec!["2026-02-West", "2026-02", "West", "120000"],
            vec!["2026-03-East", "2026-03", "East", "130000"],
            vec!["2026-03-West", "2026-03", "West", "128000"],
            vec!["2026-04-East", "2026-04", "East", "140000"],
            vec!["2026-04-West", "2026-04", "West", "138000"],
            vec!["2026-05-East", "2026-05", "East", "150000"],
            vec!["2026-05-West", "2026-05", "West", "145000"],
            vec!["2026-06-East", "2026-06", "East", "160000"],
            vec!["2026-06-West", "2026-06", "West", "150000"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 70.0,
        y: budget_start_y,
    })?;
    let checks_start_y = budget_start_y + view_height(&store, &frame(&store, "Budget").id) + 40.0;
    store.apply(Operation::AddBlock {
        name: "Close checks".into(),
        x: 70.0,
        y: checks_start_y,
    })?;
    add_tutorial_walkthrough(
        &mut store,
        include_str!("../../../tutorials/month-end-close/README.md"),
    )?;
    let start = output.join("month-end-close-start.fw");
    store.save(&start)?;

    let actuals = frame(&store, "Actuals");
    let budget = frame(&store, "Budget");
    let actual_key = column_id_named(&actuals, "Key");
    let budget_key = column_id_named(&budget, "Key");
    store.apply(Operation::SetUniqueKey {
        frame_id: budget.id.clone(),
        column_ids: vec![budget_key.clone()],
        enabled: true,
    })?;
    store.apply(Operation::AddJoinFrame {
        primary_frame_id: actuals.id.clone(),
        lookup_frame_id: budget.id.clone(),
        primary_key_column_ids: vec![actual_key],
        lookup_key_column_ids: vec![budget_key],
        join_type: FrameJoinType::Left,
        columns: vec![
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: column_id_named(&actuals, "Month"),
                name: "Month".into(),
            },
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: column_id_named(&actuals, "Region"),
                name: "Region".into(),
            },
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: column_id_named(&actuals, "Revenue"),
                name: "Revenue".into(),
            },
            JoinColumnInput {
                source_frame_id: actuals.id.clone(),
                source_column_id: column_id_named(&actuals, "Cost"),
                name: "Cost".into(),
            },
            JoinColumnInput {
                source_frame_id: budget.id.clone(),
                source_column_id: column_id_named(&budget, "Budget"),
                name: "Budget".into(),
            },
        ],
        name: "Actuals vs budget".into(),
        x: 760.0,
        y: 70.0,
    })?;
    let joined = frame(&store, "Actuals vs budget");
    store.apply(Operation::SetFramePipeline {
        frame_id: joined.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![
                ExistingFormulaInput {
                    output_column_id: column_id("Profit"),
                    name: "Profit".into(),
                    formula: "`Revenue` - `Cost`".into(),
                },
                ExistingFormulaInput {
                    output_column_id: column_id("Variance"),
                    name: "Variance".into(),
                    formula: "`Revenue` - `Budget`".into(),
                },
                ExistingFormulaInput {
                    output_column_id: column_id("Variance %"),
                    name: "Variance %".into(),
                    formula: "(`Revenue` - `Budget`) / `Budget`".into(),
                },
            ],
        }],
    })?;
    let analysis = frame(&store, "Actuals vs budget");
    for name in ["Revenue", "Cost", "Budget", "Profit", "Variance"] {
        store.apply(Operation::SetColumnFormat {
            frame_id: analysis.id.clone(),
            column_id: column_id_named(&analysis, name),
            format: Some(money_format()),
        })?;
    }
    store.apply(Operation::SetColumnFormat {
        frame_id: analysis.id.clone(),
        column_id: column_id_named(&analysis, "Variance %"),
        format: Some(percent_format()),
    })?;
    store.apply(Operation::SetFrameDisplaySort {
        frame_id: analysis.id.clone(),
        keys: vec![
            DerivedSort {
                column_id: column_id_named(&analysis, "Month"),
                descending: false,
            },
            DerivedSort {
                column_id: column_id_named(&analysis, "Region"),
                descending: false,
            },
        ],
    })?;

    let summary = branch_frame_mut(&mut store, "Actuals vs budget", "Regional summary")?;
    let mut summary_steps = pass_through_steps(&summary);
    summary_steps.push(FrameStepInput::Summarize {
        group_keys: vec![ExistingFormulaInput {
            output_column_id: column_id("Region"),
            name: "Region".into(),
            formula: "`Region`".into(),
        }],
        aggregates: ["Revenue", "Budget", "Variance", "Profit"]
            .into_iter()
            .map(|name| ExistingFormulaInput {
                output_column_id: column_id(&format!("Total {name}")),
                name: format!("Total {name}"),
                formula: format!("`{name}`.sum()"),
            })
            .collect(),
        maintain_order: true,
    });
    store.apply(Operation::SetFramePipeline {
        frame_id: summary.id.clone(),
        steps: summary_steps,
    })?;

    let pivot = branch_frame_mut(&mut store, "Actuals vs budget", "Revenue by month")?;
    let pivot_region = column_id_named(&pivot, "Region");
    let pivot_month = column_id_named(&pivot, "Month");
    let pivot_revenue = column_id_named(&pivot, "Revenue");
    let mut pivot_steps = pass_through_steps(&pivot);
    pivot_steps.push(FrameStepInput::Select {
        column_ids: vec![pivot_region, pivot_month.clone(), pivot_revenue.clone()],
    });
    pivot_steps.push(FrameStepInput::Pivot {
        names_column_id: pivot_month,
        values_column_id: pivot_revenue,
        aggregate: PivotAggregate::Sum,
    });
    store.apply(Operation::SetFramePipeline {
        frame_id: pivot.id.clone(),
        steps: pivot_steps,
    })?;

    let exceptions = branch_frame_mut(&mut store, "Actuals vs budget", "Below budget")?;
    let keep = ["Month", "Region", "Revenue", "Budget", "Variance"]
        .into_iter()
        .map(|name| column_id_named(&exceptions, name))
        .collect::<Vec<_>>();
    let mut exception_steps = pass_through_steps(&exceptions);
    exception_steps.push(FrameStepInput::Select { column_ids: keep });
    exception_steps.push(FrameStepInput::Filter {
        predicates: vec!["`Variance` < 0".into()],
        match_all: true,
    });
    store.apply(Operation::SetFramePipeline {
        frame_id: exceptions.id.clone(),
        steps: exception_steps,
    })?;

    let checks = block_id(&store, "Close checks");
    store.apply(Operation::SetBlockSource {
        block_id: checks.clone(),
        source: "Total revenue = `Actuals vs budget`.`Revenue`.sum()\nTotal budget = `Actuals vs budget`.`Budget`.sum()".into(),
        editing: None,
    })?;
    // Control totals are semantic queries over the current analysis, not a
    // pair of captured cells. Keeping them live is what makes the tutorial's
    // final upstream-edit check meaningful.
    // The analyzed join and its three branch tabs share one card. Open the
    // answer key on the join itself and move the raw inputs below
    // the first viewport: the start file already introduces those inputs,
    // while the finished file should lead with what the work produced.
    let analysis_view = view_id(&store, &analysis.id);
    store.apply(Operation::MoveView {
        view_id: analysis_view.clone(),
        x: 720.0,
        y: 70.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: analysis_view.clone(),
        width: 1156.0,
        height: 470.0,
    })?;
    store.apply(Operation::AddPlot {
        name: "Actuals vs budget plot".into(),
        source_frame_id: analysis.id.clone(),
        spec: serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v6.json",
            "mark": {"type": "line", "tooltip": true},
            "encoding": {
                "x": {
                    "field": column_id_named(&analysis, "Month"),
                    "type": "nominal",
                    "title": "Month",
                    "sort": "-y"
                },
                "y": {
                    "field": column_id_named(&analysis, "Revenue"),
                    "type": "quantitative",
                    "title": "Revenue",
                    "aggregate": "sum"
                },
                "tooltip": [
                    {
                        "field": column_id_named(&analysis, "Month"),
                        "type": "nominal",
                        "title": "Month"
                    },
                    {
                        "field": column_id_named(&analysis, "Revenue"),
                        "type": "quantitative",
                        "title": "Revenue"
                    }
                ],
                "color": {
                    "field": column_id_named(&analysis, "Region"),
                    "type": "nominal",
                    "title": "Region"
                }
            },
            "title": "Revenue by region"
        }),
        x: 70.0,
        y: 70.0,
        view_id: Some(analysis_view.clone()),
    })?;
    store.apply(Operation::SetActiveTab {
        view_id: analysis_view,
        object_id: analysis.id.clone(),
    })?;
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &checks),
        x: 692.0,
        y: 547.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &checks),
        width: 983.0,
        height: 255.0,
    })?;
    // Below the checks card, not at a guessed offset: the raw inputs must
    // clear whatever height the checks block actually resized to.
    let raw_inputs_y = 547.0 + view_height(&store, &checks) + 40.0;
    for (object, x) in [(&actuals, 720.0), (&budget, 1520.0)] {
        store.apply(Operation::MoveView {
            view_id: view_id(&store, &object.id),
            x,
            y: raw_inputs_y,
        })?;
    }

    let finished = output.join("month-end-close-finished.fw");
    store.save(&finished)?;
    let reloaded = Store::load(&finished)?;
    let joined_page = reloaded.get_frame_page(&joined.id, 0, 30)?;
    assert_eq!(joined_page.total_rows, 12);
    let analysis_page = reloaded.get_frame_page(&analysis.id, 0, 30)?;
    assert_eq!(analysis_page.total_rows, 12);
    let summary_page = reloaded.get_frame_page(&summary.id, 0, 10)?;
    assert_eq!(summary_page.total_rows, 2);
    let exception_page = reloaded.get_frame_page(&exceptions.id, 0, 10)?;
    assert_eq!(exception_page.total_rows, 2);
    let pivot_page = reloaded.get_frame_page(&pivot.id, 0, 10)?;
    assert_eq!(pivot_page.total_rows, 2);
    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

fn generate_vectors_and_joins(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output)?;
    let mut store = Store::new_tutorial(Document::blank(
        "Vectors, Calculation Matrix, dates, and visual joins",
    ));
    store.apply(Operation::AddText { x: 70.0, y: 70.0 })?;
    let guide_id = text_id(&store, "Text");
    store.apply(Operation::RenameObject {
        object_id: guide_id.clone(),
        name: "Tutorial walkthrough".into(),
    })?;
    // One source for the repository guide and the card rendered inside both
    // workbooks. A copied summary would inevitably drift from the smoke test
    // it explains, which is exactly when a tutorial becomes least useful.
    store.apply(Operation::SetTextSource {
        object_id: guide_id.clone(),
        source: include_str!("../../../tutorials/vectors-and-joins/README.md").into(),
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &guide_id),
        width: 580.0,
        height: 820.0,
    })?;
    store.apply(Operation::AddFrame {
        name: "Launch inputs".into(),
        grid: vec![
            vec!["Line", "SKU", "Units", "Launch month"],
            vec!["1", "A-100", "120", "2026-09-01"],
            vec!["2", "B-200", "90", "2026-10-01"],
            vec!["3", "C-300", "75", ""],
            vec!["4", "A-100", "140", ""],
            vec!["5", "D-400", "60", ""],
            vec!["6", "B-200", "110", ""],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 720.0,
        y: 540.0,
    })?;
    let launch_inputs = frame(&store, "Launch inputs");
    store.apply(Operation::AddLinkedFrame {
        source_frame_id: launch_inputs.id.clone(),
        name: "Launch plan".into(),
        x: 720.0,
        y: 70.0,
    })?;
    store.apply(Operation::AddFrame {
        name: "Product catalog".into(),
        grid: vec![
            vec!["SKU", "Product", "Region", "Unit price"],
            vec!["A-100", "Aurora", "West", "250"],
            vec!["B-200", "Boreal", "East", "180"],
            vec!["C-300", "Cedar", "North", "320"],
            vec!["D-400", "Delta", "South", "210"],
        ]
        .into_iter()
        .map(|row| row.into_iter().map(str::to_string).collect())
        .collect(),
        x: 1410.0,
        y: 70.0,
    })?;
    store.apply(Operation::AddBlock {
        name: "Checks".into(),
        x: 1410.0,
        y: 470.0,
    })?;
    let start = output.join("vectors-and-joins-start.fw");
    store.save(&start)?;

    let launch = frame(&store, "Launch plan");
    let line_id = column_id_named(&launch_inputs, "Line");
    let launch_month_id = column_id_named(&launch_inputs, "Launch month");
    store.apply(Operation::SetFramePipeline {
        frame_id: launch.id.clone(),
        steps: vec![
            FrameStepInput::Sort {
                keys: vec![framework_core::SortInput {
                    column_id: line_id,
                    descending: false,
                }],
            },
            FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: launch_month_id,
                    name: "Launch month".into(),
                    formula: "sequence(2026-09-01, periods=frame.len(), step=1mo)".into(),
                }],
            },
        ],
    })?;

    let launch = frame(&store, "Launch plan");
    let catalog = frame(&store, "Product catalog");
    let launch_sku = column_id_named(&launch, "SKU");
    let catalog_sku = column_id_named(&catalog, "SKU");
    store.apply(Operation::SetUniqueKey {
        frame_id: catalog.id.clone(),
        column_ids: vec![catalog_sku.clone()],
        enabled: true,
    })?;
    store.apply(Operation::AddJoinFrame {
        primary_frame_id: launch.id.clone(),
        lookup_frame_id: catalog.id.clone(),
        primary_key_column_ids: vec![launch_sku],
        lookup_key_column_ids: vec![catalog_sku],
        join_type: FrameJoinType::Left,
        columns: ["Line", "SKU", "Units", "Launch month"]
            .into_iter()
            .map(|name| JoinColumnInput {
                source_frame_id: launch.id.clone(),
                source_column_id: column_id_named(&launch, name),
                name: name.into(),
            })
            .chain(
                ["Product", "Region", "Unit price"]
                    .into_iter()
                    .map(|name| JoinColumnInput {
                        source_frame_id: catalog.id.clone(),
                        source_column_id: column_id_named(&catalog, name),
                        name: name.into(),
                    }),
            )
            .collect(),
        name: "Scheduled launches".into(),
        x: 720.0,
        y: 1320.0,
    })?;
    let scheduled = frame(&store, "Scheduled launches");
    store.apply(Operation::SetFramePipeline {
        frame_id: scheduled.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: column_id("Revenue"),
                name: "Revenue".into(),
                formula: "`Units` * `Unit price`".into(),
            }],
        }],
    })?;
    let scheduled = frame(&store, "Scheduled launches");
    for name in ["Unit price", "Revenue"] {
        store.apply(Operation::SetColumnFormat {
            frame_id: scheduled.id.clone(),
            column_id: column_id_named(&scheduled, name),
            format: Some(money_format()),
        })?;
    }

    store.apply(Operation::AddVariable {
        name: "Scenario".into(),
        formula: "[\"Base\", \"Upside\", \"Downside\"]".into(),
        x: 1410.0,
        y: 560.0,
    })?;
    store.apply(Operation::AddVariable {
        name: "Multiplier".into(),
        formula: "[1, 1.15, 0.85]".into(),
        x: 1410.0,
        y: 630.0,
    })?;
    store.apply(Operation::AddVariable {
        name: "Quarter".into(),
        formula: "[\"Q1\", \"Q2\", \"Q3\", \"Q4\"]".into(),
        x: 1410.0,
        y: 700.0,
    })?;
    store.apply(Operation::AddVariable {
        name: "Base revenue".into(),
        formula: "[100, 110, 120, 130]".into(),
        x: 1410.0,
        y: 770.0,
    })?;
    // Below Launch inputs' actual card height, not the 6-row guess: that
    // card sizes itself to its row count and would otherwise clip under
    // the matrix in the same column.
    let launch_inputs = frame(&store, "Launch inputs");
    let matrix_y = 540.0 + view_height(&store, &launch_inputs.id) + 40.0;
    store.apply(Operation::AddCalculationMatrix {
        name: "Scenario × Quarter".into(),
        x: 720.0,
        y: matrix_y,
    })?;
    let calculation_matrix_id = calculation_matrix_id(&store, "Scenario × Quarter");
    store.apply(Operation::SetCalculationMatrix {
        object_id: calculation_matrix_id,
        rows: vec![
            CalculationMatrixFormulaInput {
                id: None,
                name: "Scenario".into(),
                formula: "`Scenario`".into(),
            },
            CalculationMatrixFormulaInput {
                id: None,
                name: "Multiplier".into(),
                formula: "`Multiplier`".into(),
            },
        ],
        columns: vec![
            CalculationMatrixFormulaInput {
                id: None,
                name: "Quarter".into(),
                formula: "`Quarter`".into(),
            },
            CalculationMatrixFormulaInput {
                id: None,
                name: "Base revenue".into(),
                formula: "`Base revenue`".into(),
            },
        ],
        body: "`Base revenue` * `Multiplier`".into(),
    })?;

    store.apply(Operation::SetBlockSource {
        block_id: block_id(&store, "Checks"),
        source: "Rows scheduled = `Scheduled launches`.`Line`.len()\nRevenue scheduled = `Scheduled launches`.`Revenue`.sum()".into(),
        editing: None,
    })?;
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &block_id(&store, "Checks")),
        x: 1410.0,
        y: 930.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &launch.id),
        width: 620.0,
        height: 390.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &catalog.id),
        width: 600.0,
        height: 340.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &scheduled.id),
        width: 1160.0,
        height: 430.0,
    })?;

    let finished = output.join("vectors-and-joins-finished.fw");
    store.save(&finished)?;
    let mut reloaded = Store::load(&finished)?;
    let launch_page = reloaded.get_frame_page(&launch.id, 0, 20)?;
    assert_eq!(launch_page.total_rows, 6);
    assert_eq!(launch_page.rows[0][3], "2026-09-01");
    assert_eq!(launch_page.rows[5][3], "2027-02-01");
    let matrix = reloaded
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::CalculationMatrix(matrix) if matrix.name == "Scenario × Quarter" => {
                Some(matrix)
            }
            _ => None,
        })
        .expect("the answer key contains its Calculation Matrix");
    assert_eq!(matrix.rows.len(), 2);
    assert_eq!(matrix.columns.len(), 2);
    assert_eq!(matrix.body.source, "`Base revenue` * `Multiplier`");
    let matrix_id = matrix.id.clone();
    let view = reloaded.view();
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
        let id = reloaded
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Result(result) if result.variable && result.name == name => {
                    Some(result.id.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the answer key contains the {name} variable"));
        assert_eq!(view.computed_results[&id].value_count, count);
    }
    let joined_page = reloaded.get_frame_page(&scheduled.id, 0, 20)?;
    assert_eq!(joined_page.total_rows, 6);
    assert_eq!(joined_page.rows[0][7], "30000");
    assert_block_answers(&reloaded, "Checks", &["6", "137600"]);

    // The tutorial's central promise is behavior, not merely a stored formula:
    // appending a source row grows the calendar and the visual join together.
    add_seventh_launch_row(&mut reloaded, &launch_inputs)?;
    let grown_launch = reloaded.get_frame_page(&launch.id, 0, 20)?;
    assert_eq!(grown_launch.total_rows, 7);
    assert_eq!(grown_launch.rows[6][3], "2027-03-01");
    assert_eq!(reloaded.get_frame_page(&scheduled.id, 0, 20)?.total_rows, 7);
    assert_block_answers(&reloaded, "Checks", &["7", "153600"]);

    println!("wrote {}", start.display());
    println!("wrote {}", finished.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    match std::env::args().nth(1).as_deref() {
        Some("vectors-and-joins") => {
            generate_vectors_and_joins(&workspace.join("tutorials/vectors-and-joins"))?;
            return Ok(());
        }
        Some("grand-tour") => {
            generate_grand_tour(&workspace.join("tutorials/grand-tour"))?;
            return Ok(());
        }
        _ => {}
    }
    generate_grand_tour(&workspace.join("tutorials/grand-tour"))?;
    generate_basic(&workspace.join("tutorials/first-workbook"))?;
    generate_advanced(&workspace.join("tutorials/month-end-close"))?;
    generate_vectors_and_joins(&workspace.join("tutorials/vectors-and-joins"))?;
    Ok(())
}
