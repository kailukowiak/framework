//! Scenarios: named sets of value-object values the whole document can be
//! switched between.
//!
//! The claim being tested is the one that makes this worth having at all —
//! that switching moves *every* answer downstream of an overridden value,
//! not just the card the number sits on — plus the ordinary lifecycle
//! obligations: each edit refuses what it should, undoes exactly, and
//! survives a round trip through a file.

#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;
use std::fs;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Scenarios".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
        scenarios: Vec::new(),
        active_scenario: None,
    })
}

fn add_value(store: &mut Store, name: &str, raw: &str) -> Id {
    let holder = a_container(store);
    store
        .apply(Operation::AddValue {
            name: name.into(),
            raw: raw.into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .id()
        .to_string()
}

fn add_scenario(store: &mut Store, name: &str, copy_from: Option<Id>) -> Id {
    store
        .apply(Operation::AddScenario {
            scenario_id: None,
            name: name.into(),
            copy_from,
        })
        .unwrap();
    store
        .document()
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap()
        .id
        .clone()
}

fn result_value(store: &Store, name: &str) -> Option<f64> {
    let id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .id()
        .to_string();
    store.view().computed_results[&id].cell.value
}

#[test]
fn scenarios_are_added_renamed_copied_and_removed_with_undo_for_each() {
    let mut store = blank_store();
    let rate = add_value(&mut store, "Growth rate", "0.05");
    let base = add_scenario(&mut store, "Upside", None);
    assert_eq!(store.document().scenarios.len(), 1);
    store.undo();
    assert!(store.document().scenarios.is_empty());
    store.redo();
    assert_eq!(store.document().scenarios[0].name, "Upside");

    store
        .apply(Operation::SetScenarioValue {
            scenario_id: base.clone(),
            value_id: rate.clone(),
            raw: Some("0.12".into()),
        })
        .unwrap();

    // Copying is how the third scenario is written: take the second and
    // change what differs.
    let downside = add_scenario(&mut store, "Downside", Some(base.clone()));
    assert_eq!(
        store.document().scenario(&downside).unwrap().values[&rate],
        "0.12"
    );

    store
        .apply(Operation::RenameScenario {
            scenario_id: downside.clone(),
            name: "  Bear case  ".into(),
        })
        .unwrap();
    assert_eq!(
        store.document().scenario(&downside).unwrap().name,
        "Bear case"
    );
    store.undo();
    assert_eq!(
        store.document().scenario(&downside).unwrap().name,
        "Downside"
    );
    store.redo();

    // Removing while it is the one being read also puts the document back
    // on the base, and undo restores both halves of that.
    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(downside.clone()),
        })
        .unwrap();
    store
        .apply(Operation::RemoveScenario {
            scenario_id: downside.clone(),
        })
        .unwrap();
    assert_eq!(store.document().scenarios.len(), 1);
    assert_eq!(store.document().active_scenario, None);
    store.undo();
    assert_eq!(store.document().active_scenario, Some(downside.clone()));
    assert_eq!(
        store.document().scenario(&downside).unwrap().values[&rate],
        "0.12"
    );
}

#[test]
fn scenario_names_are_trimmed_unique_and_never_empty() {
    let mut store = blank_store();
    add_scenario(&mut store, "Upside", None);
    let blank = store.apply(Operation::AddScenario {
        scenario_id: None,
        name: "   ".into(),
        copy_from: None,
    });
    assert!(blank.unwrap_err().to_string().contains("needs a name"));

    let duplicate = store.apply(Operation::AddScenario {
        scenario_id: None,
        name: "upside".into(),
        copy_from: None,
    });
    assert!(
        duplicate
            .unwrap_err()
            .to_string()
            .contains("already has a scenario")
    );
    assert_eq!(store.document().scenarios.len(), 1);

    // A scenario keeping its own name is not a duplicate of itself.
    let id = store.document().scenarios[0].id.clone();
    store
        .apply(Operation::RenameScenario {
            scenario_id: id,
            name: "Upside".into(),
        })
        .unwrap();
}

#[test]
fn an_override_must_parse_as_the_value_it_overrides() {
    let mut store = blank_store();
    let rate = add_value(&mut store, "Growth rate", "0.05");
    let opened = add_value(&mut store, "Opened", "2024-01-31");
    let upside = add_scenario(&mut store, "Upside", None);

    let refused = store.apply(Operation::SetScenarioValue {
        scenario_id: upside.clone(),
        value_id: rate.clone(),
        raw: Some("about a tenth".into()),
    });
    let message = refused.unwrap_err().to_string();
    assert!(message.contains("is not a valid"), "{message}");
    assert!(message.contains("Growth rate"), "{message}");
    assert!(
        store
            .document()
            .scenario(&upside)
            .unwrap()
            .values
            .is_empty()
    );

    let refused = store.apply(Operation::SetScenarioValue {
        scenario_id: upside.clone(),
        value_id: opened,
        raw: Some("31/01/2024".into()),
    });
    assert!(refused.unwrap_err().to_string().contains("use YYYY-MM-DD"));

    // Only a value takes an override; a result computes its own answer.
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddResult {
            name: "Doubled".into(),
            formula: "`Growth rate` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    let result_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Doubled")
        .unwrap()
        .id()
        .to_string();
    let refused = store.apply(Operation::SetScenarioValue {
        scenario_id: upside,
        value_id: result_id,
        raw: Some("1".into()),
    });
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("Only a value can hold a scenario override")
    );

    let missing = store.apply(Operation::SetScenarioValue {
        scenario_id: "no-such-scenario".into(),
        value_id: rate,
        raw: Some("0.1".into()),
    });
    assert!(
        missing
            .unwrap_err()
            .to_string()
            .contains("no longer in this document")
    );
}

/// The claim that makes the feature worth having: activating a scenario
/// moves the answers, and deactivating puts them back.
#[test]
fn activating_a_scenario_moves_every_answer_that_reads_an_overridden_value() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    let rate = add_value(&mut store, "Growth rate", "0.05");
    add_value(&mut store, "Revenue", "1000");
    store
        .apply(Operation::AddResult {
            name: "Growth".into(),
            formula: "`Revenue` * `Growth rate`".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    assert_eq!(result_value(&store, "Growth"), Some(50.0));

    let upside = add_scenario(&mut store, "Upside", None);
    store
        .apply(Operation::SetScenarioValue {
            scenario_id: upside.clone(),
            value_id: rate.clone(),
            raw: Some("0.12".into()),
        })
        .unwrap();
    // Written down but not switched on: the base is still what everything
    // reads.
    assert_eq!(result_value(&store, "Growth"), Some(50.0));

    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(upside.clone()),
        })
        .unwrap();
    assert_eq!(result_value(&store, "Growth"), Some(120.0));

    // The card still holds the number somebody typed; the view reports what
    // is actually being read, and says whose idea it was.
    let computed = store.view().computed_values[&rate].clone();
    assert_eq!(computed.raw, "0.12");
    assert_eq!(computed.scenario_name.as_deref(), Some("Upside"));
    assert_eq!(computed.scenario_id.as_deref(), Some(upside.as_str()));
    let DataObject::Value(stored) = store.document().object(&rate).unwrap() else {
        panic!("expected a value");
    };
    assert_eq!(stored.raw, "0.05");

    // An unoverridden value reads its own number under a scenario.
    let revenue = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Revenue")
        .unwrap()
        .id()
        .to_string();
    assert_eq!(store.view().computed_values[&revenue].scenario_name, None);

    store
        .apply(Operation::ActivateScenario { scenario_id: None })
        .unwrap();
    assert_eq!(result_value(&store, "Growth"), Some(50.0));
    store.undo();
    assert_eq!(store.document().active_scenario, Some(upside));
    assert_eq!(result_value(&store, "Growth"), Some(120.0));
}

/// The same claim one layer out: a calculated column reads a value, so a
/// scenario switch has to reach through the frame's lineage fingerprint and
/// recompute the column rather than serving the cached page.
#[test]
fn a_calculated_column_recomputes_when_the_scenario_switches() {
    let mut store = blank_store();
    let rate = add_value(&mut store, "Growth rate", "0.05");
    store
        .apply(Operation::AddFrame {
            name: "Plan".into(),
            grid: vec![vec!["Revenue".into()], vec!["1000".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Plan").id.clone();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Grown".into(),
            formula: "`Revenue` * `Growth rate`".into(),
            after_column_id: None,
        })
        .unwrap();

    let grown = |store: &Store| -> Option<f64> {
        let frame = frame_named(store.document(), "Plan");
        let row_id = frame.rows[0].id.clone();
        let column = frame
            .columns
            .iter()
            .find(|column| column.name == "Grown")
            .unwrap()
            .id
            .clone();
        store.view().computed_frames[&frame_id].rows[&row_id][&column].value
    };
    assert_eq!(grown(&store), Some(50.0));

    let upside = add_scenario(&mut store, "Upside", None);
    store
        .apply(Operation::SetScenarioValue {
            scenario_id: upside.clone(),
            value_id: rate,
            raw: Some("0.20".into()),
        })
        .unwrap();
    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(upside),
        })
        .unwrap();
    assert_eq!(grown(&store), Some(200.0));

    store.undo();
    assert_eq!(grown(&store), Some(50.0));
}

/// Editing an override while a scenario is active is as live as editing the
/// value itself — the fingerprint has to move for the override too, not just
/// for the activation.
#[test]
fn editing_the_active_scenarios_override_recomputes_immediately() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    let rate = add_value(&mut store, "Rate", "1");
    store
        .apply(Operation::AddResult {
            name: "Ten times".into(),
            formula: "`Rate` * 10".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    let upside = add_scenario(&mut store, "Upside", None);
    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(upside.clone()),
        })
        .unwrap();
    assert_eq!(result_value(&store, "Ten times"), Some(10.0));

    store
        .apply(Operation::SetScenarioValue {
            scenario_id: upside.clone(),
            value_id: rate.clone(),
            raw: Some("3".into()),
        })
        .unwrap();
    assert_eq!(result_value(&store, "Ten times"), Some(30.0));

    // Clearing an override is the way back to the base number, and undoing
    // the clear puts the override back rather than writing the base into it.
    store
        .apply(Operation::SetScenarioValue {
            scenario_id: upside.clone(),
            value_id: rate.clone(),
            raw: None,
        })
        .unwrap();
    assert_eq!(result_value(&store, "Ten times"), Some(10.0));
    store.undo();
    assert_eq!(
        store.document().scenario(&upside).unwrap().values[&rate],
        "3"
    );
    // And undoing the first override takes it off entirely.
    store.undo();
    assert!(
        store
            .document()
            .scenario(&upside)
            .unwrap()
            .values
            .is_empty()
    );
}

#[test]
fn deleting_a_value_takes_its_overrides_with_it_and_undo_brings_them_back() {
    let mut store = blank_store();
    let rate = add_value(&mut store, "Growth rate", "0.05");
    let kept = add_value(&mut store, "Revenue", "1000");
    let upside = add_scenario(&mut store, "Upside", None);
    for (value_id, raw) in [(&rate, "0.12"), (&kept, "2000")] {
        store
            .apply(Operation::SetScenarioValue {
                scenario_id: upside.clone(),
                value_id: value_id.clone(),
                raw: Some(raw.into()),
            })
            .unwrap();
    }

    store
        .apply(Operation::DeleteObject {
            object_id: rate.clone(),
        })
        .unwrap();
    let values = &store.document().scenario(&upside).unwrap().values;
    assert!(!values.contains_key(&rate));
    assert_eq!(values[&kept], "2000");

    store.undo();
    let values = &store.document().scenario(&upside).unwrap().values;
    assert_eq!(values[&rate], "0.12");
    assert_eq!(values[&kept], "2000");
}

#[test]
fn scenarios_and_the_activation_survive_a_save_and_load() {
    let directory = temporary_test_directory("scenarios-file");
    let path = directory.join("Plan.fw");
    let mut store = blank_store();
    let rate = add_value(&mut store, "Growth rate", "0.05");
    let upside = add_scenario(&mut store, "Upside", None);
    store
        .apply(Operation::SetScenarioValue {
            scenario_id: upside.clone(),
            value_id: rate.clone(),
            raw: Some("0.12".into()),
        })
        .unwrap();
    store
        .apply(Operation::ActivateScenario {
            scenario_id: Some(upside.clone()),
        })
        .unwrap();
    store.save(&path).unwrap();

    let loaded = Store::load(&path).unwrap();
    assert_eq!(loaded.document(), store.document());
    assert_eq!(loaded.document().active_scenario, Some(upside.clone()));
    assert_eq!(
        loaded.document().scenario(&upside).unwrap().values[&rate],
        "0.12"
    );
    assert_eq!(loaded.view().computed_values[&rate].raw, "0.12");

    // A document written before scenarios existed opens with none, rather
    // than refusing to open for a missing key.
    let legacy = directory.join("legacy.json");
    let mut serialized = serde_json::to_value(store.document()).unwrap();
    let object = serialized.as_object_mut().unwrap();
    object.remove("scenarios");
    object.remove("activeScenario");
    fs::write(&legacy, serde_json::to_string(&serialized).unwrap()).unwrap();
    let opened = Store::load(&legacy).unwrap();
    assert!(opened.document().scenarios.is_empty());
    assert_eq!(opened.document().active_scenario, None);

    fs::remove_dir_all(directory).unwrap();
}
