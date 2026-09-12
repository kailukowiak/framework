use framework_core::*;

fn store() -> Store {
    Store::new(Document::blank("Matrix"))
}

fn add_matrix(store: &mut Store) -> String {
    store
        .apply(Operation::AddCalculationMatrix {
            name: "Scenarios".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::CalculationMatrix(matrix) => Some(matrix.id.clone()),
            _ => None,
        })
        .unwrap()
}

fn input(name: &str, formula: &str) -> CalculationMatrixFormulaInput {
    CalculationMatrixFormulaInput {
        target_id: None,
        id: None,
        name: name.into(),
        formula: formula.into(),
    }
}

#[test]
fn axes_zip_with_even_repetition_then_cross_only_between_axes() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![
                input("r", "sequence(1, 5)"),
                input("factor", "sequence(10, 30, 10)"),
            ],
            columns: vec![input("c", "sequence(100, 300, 100)")],
            body: "r + factor + c".into(),
        })
        .unwrap();
    let computed = &store.view().computed_calculation_matrices[&id];
    assert_eq!(computed.row_tuples.len(), 4);
    assert_eq!(computed.column_tuples.len(), 2);
    assert_eq!(computed.row_tuples[2].values, ["3", "10"]);
    assert_eq!(computed.cells[0][0].display, "111");
    assert_eq!(computed.cells[3][1].display, "224");
    assert_eq!(computed.output.as_ref().unwrap().rows.len(), 8);
}

#[test]
fn uneven_axis_lengths_and_invalid_body_surface_without_results() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("a", "sequence(1, 4)"), input("b", "sequence(1, 3)")],
            columns: Vec::new(),
            body: "a + b".into(),
        })
        .unwrap();
    let view = store.view();
    let computed = &view.computed_calculation_matrices[&id];
    assert!(
        computed
            .error
            .as_deref()
            .unwrap()
            .contains("does not evenly divide")
    );
    assert!(computed.cells.is_empty());

    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("a", "sequence(1, 3)")],
            columns: Vec::new(),
            body: "a +".into(),
        })
        .unwrap();
    let computed = &store.view().computed_calculation_matrices[&id];
    assert!(computed.error.is_some());
    assert!(computed.row_tuples.is_empty());
}

#[test]
fn scalar_body_broadcasts_and_text_and_date_axes_keep_raw_values() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("scenario", "[\"Base\", \"Downside\"]")],
            columns: vec![input("month", "sequence(2026-01-01, 2026-03-01, 1mo)")],
            body: "42".into(),
        })
        .unwrap();
    let view = store.view();
    let computed = &view.computed_calculation_matrices[&id];
    assert_eq!(computed.row_tuples[1].values, ["Downside"]);
    assert_eq!(computed.column_tuples[0].values, ["2026-01-01"]);
    assert_eq!(computed.cells.len(), 2);
    assert!(
        computed
            .cells
            .iter()
            .flatten()
            .all(|cell| cell.display == "42")
    );
}

#[test]
fn body_formats_axis_values_with_the_method_spelling() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![
                input("Base Revenue", "[100, 200]"),
                input("Multiplier", "[1.1, 0.9]"),
            ],
            columns: vec![input("Quarter", "[\"Q1\", \"Q2\"]")],
            body: "(`Base Revenue` * `Multiplier`).round(2).cast(\"string\") + \" {}\".format(`Quarter`)".into(),
        })
        .unwrap();

    let computed = &store.view().computed_calculation_matrices[&id];
    assert_eq!(computed.cells[0][0].display, "110.0 Q1");
    assert_eq!(computed.cells[1][1].display, "180.0 Q2");
}

#[test]
fn axis_ids_survive_edits_and_undo_restores_the_matrix() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("a", "sequence(1, 3)")],
            columns: vec![],
            body: "a".into(),
        })
        .unwrap();
    let axis_id = match store.document().object(&id).unwrap() {
        DataObject::CalculationMatrix(matrix) => matrix.rows[0].id.clone(),
        _ => unreachable!(),
    };
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![CalculationMatrixFormulaInput {
                target_id: None,
                id: Some(axis_id.clone()),
                name: "amount".into(),
                formula: "sequence(2, 4)".into(),
            }],
            columns: vec![],
            body: "amount".into(),
        })
        .unwrap();
    let matrix = match store.document().object(&id).unwrap() {
        DataObject::CalculationMatrix(matrix) => matrix,
        _ => unreachable!(),
    };
    assert_eq!(matrix.rows[0].id, axis_id);
    store.undo();
    let matrix = match store.document().object(&id).unwrap() {
        DataObject::CalculationMatrix(matrix) => matrix,
        _ => unreachable!(),
    };
    assert_eq!(matrix.rows[0].name, "a");
}

#[test]
fn duplicate_axis_names_persist_as_an_inline_error() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("x", "1")],
            columns: vec![input("x", "2")],
            body: "x".into(),
        })
        .unwrap();
    let computed = &store.view().computed_calculation_matrices[&id];
    assert!(
        computed
            .error
            .as_deref()
            .unwrap()
            .contains("used more than once")
    );
}

#[test]
fn matrix_tracks_live_formula_dependencies() {
    let mut store = store();
    store
        .apply(Operation::AddVariable {
            name: "Rates".into(),
            formula: "[1, 2]".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let rates_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Rates")
        .unwrap()
        .id()
        .to_string();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("rate", "`Rates`")],
            columns: vec![],
            body: "rate * 2".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_calculation_matrices[&id].cells[1][0].display,
        "4"
    );
    let graph = store.dependency_graph(&id).unwrap();
    assert_eq!(graph.children[0].object_id, rates_id);
    store
        .apply(Operation::SetResultFormula {
            object_id: rates_id,
            formula: "[3, 4]".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_calculation_matrices[&id].cells[1][0].display,
        "8"
    );
}

#[test]
fn matrix_output_is_not_a_back_door_to_a_dependency_cycle() {
    let mut store = store();
    let id = add_matrix(&mut store);
    store
        .apply(Operation::SetCalculationMatrix {
            object_id: id.clone(),
            rows: vec![input("self", "`Scenarios`")],
            columns: vec![],
            body: "self".into(),
        })
        .unwrap();
    let computed = &store.view().computed_calculation_matrices[&id];
    assert!(computed.error.is_some());
    assert!(computed.output.is_none());
}
