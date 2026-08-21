use framework_core::{
    ColumnFormat, ColumnFormatScale, ColumnFormatStyle, DataObject, DerivedSort, Document,
    ExistingFormulaInput, FrameJoinType, FrameStepInput, JoinColumnInput, Operation,
    PivotAggregate, Store, column_id,
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

fn money_format() -> ColumnFormat {
    ColumnFormat {
        style: ColumnFormatStyle::Accounting,
        decimals: Some(0),
        scale: ColumnFormatScale::Units,
        negative_parens: Some(true),
        zero_dash: Some(true),
        currency_code: Some("USD".into()),
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
    }
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
    store.apply(Operation::AddText { x: 620.0, y: 669.0 })?;
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
        x: 976.0,
        y: 57.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &plot_id),
        width: 360.0,
        height: 330.0,
    })?;
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &block_id(&store, "Assumptions")),
        x: 77.0,
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
        y: 480.0,
    })?;
    store.apply(Operation::AddBlock {
        name: "Close checks".into(),
        x: 70.0,
        y: 850.0,
    })?;
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
        x: 70.0,
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
    for (object, x) in [(&actuals, 70.0), (&budget, 870.0)] {
        store.apply(Operation::MoveView {
            view_id: view_id(&store, &object.id),
            x,
            y: 800.0,
        })?;
    }
    store.apply(Operation::MoveView {
        view_id: view_id(&store, &checks),
        x: 42.0,
        y: 547.0,
    })?;
    store.apply(Operation::ResizeView {
        view_id: view_id(&store, &checks),
        width: 983.0,
        height: 255.0,
    })?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    generate_basic(&workspace.join("tutorials/first-workbook"))?;
    generate_advanced(&workspace.join("tutorials/month-end-close"))?;
    Ok(())
}
