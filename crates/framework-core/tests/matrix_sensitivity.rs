use framework_core::*;

#[test]
fn dropdown_and_date_inputs_keep_their_types_and_domains() {
    let (mut store, _, _, matrix) = setup();
    for (name, formula) in [("region", "dropdown([\"\",\"North\"] )"), ("cutoff", "date_input(date(2026,1,1))")] {
        store.apply(Operation::AddVariable { name: name.into(), formula: formula.into(), x: 0.0, y: 0.0 }).unwrap();
    }
    let region = store.document().objects.iter().find(|o| o.name() == "region").unwrap().id().to_string();
    let cutoff = store.document().objects.iter().find(|o| o.name() == "cutoff").unwrap().id().to_string();
    set(&mut store, &matrix, vec![axis("r", "[\"\",\"North\",\"Invalid\"]", Some(&region))], vec![axis("c", "[date(2026,1,1),date(2027,1,1)]", Some(&cutoff))], "`region` + `cutoff`.dt.year().cast(\"string\")");
    let output = &store.view().computed_calculation_matrices[&matrix];
    assert_eq!(output.cells[0][0].display, "2026", "{output:?}");
    assert_eq!(output.cells[1][1].display, "North2027");
    assert!(output.cells[2][0].error.is_some());
}

#[test]
fn frozen_inputs_are_refused_and_recorded_outputs_stay_recorded() {
    let (mut store, growth, _, matrix) = setup();
    let directory = std::env::temp_dir().join(format!("framework-sensitivity-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let revenue = store.document().objects.iter().find(|o| o.name() == "revenue").unwrap().id().to_string();
    store.freeze_value(&revenue, &directory).unwrap();
    set(&mut store, &matrix, vec![axis("r", "[0,1]", Some(&growth))], vec![], "`revenue`");
    let output = &store.view().computed_calculation_matrices[&matrix];
    assert_eq!(output.cells[0][0].value, Some(12.0));
    assert_eq!(output.cells[1][0].value, Some(12.0));
    store.freeze_value(&growth, &directory).unwrap();
    assert!(store.view().computed_calculation_matrices[&matrix].error.as_ref().unwrap().contains("frozen"));
    std::fs::remove_dir_all(directory).unwrap();
}

fn setup() -> (Store, String, String, String) {
    let mut store = Store::new(Document::blank("Sensitivity"));
    for (name, formula) in [
        ("growth", "slider(0,1,0.1,0.2)"),
        ("price", "slider(1,100,1,10)"),
        ("revenue", "(1 + `growth`) * `price`"),
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
            name: "Sensitivity".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ids: Vec<_> = store
        .document()
        .objects
        .iter()
        .map(|object| object.id().to_string())
        .collect();
    (store, ids[0].clone(), ids[1].clone(), ids[3].clone())
}

#[test]
fn sensitivity_rebuilds_live_filtered_frames_for_each_point() {
    let (mut store, _, price, matrix) = setup();
    store.apply(Operation::AddFrame { name: "Sales".into(), grid: vec![vec!["Amount".into()],vec!["10".into()],vec!["30".into()]], x: 0.0, y: 0.0 }).unwrap();
    let source_frame_id = store.document().objects.last().unwrap().id().to_string();
    store.apply(Operation::AddLinkedFrame { source_frame_id, name: "Filtered".into(), x: 0.0, y: 0.0 }).unwrap();
    let frame_id = store.document().objects.last().unwrap().id().to_string();
    store.apply(Operation::SetFramePipeline { frame_id, steps: vec![FrameStepInput::Filter { predicates: vec!["`Amount` >= `price`".into()], match_all: true }] }).unwrap();
    store.apply(Operation::AddVariable { name: "total".into(), formula: "`Filtered`.`Amount`.sum()".into(), x: 0.0, y: 0.0 }).unwrap();
    set(&mut store, &matrix, vec![axis("r", "[10,20,50]", Some(&price))], vec![], "`total`");
    let output = &store.view().computed_calculation_matrices[&matrix];
    assert_eq!(output.cells.iter().map(|row| row[0].value).collect::<Vec<_>>(), vec![Some(40.0),Some(30.0),Some(0.0)], "{output:?}");
}

fn axis(name: &str, source: &str, target: Option<&str>) -> CalculationMatrixFormulaInput {
    CalculationMatrixFormulaInput {
        id: None,
        name: name.into(),
        formula: source.into(),
        target_id: target.map(Into::into),
    }
}

fn set(
    store: &mut Store,
    matrix: &str,
    rows: Vec<CalculationMatrixFormulaInput>,
    columns: Vec<CalculationMatrixFormulaInput>,
    body: &str,
) {
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: matrix.into(),
            rows,
            columns,
            body: body.into(),
        })
        .unwrap();
}

#[test]
fn two_input_sensitivity_recalculates_a_named_result_without_mutating_inputs() {
    let (mut store, growth, price, matrix) = setup();
    set(
        &mut store,
        &matrix,
        vec![axis("r", "[0.1,0.3]", Some(&growth))],
        vec![axis("c", "[10,20]", Some(&price))],
        "`revenue`",
    );
    let before = serde_json::to_string(store.document()).unwrap();
    let view = store.view();
    let output = &view.computed_calculation_matrices[&matrix];
    assert!(output.error.is_none(), "{output:?}");
    assert_eq!(
        output
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.value)
            .collect::<Vec<_>>(),
        vec![Some(11.0), Some(22.0), Some(13.0), Some(26.0)]
    );
    assert_eq!(output.output.as_ref().unwrap().rows.len(), 4);
    assert_eq!(serde_json::to_string(store.document()).unwrap(), before);
    let loaded: Document = serde_json::from_str(&before).unwrap();
    assert_eq!(
        Store::new(loaded).view().computed_calculation_matrices[&matrix].cells[1][1].value,
        Some(26.0)
    );
    store.undo();
    assert!(
        store.view().computed_calculation_matrices[&matrix]
            .cells
            .is_empty()
    );
    store.redo();
    assert_eq!(
        store.view().computed_calculation_matrices[&matrix].cells[1][1].value,
        Some(26.0)
    );
}

#[test]
fn invalid_points_are_local_and_duplicate_or_missing_targets_are_explicit() {
    let (mut store, growth, _, matrix) = setup();
    set(
        &mut store,
        &matrix,
        vec![axis("r", "[0.1,2,0.3]", Some(&growth))],
        vec![],
        "`revenue`",
    );
    let output = &store.view().computed_calculation_matrices[&matrix];
    assert_eq!(output.cells[0][0].value, Some(11.0));
    assert!(output.cells[1][0].error.is_some());
    assert_eq!(output.cells[2][0].value, Some(13.0));
    set(
        &mut store,
        &matrix,
        vec![axis("r", "[0.1]", Some(&growth))],
        vec![axis("c", "[0.2]", Some(&growth))],
        "`revenue`",
    );
    assert!(
        store.view().computed_calculation_matrices[&matrix]
            .error
            .as_ref()
            .unwrap()
            .contains("only one")
    );
    set(
        &mut store,
        &matrix,
        vec![axis("r", "[0.1]", Some("missing"))],
        vec![],
        "`revenue`",
    );
    assert!(
        store.view().computed_calculation_matrices[&matrix]
            .error
            .is_some()
    );
    set(
        &mut store,
        &matrix,
        vec![axis("r", "sequence(0,2501)", Some(&growth))],
        vec![],
        "`revenue`",
    );
    assert!(
        store.view().computed_calculation_matrices[&matrix]
            .error
            .as_ref()
            .unwrap()
            .contains("2,500")
    );
}

#[test]
fn sensitivity_reads_live_aggregates_and_recomputes_after_source_edits() {
    let (mut store, growth, _, matrix) = setup();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["100".into()],
                vec!["200".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddBlock {
            name: "Model".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block = store.document().objects.last().unwrap().id().to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: block,
            source: "total = `Sales`.`Amount`.sum()\nforecast = total * (1 + `growth`)".into(),
            editing: None,
        })
        .unwrap();
    set(
        &mut store,
        &matrix,
        vec![axis("r", "[0,1]", Some(&growth))],
        vec![],
        "`Model`.`forecast`",
    );
    let output = &store.view().computed_calculation_matrices[&matrix];
    assert_eq!(output.cells[1][0].value, Some(600.0), "{output:?}");
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| {
            if let DataObject::Frame(frame) = object {
                Some(frame.clone())
            } else {
                None
            }
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id,
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[0].id.clone(),
            raw: "300".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_calculation_matrices[&matrix].cells[1][0].value,
        Some(1000.0)
    );
}
