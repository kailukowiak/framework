//! Evaluate under overrides: `under`, `solve`, and the matrix's evaluation
//! count.
//!
//! The document under test is the *Scenarios, sensitivity and goal seek*
//! tutorial's model, built the way the lesson builds it, so the numbers
//! here are the lesson's checkpoints. The claim that matters most is the
//! property the finance plan names as this phase's gate: for every scenario,
//! `under(scenario, x)` equals activating that scenario and reading `x` —
//! and the document is left exactly as it was.

#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;

fn value_id(store: &Store, name: &str) -> Id {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap_or_else(|| panic!("{name} exists"))
        .id()
        .to_string()
}

fn scenario_id(store: &Store, name: &str) -> Id {
    store
        .document()
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap()
        .id
        .clone()
}

fn set_block(store: &mut Store, name: &str, source: &str) -> Result<(), CoreError> {
    let block_id = value_id(store, name);
    store.apply(Operation::SetBlockSource {
        block_id,
        source: source.into(),
        editing: None,
    })?;
    Ok(())
}

fn line(store: &Store, block: &str, name: &str) -> ComputedBlockLine {
    let block_id = value_id(store, block);
    store.view().computed_blocks[&block_id]
        .lines
        .iter()
        .find(|line| line.name == name)
        .unwrap_or_else(|| panic!("line {name} exists"))
        .clone()
}

fn line_value(store: &Store, block: &str, name: &str) -> f64 {
    let line = line(store, block, name);
    assert!(line.cell.error.is_none(), "{name}: {:?}", line.cell.error);
    line.cell
        .value
        .unwrap_or_else(|| panic!("{name} has a value"))
}

/// The lesson's model: four assumptions, two scenarios, a twelve-month plan
/// with the price applied inside the table, and a block that sums it.
fn lesson() -> Store {
    let mut store = Store::new(Document::blank("Scenarios"));
    store
        .apply(Operation::AddContainer {
            name: "Assumptions".into(),
            x: 0.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    let assumptions = value_id(&store, "Assumptions");
    for (name, raw) in [
        ("Price", "120"),
        ("Annual units", "8000"),
        ("Unit cost", "70"),
        ("Fixed costs", "250000"),
    ] {
        store
            .apply(Operation::AddValue {
                name: name.into(),
                raw: raw.into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(assumptions.clone()),
            })
            .unwrap();
    }
    for (scenario, values) in [
        ("Upside", [("Price", "125"), ("Annual units", "9500")]),
        ("Downside", [("Price", "115"), ("Annual units", "6500")]),
    ] {
        store
            .apply(Operation::AddScenario {
                scenario_id: None,
                name: scenario.into(),
                copy_from: None,
            })
            .unwrap();
        let id = scenario_id(&store, scenario);
        for (value, raw) in values {
            store
                .apply(Operation::SetScenarioValue {
                    scenario_id: id.clone(),
                    value_id: value_id(&store, value),
                    raw: Some(raw.into()),
                })
                .unwrap();
        }
    }
    let weights = [6, 6, 7, 8, 9, 9, 9, 9, 8, 9, 10, 10];
    let mut grid = vec![vec!["Month".to_string(), "Weight".to_string()]];
    for (index, weight) in weights.iter().enumerate() {
        grid.push(vec![format!("M{}", index + 1), weight.to_string()]);
    }
    store
        .apply(Operation::AddFrame {
            name: "Plan".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let plan = value_id(&store, "Plan");
    let step = |name: &str, formula: &str| FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: format!("col-{}", name.to_lowercase()),
            name: name.into(),
            formula: formula.into(),
        }],
    };
    store
        .apply(Operation::SetFramePipeline {
            frame_id: plan,
            steps: vec![
                step("Units", "`Annual units` * `Weight` / 100"),
                step("Revenue", "`Units` * `Price`"),
                step("Cost", "`Units` * `Unit cost`"),
            ],
        })
        .unwrap();
    store
        .apply(Operation::AddBlock {
            name: "Model".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    set_block(
        &mut store,
        "Model",
        "revenue = `Plan`.`Revenue`.sum()\ngross = revenue - `Plan`.`Cost`.sum()\nebitda = gross - `Fixed costs`\nmargin = ebitda / revenue",
    )
    .unwrap();
    store
}

#[test]
fn under_reads_a_scenario_without_switching_the_document_into_it() {
    let mut store = lesson();
    assert_eq!(line_value(&store, "Model", "ebitda"), 150000.0);
    set_block(
        &mut store,
        "Model",
        "revenue = `Plan`.`Revenue`.sum()\ngross = revenue - `Plan`.`Cost`.sum()\nebitda = gross - `Fixed costs`\nmargin = ebitda / revenue\nupside ebitda = under(`Upside`, ebitda)\ndownside ebitda = under(`Downside`, ebitda)\nby name = under(\"downside\", revenue)",
    )
    .unwrap();
    assert_eq!(line_value(&store, "Model", "upside ebitda"), 272500.0);
    assert_eq!(line_value(&store, "Model", "downside ebitda"), 42500.0);
    assert_eq!(line_value(&store, "Model", "by name"), 747500.0);
    // The base is untouched: the document is still reading its own cards.
    assert_eq!(line_value(&store, "Model", "ebitda"), 150000.0);
    assert_eq!(store.document().active_scenario, None);
    // And the line reads back the way it was written, scenario name and all.
    let text = line(&store, "Model", "upside ebitda").text;
    assert_eq!(text, "upside ebitda = under(`Upside`, ebitda)");
}

/// The gate: for every scenario, `under(scenario, x)` equals activating
/// that scenario and reading `x`, across a block line, a result and a
/// frame summary, and evaluating leaves the document byte-for-byte as it
/// was.
#[test]
fn under_equals_activating_the_scenario_and_reading_the_answer() {
    let mut store = lesson();
    store
        .apply(Operation::AddVariable {
            name: "Margin result".into(),
            formula: "(`Price` - `Unit cost`) * `Annual units`".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddBlock {
            name: "Probe".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let scenarios: Vec<(String, String)> = store
        .document()
        .scenarios
        .iter()
        .map(|scenario| (scenario.id.clone(), scenario.name.clone()))
        .collect();
    // A compact variable reading a live line cannot itself be inlined by a
    // block line today — that is the freeze boundary, and not this
    // phase's — so the variable probe reads values only.
    let probes = [
        "`Model`.ebitda",
        "`Model`.margin",
        "`Margin result`",
        "`Plan`.`Revenue`.sum()",
        "`Plan`.`Units`.max()",
    ];
    for (scenario_id, scenario) in &scenarios {
        let source = probes
            .iter()
            .enumerate()
            .map(|(index, probe)| format!("p{index} = under(`{scenario}`, {probe})"))
            .collect::<Vec<_>>()
            .join("\n");
        set_block(&mut store, "Probe", &source).unwrap();
        let before = serde_json::to_string(store.document()).unwrap();
        let under: Vec<f64> = (0..probes.len())
            .map(|index| line_value(&store, "Probe", &format!("p{index}")))
            .collect();
        assert_eq!(serde_json::to_string(store.document()).unwrap(), before);

        let direct = probes
            .iter()
            .enumerate()
            .map(|(index, probe)| format!("p{index} = {probe}"))
            .collect::<Vec<_>>()
            .join("\n");
        set_block(&mut store, "Probe", &direct).unwrap();
        store
            .apply(Operation::ActivateScenario {
                scenario_id: Some(scenario_id.clone()),
            })
            .unwrap();
        let activated: Vec<f64> = (0..probes.len())
            .map(|index| line_value(&store, "Probe", &format!("p{index}")))
            .collect();
        store
            .apply(Operation::ActivateScenario { scenario_id: None })
            .unwrap();
        assert_eq!(under, activated, "{scenario}");
    }
}

#[test]
fn under_holds_the_scenario_in_place_and_follows_a_rename() {
    let mut store = lesson();
    set_block(
        &mut store,
        "Model",
        "ebitda = `Plan`.`Revenue`.sum() - `Plan`.`Cost`.sum() - `Fixed costs`\nup = under(`Upside`, ebitda)",
    )
    .unwrap();
    let upside = scenario_id(&store, "Upside");
    let refusal = store
        .apply(Operation::RemoveScenario {
            scenario_id: upside.clone(),
        })
        .unwrap_err()
        .to_string();
    assert!(
        refusal.contains("Model") && refusal.contains("Upside"),
        "{refusal}"
    );
    store
        .apply(Operation::RenameScenario {
            scenario_id: upside,
            name: "Bull".into(),
        })
        .unwrap();
    // The formula held the id, so it reads the same scenario by its new
    // name; the stored text keeps the author's spelling, as it does for a
    // renamed value.
    assert_eq!(line_value(&store, "Model", "up"), 272500.0);

    let unknown = store.apply(Operation::SetBlockSource {
        block_id: value_id(&store, "Model"),
        source: "x = under(`Sideways`, 1)".into(),
        editing: None,
    });
    // An unknown backticked name is unknown; a string names the scenarios.
    let message = match unknown {
        Err(error) => error.to_string(),
        Ok(_) => line(&store, "Model", "x").cell.error.unwrap_or_default(),
    };
    assert!(message.contains("Sideways"), "{message}");
    set_block(&mut store, "Model", "x = under(\"Sideways\", 1)").unwrap();
    let error = line(&store, "Model", "x").cell.error.unwrap_or_default();
    assert!(
        error.contains("‘Bull’") && error.contains("‘Downside’"),
        "{error}"
    );
}

#[test]
fn solve_finds_the_price_that_hits_the_target_and_apply_is_an_ordinary_edit() {
    let mut store = lesson();
    set_block(
        &mut store,
        "Model",
        "revenue = `Plan`.`Revenue`.sum()\nebitda = revenue - `Plan`.`Cost`.sum() - `Fixed costs`\ntarget price = solve(ebitda == 300000, by=`Price`, within=[100, 200])",
    )
    .unwrap();
    let solved = line(&store, "Model", "target price");
    assert!(solved.cell.error.is_none(), "{:?}", solved.cell.error);
    let report = solved.solve.clone().expect("a whole-line solve reports");
    assert!((report.answer - 138.75).abs() < 1e-6, "{}", report.answer);
    assert_eq!(report.answer_raw, "138.75");
    assert_eq!(report.target_name, "Price");
    assert!(report.iterations > 0);
    assert!(report.residual.abs() < 1e-3, "{}", report.residual);
    assert!(report.evaluations >= report.iterations + 2);
    assert_eq!((report.low, report.high), (100.0, 200.0));
    // Nothing moved.
    assert_eq!(line_value(&store, "Model", "ebitda"), 150000.0);

    // Apply is the ordinary edit, so undo reaches it.
    store
        .apply(Operation::SetValue {
            object_id: report.target_id.clone(),
            raw: report.answer_raw.clone(),
        })
        .unwrap();
    assert!((line_value(&store, "Model", "ebitda") - 300000.0).abs() < 1e-6);
    store.undo();
    assert_eq!(line_value(&store, "Model", "ebitda"), 150000.0);

    // Inside arithmetic, solve is just its number.
    set_block(
        &mut store,
        "Model",
        "revenue = `Plan`.`Revenue`.sum()\nebitda = revenue - `Plan`.`Cost`.sum() - `Fixed costs`\nuplift = solve(ebitda == 300000, by=`Price`, within=[100, 200]) - `Price`",
    )
    .unwrap();
    let uplift = line(&store, "Model", "uplift");
    assert!(uplift.solve.is_none());
    assert!((uplift.cell.value.unwrap() - 18.75).abs() < 1e-6);
}

#[test]
fn solve_refuses_a_bracket_without_a_crossing_and_a_value_it_cannot_vary() {
    let mut store = lesson();
    let model = "revenue = `Plan`.`Revenue`.sum()\nebitda = revenue - `Plan`.`Cost`.sum() - `Fixed costs`\n";
    for (formula, expected) in [
        (
            "solve(ebitda == 300000, by=`Price`, within=[100, 120])",
            "same side of zero",
        ),
        (
            "solve(ebitda == 300000, by=`Price`, within=[200, 100])",
            "low before high",
        ),
        ("solve(ebitda == 300000, by=`Price`)", "within="),
        (
            "solve(ebitda, by=`Price`, within=[100, 200])",
            "what should equal what",
        ),
        (
            "solve(ebitda == 300000, by=`Plan`.`Revenue`.sum(), within=[100, 200])",
            "by= names the value",
        ),
        (
            "solve(`Fixed costs` == 1, by=`Price`, within=[100, 200])",
            "Nothing in the comparison reads ‘Price’",
        ),
    ] {
        set_block(&mut store, "Model", &format!("{model}x = {formula}")).unwrap();
        let error = line(&store, "Model", "x").cell.error.unwrap_or_default();
        assert!(error.contains(expected), "{formula}: {error}");
    }
}

#[test]
fn a_sensitivity_matrix_reports_how_many_evaluations_it_ran() {
    let mut store = lesson();
    for (name, formula) in [
        ("Price axis", "[100, 110, 120, 130, 140]"),
        ("Units axis", "[6000, 7000, 8000, 9000, 10000]"),
    ] {
        store
            .apply(Operation::AddVariable {
                name: name.into(),
                formula: formula.into(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }
    store
        .apply(Operation::AddCalculationMatrix {
            name: "Grid".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let axis = |name: &str, target: &str| CalculationMatrixFormulaInput {
        target_id: Some(value_id(&store, target)),
        id: None,
        name: name.into(),
        formula: format!("`{name}`"),
    };
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: value_id(&store, "Grid"),
            rows: vec![axis("Price axis", "Price")],
            columns: vec![axis("Units axis", "Annual units")],
            body: "`Model`.ebitda".into(),
        })
        .unwrap();
    let grid_id = value_id(&store, "Grid");
    let view = store.view();
    let computed = &view.computed_calculation_matrices[&grid_id];
    assert_eq!(computed.error, None);
    assert_eq!(computed.evaluations, Some(25));
    assert!(computed.elapsed_ms.is_some());
    let corner = |row: usize, column: usize| computed.cells[row][column].value.unwrap();
    assert_eq!(corner(0, 0), -70000.0);
    assert_eq!(corner(2, 2), 150000.0);
    assert_eq!(corner(4, 4), 450000.0);
}
