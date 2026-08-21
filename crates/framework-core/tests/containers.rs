use crate::common::*;
use framework_core::*;

fn object_id_named(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .id()
        .to_string()
}

fn container_named<'a>(store: &'a Store, name: &str) -> &'a ContainerObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Container(container) if container.name == name => Some(container),
            _ => None,
        })
        .unwrap()
}

fn column_values(store: &Store, frame_id: &str, name: &str) -> Vec<String> {
    let page = store.get_frame_page(frame_id, 0, 1000).unwrap();
    let index = page
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    page.rows.iter().map(|row| row[index].clone()).collect()
}

/// A frame, and a container holding a rate and a list of currencies — the
/// "Finance" block someone would draw in a spreadsheet with a merged cell.
fn finance_store() -> (Store, String) {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Finance".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Loans".into(),
            grid: vec![
                vec!["Currency".into(), "Principal".into()],
                vec!["USD".into(), "100".into()],
                vec!["JPY".into(), "200".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Loans").id.clone();
    store
        .apply(Operation::AddContainer {
            name: "Finance".into(),
            x: 400.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddValue {
            name: "Interest rate".into(),
            raw: "0.05".into(),
            x: 400.0,
            y: 300.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::AddSeries {
            name: "Currencies".into(),
            values: "USD, CAD, EUR".into(),
            x: 400.0,
            y: 500.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    for member in ["Interest rate", "Currencies"] {
        let object_id = object_id_named(&store, member);
        store
            .apply(Operation::MoveIntoContainer {
                object_id,
                container_id: Some(object_id_named(&store, "Finance")),
            })
            .unwrap();
    }
    (store, frame_id)
}

/// The arrangement that makes a canvas legible is the arrangement you can
/// write down: `Finance`.`Interest rate` reads the value kept under that
/// heading, and the same for a list.
#[test]
fn a_container_can_be_named_through_to_what_it_holds() {
    let (mut store, frame_id) = finance_store();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Interest".into(),
            formula: "`Principal` * `Finance`.`Interest rate`".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame_id, "Interest"),
        vec!["5", "10"]
    );

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Supported".into(),
            formula: "`Currency`.is_in(`Finance`.`Currencies`)".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame_id, "Supported"),
        vec!["true", "false"]
    );

    // Both read back as what was written.
    let view = store.view();
    let formulas = &view.computed_frames[&frame_id].formulas;
    assert!(
        formulas
            .values()
            .any(|formula| formula == "`Principal` * `Finance`.`Interest rate`"),
        "{formulas:?}"
    );
}

/// A container is a place, not a value, and each step of a name looks only
/// at what that container holds — which is what makes two Rates in two
/// containers two different numbers rather than an ambiguity.
#[test]
fn each_step_of_a_name_looks_only_inside_the_container_before_it() {
    let (mut store, frame_id) = finance_store();
    store
        .apply(Operation::AddContainer {
            name: "Ops".into(),
            x: 800.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddValue {
            name: "Interest rate".into(),
            raw: "0.20".into(),
            x: 800.0,
            y: 300.0,
            container_id: Some(object_id_named(&store, "Ops")),
        })
        .unwrap();

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Ops interest".into(),
            formula: "`Principal` * `Ops`.`Interest rate`".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame_id, "Ops interest"),
        vec!["20", "40"]
    );

    // Naming the container alone is not naming a value.
    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Nope".into(),
            formula: "`Principal` * `Finance`".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("is a container"), "{error}");

    // Nor is naming something it does not hold.
    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "Nope".into(),
            formula: "`Finance`.`Headcount`".into(),
            after_column_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("holds nothing called"), "{error}");
}

/// A container holds containers, and a name walks all the way down. How
/// deep is not fixed, because the test for "does this dot continue the
/// name" is the same at every step.
#[test]
fn containers_nest_and_a_name_walks_the_whole_way_down() {
    let (mut store, frame_id) = finance_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddContainer {
            name: "Rates".into(),
            x: 800.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddValue {
            name: "Prime".into(),
            raw: "0.08".into(),
            x: 800.0,
            y: 300.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::MoveIntoContainer {
            object_id: object_id_named(&store, "Prime"),
            container_id: Some(object_id_named(&store, "Rates")),
        })
        .unwrap();
    store
        .apply(Operation::MoveIntoContainer {
            object_id: object_id_named(&store, "Rates"),
            container_id: Some(object_id_named(&store, "Finance")),
        })
        .unwrap();

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Prime interest".into(),
            formula: "`Principal` * `Finance`.`Rates`.`Prime`".into(),
            after_column_id: None,
        })
        .unwrap();
    assert_eq!(
        column_values(&store, &frame_id, "Prime interest"),
        vec!["8", "16"]
    );
}

/// Moving is one act: it leaves wherever it was as part of arriving, so
/// nothing is ever in two containers. Undo puts both back.
#[test]
fn moving_between_containers_leaves_the_first_and_undoes_as_one() {
    let (mut store, _) = finance_store();
    store
        .apply(Operation::AddContainer {
            name: "Ops".into(),
            x: 800.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    let rate = object_id_named(&store, "Interest rate");
    store
        .apply(Operation::MoveIntoContainer {
            object_id: rate.clone(),
            container_id: Some(object_id_named(&store, "Ops")),
        })
        .unwrap();
    assert_eq!(container_named(&store, "Finance").member_ids.len(), 1);
    assert_eq!(
        container_named(&store, "Ops").member_ids,
        vec![rate.clone()]
    );

    store.undo();
    assert_eq!(container_named(&store, "Finance").member_ids.len(), 2);
    assert!(container_named(&store, "Ops").member_ids.is_empty());

    // Out onto the canvas is the one move it cannot make: a loose value has
    // no home there. It stays where it was, which is the point of refusing
    // rather than dropping it somewhere.
    let refused = store
        .apply(Operation::MoveIntoContainer {
            object_id: rate.clone(),
            container_id: None,
        })
        .unwrap_err()
        .to_string();
    assert!(refused.contains("formula block"), "{refused}");
    assert_eq!(
        store
            .document()
            .container_of(&rate)
            .map(|held| held.id.clone()),
        Some(object_id_named(&store, "Finance"))
    );
}

/// Adding something to a heading is one act, not an add followed by a
/// move. And deleting it takes it out of the heading rather than leaving a
/// name pointing at nothing — which undo puts back, heading and all.
#[test]
fn something_can_be_created_inside_a_container_and_leaves_no_trace_when_deleted() {
    let (mut store, _) = finance_store();
    let finance = object_id_named(&store, "Finance");
    store
        .apply(Operation::AddValue {
            name: "Fee".into(),
            raw: "12".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(finance.clone()),
        })
        .unwrap();
    let fee = object_id_named(&store, "Fee");
    assert!(container_named(&store, "Finance").member_ids.contains(&fee));
    assert!(!store.document().is_on_canvas(&fee));

    store
        .apply(Operation::DeleteObject {
            object_id: fee.clone(),
        })
        .unwrap();
    assert!(
        !container_named(&store, "Finance").member_ids.contains(&fee),
        "a deleted member leaves the container too"
    );

    store.undo();
    assert!(
        container_named(&store, "Finance").member_ids.contains(&fee),
        "and comes back to where it lived"
    );
}

/// A container inside itself is a shape with no bottom, at any depth.
/// Frames stay out for a different reason: one already has a card and a
/// place it belongs, and being inside something else would make "where
/// does this live" have two answers.
#[test]
fn a_container_cannot_hold_itself_and_a_frame_cannot_go_in_one() {
    let (mut store, frame_id) = finance_store();
    let finance = object_id_named(&store, "Finance");
    assert!(matches!(
        store.apply(Operation::MoveIntoContainer {
            object_id: finance.clone(),
            container_id: Some(finance.clone()),
        }),
        Err(CoreError::InvalidOperation(_))
    ));

    store
        .apply(Operation::AddContainer {
            name: "Inner".into(),
            x: 800.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    let inner = object_id_named(&store, "Inner");
    store
        .apply(Operation::MoveIntoContainer {
            object_id: inner.clone(),
            container_id: Some(finance.clone()),
        })
        .unwrap();
    let error = store
        .apply(Operation::MoveIntoContainer {
            object_id: finance,
            container_id: Some(inner),
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("inside itself"), "{error}");

    let error = store
        .apply(Operation::MoveIntoContainer {
            object_id: frame_id,
            container_id: Some(object_id_named(&store, "Finance")),
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("can go in a container"), "{error}");
}
