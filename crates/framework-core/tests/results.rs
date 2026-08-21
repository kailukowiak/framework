#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Results".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    })
}

fn add_value(store: &mut Store, name: &str, raw: &str) {
    let holder = a_container(store);
    store
        .apply(Operation::AddValue {
            name: name.into(),
            raw: raw.into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
}

fn result_named<'a>(store: &'a Store, name: &str) -> &'a ResultObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Result(result) if result.name == name => Some(result),
            _ => None,
        })
        .unwrap()
}

fn computed_result(store: &Store, name: &str) -> ComputedResult {
    let id = result_named(store, name).id.clone();
    store.view().computed_results.get(&id).cloned().unwrap()
}

/// The spec's `= DownPayment / PurchasePrice`: a result is a formula on the
/// canvas, and the answer is worked out from what the values hold now.
#[test]
fn a_result_computes_from_canvas_values_and_stays_live() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    add_value(&mut store, "Down payment", "50000");
    add_value(&mut store, "Purchase price", "200000");
    store
        .apply(Operation::AddResult {
            name: "Down payment percentage".into(),
            formula: "`Down payment` / `Purchase price`".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();

    let computed = computed_result(&store, "Down payment percentage");
    assert_eq!(computed.cell.value, Some(0.25));
    assert_eq!(
        computed.formula,
        "`Holder`.`Down payment` / `Holder`.`Purchase price`"
    );
    assert!(computed.cell.error.is_none());

    // Live: retype an input and the next view holds the new answer.
    let value_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Down payment")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetValue {
            object_id: value_id,
            raw: "100000".into(),
        })
        .unwrap();
    assert_eq!(
        computed_result(&store, "Down payment percentage")
            .cell
            .value,
        Some(0.5)
    );
}

/// A result may read another result; the chain compiles into one
/// expression. A result may not read itself, however many results the loop
/// passes through on the way back.
#[test]
fn results_chain_and_cycles_are_refused_by_name() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    add_value(&mut store, "Base", "10");
    store
        .apply(Operation::AddResult {
            name: "Doubled".into(),
            formula: "`Base` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::AddResult {
            name: "Quadrupled".into(),
            formula: "`Doubled` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    assert_eq!(computed_result(&store, "Quadrupled").cell.value, Some(40.0));

    let doubled_id = result_named(&store, "Doubled").id.clone();
    let cycle = store.apply(Operation::SetResultFormula {
        object_id: doubled_id,
        formula: "`Quadrupled` / 2".into(),
    });
    assert!(
        matches!(cycle, Err(CoreError::Formula(message)) if message.contains("itself")),
        "a result reaching itself through another result is refused"
    );
}

/// What holds a value in place holds it in place from a result too: a value
/// a result reads cannot be deleted, and a result a formula reads cannot
/// either. Renaming stays free because references travel by id.
#[test]
fn references_from_results_protect_and_rename_like_any_other() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    add_value(&mut store, "Rate", "0.05");
    store
        .apply(Operation::AddResult {
            name: "Doubled rate".into(),
            formula: "`Rate` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();

    let value_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Rate")
        .unwrap()
        .id()
        .to_string();
    // And it says which formula, because the alternative is somebody
    // searching their own document for something they cannot see.
    let refused = store.apply(Operation::DeleteObject {
        object_id: value_id.clone(),
    });
    let Err(CoreError::ReferencedByFormula(message)) = refused else {
        panic!("deleting a value a result reads is refused");
    };
    assert!(message.contains("Doubled rate"), "{message}");
    assert!(message.contains("Rate"), "{message}");

    store
        .apply(Operation::RenameObject {
            object_id: value_id,
            name: "Interest rate".into(),
        })
        .unwrap();
    let computed = computed_result(&store, "Doubled rate");
    assert_eq!(computed.formula, "`Holder`.`Interest rate` * 2");
    assert_eq!(computed.cell.value, Some(0.1));
}

/// A result folds a live or materialized frame's column to one number, and
/// follows current data rather than the afternoon the formula was written.
#[test]
fn a_result_aggregates_a_live_or_materialized_frame_column() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["100".into()],
                vec!["20".into()],
                vec!["5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Orders")
        .unwrap()
        .id()
        .to_string();
    // Only a derived frame can hold a snapshot, so derive one — grouped by
    // Amount, which keeps all three distinct rows.
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: frame_id.clone(),
            name: "Amounts".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Amount".into(),
                formula: "`Amount`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Rows".into(),
                formula: "`Amount`.count()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = frame_named(store.document(), "Amounts").id.clone();

    // A semantic aggregate reads the live derived frame directly. Liveness
    // is not a reason to pin an answer; freezing remains an explicit history
    // choice, and materialization remains an explicit performance choice.
    store
        .apply(Operation::AddResult {
            name: "Total".into(),
            formula: "`Amounts`.`Amount`.sum()".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    assert_eq!(computed_result(&store, "Total").cell.value, Some(125.0));

    let orders = frame_named(store.document(), "Orders").clone();
    store
        .apply(Operation::SetCell {
            frame_id,
            row_id: orders.rows[0].id.clone(),
            column_id: orders.columns[0].id.clone(),
            raw: "200".into(),
        })
        .unwrap();
    assert_eq!(computed_result(&store, "Total").cell.value, Some(225.0));

    let directory = temporary_test_directory("results-aggregate");
    // Writing the answer down now pins the current total, without changing
    // the live frame it came from.
    let total_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Total")
        .unwrap()
        .id()
        .to_string();
    store
        .freeze_value(&total_id, &directory.join("data"))
        .unwrap();
    assert_eq!(computed_result(&store, "Total").cell.value, Some(225.0));
    assert!(!computed_result(&store, "Total").frozen.unwrap().stale);

    // Letting it go returns to the current live answer; materializing the
    // derived frame afterwards produces the same value.
    store.thaw_value(&total_id).unwrap();
    assert_eq!(computed_result(&store, "Total").cell.value, Some(225.0));
    store
        .materialize_frame(&derived_id, &directory.join("data"))
        .unwrap();
    assert_eq!(computed_result(&store, "Total").cell.value, Some(225.0));

    // A bare list is still not a result; the refusal explains the fold.
    let unfolded = store.apply(Operation::AddResult {
        name: "All amounts".into(),
        formula: "`Amounts`.`Amount`".into(),
        x: 0.0,
        y: 0.0,
        container_id: Some(holder.clone()),
    });
    assert!(matches!(unfolded, Err(CoreError::Formula(_))));
}

/// Undo puts the previous formula back, exactly as SetSeries does for a
/// list's values.
#[test]
fn editing_a_result_formula_is_undoable() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    add_value(&mut store, "Base", "10");
    store
        .apply(Operation::AddResult {
            name: "Answer".into(),
            formula: "`Base` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let result_id = result_named(&store, "Answer").id.clone();
    store
        .apply(Operation::SetResultFormula {
            object_id: result_id,
            formula: "`Base` * 3".into(),
        })
        .unwrap();
    assert_eq!(computed_result(&store, "Answer").cell.value, Some(30.0));
    store.undo();
    assert_eq!(computed_result(&store, "Answer").cell.value, Some(20.0));
    assert_eq!(
        computed_result(&store, "Answer").formula,
        "`Holder`.`Base` * 2"
    );
}
