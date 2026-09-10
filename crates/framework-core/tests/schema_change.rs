//! A connector refresh whose source changed shape.
//!
//! The old behaviour was all-or-nothing: one field missing from the new
//! extract and `base_polars_lazy` refused to build a scan at all, so every
//! column of the frame and every frame below it went dark over a field
//! nobody may have been reading. These tests hold the line at the column —
//! the frame materializes, the orphaned column is nulls and says why, and
//! the refresh reports what it did to the schema.

use crate::common::*;
use framework_core::*;
use std::fs;

/// Builds a linked frame over a two-field CSV, plus whatever else the test
/// wants layered on it. Returns the store, the frame id, and the directory
/// artifacts are staged into.
fn imported(name: &str, contents: &str) -> (Store, Id, std::path::PathBuf, std::path::PathBuf) {
    let directory = temporary_test_directory(name);
    let source = directory.join("source.csv");
    fs::write(&source, contents).unwrap();
    let artifacts = directory.join("artifacts");
    let mut store = Store::new(Document::blank("Schema change"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&source, &artifacts).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    (store, frame_id, directory, artifacts)
}

fn column_id(store: &Store, frame: &str, column: &str) -> Id {
    frame_named(store.document(), frame)
        .columns
        .iter()
        .find(|candidate| candidate.name == column)
        .unwrap_or_else(|| panic!("no column named {column}"))
        .id
        .clone()
}

fn column_names(store: &Store, frame: &str) -> Vec<String> {
    frame_named(store.document(), frame)
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect()
}

/// The whole point: one field goes away and the model keeps working.
///
/// `Cost` is read by a calculated column and by a frame downstream, so
/// reconciliation keeps it; the scan now supplies nulls for it rather than
/// refusing to run. Everything that never touched `Cost` — the other two
/// columns, the new one, and the derived frame's own rows — is unaffected,
/// and the failure is reported against `Cost` itself.
#[test]
fn a_vanished_source_field_empties_its_own_column_and_nothing_else() {
    let (mut store, frame_id, directory, artifacts) = imported(
        "schema-change-missing",
        "Region,Units,Cost\nNorth,3,10\nSouth,5,20\n",
    );
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Margin".into(),
            formula: "`Cost` * 2".into(),
            after_column_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame_id.clone(),
            name: "Review".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let review_id = frame_named(store.document(), "Review").id.clone();
    let cost_id = column_id(&store, "Orders", "Cost");
    let margin_id = column_id(&store, "Orders", "Margin");

    let source = directory.join("source.csv");
    fs::write(&source, "Region,Units,Channel\nEast,7,Web\nWest,9,Retail\n").unwrap();
    let replacement = create_data_artifact(&source, &artifacts).unwrap();

    // The account of the change is asked for before it lands, because it is
    // a comparison against the schema the frame still has.
    let diff = store
        .frame_source_schema_diff(&frame_id, &replacement)
        .unwrap();
    assert_eq!(diff.added, vec!["Channel".to_string()]);
    assert_eq!(
        diff.kept_missing,
        vec![(
            "Cost".to_string(),
            "still read by the Margin formula".to_string()
        )]
    );
    assert!(diff.removed.is_empty());
    assert!(diff.type_changed.is_empty());
    assert!(!diff.is_empty());

    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame_id.clone(),
            artifact: replacement,
        })
        .unwrap();

    // The frame materializes. The fields that arrived are there, the field
    // that left is a column of blanks, and the frame keeps its shape.
    assert_eq!(
        column_names(&store, "Orders"),
        vec!["Region", "Units", "Channel", "Cost", "Margin"]
    );
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![
            vec!["East", "7", "Web", "", ""],
            vec!["West", "9", "Retail", "", ""],
        ]
    );

    // The failure is recorded against the column that has it, and against
    // the calculated column now computing over nothing.
    let view = store.view();
    let errors = &view.computed_frames[&frame_id].column_errors;
    assert_eq!(
        errors.get(&cost_id).map(String::as_str),
        Some("Source field \"Cost\" is missing since the last refresh")
    );
    assert_eq!(
        errors.get(&margin_id).map(String::as_str),
        Some("Source field \"Cost\" is missing since the last refresh")
    );
    let region_id = column_id(&store, "Orders", "Region");
    assert!(!errors.contains_key(&region_id));

    // And the frame downstream still computes: reading a column that has
    // become null is an honest empty answer, not a broken frame.
    assert_eq!(
        store
            .get_frame_page(&review_id, 0, 10)
            .unwrap()
            .rows
            .iter()
            .map(|row| row[..2].to_vec())
            .collect::<Vec<_>>(),
        vec![vec!["East", "7"], vec!["West", "9"]]
    );
    assert!(view.computed_frames[&review_id].column_errors.is_empty());

    fs::remove_dir_all(directory).unwrap();
}

/// A field nobody reads simply goes, and a field that changed type says so.
/// Both are things the person refreshing wants told once, in the moment.
#[test]
fn a_refresh_reports_fields_dropped_added_and_retyped() {
    let (mut store, frame_id, directory, artifacts) = imported(
        "schema-change-report",
        "Region,Units,Notes\nNorth,3,check\nSouth,5,ok\n",
    );

    let source = directory.join("source.csv");
    fs::write(&source, "Region,Units,Channel\nNorth,3.5,Web\n").unwrap();
    let replacement = create_data_artifact(&source, &artifacts).unwrap();
    let diff = store
        .frame_source_schema_diff(&frame_id, &replacement)
        .unwrap();
    assert_eq!(diff.added, vec!["Channel".to_string()]);
    assert_eq!(diff.removed, vec!["Notes".to_string()]);
    assert!(diff.kept_missing.is_empty());
    assert_eq!(
        diff.type_changed,
        vec![("Units".to_string(), DataType::Integer, DataType::Number)]
    );

    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame_id.clone(),
            artifact: replacement,
        })
        .unwrap();
    assert_eq!(
        column_names(&store, "Orders"),
        vec!["Region", "Units", "Channel"]
    );
    assert!(
        store.view().computed_frames[&frame_id]
            .column_errors
            .is_empty()
    );

    fs::remove_dir_all(directory).unwrap();
}

/// New numbers under the same headers is the ordinary refresh, and it should
/// say nothing at all — a report that fires every time is a report nobody
/// reads on the day it matters.
#[test]
fn a_refresh_with_the_same_schema_reports_nothing() {
    let (store, frame_id, directory, artifacts) =
        imported("schema-change-quiet", "Region,Units\nNorth,3\nSouth,5\n");

    let source = directory.join("source.csv");
    fs::write(&source, "Region,Units\nEast,7\nWest,9\nSouth,11\n").unwrap();
    let replacement = create_data_artifact(&source, &artifacts).unwrap();
    let diff = store
        .frame_source_schema_diff(&frame_id, &replacement)
        .unwrap();
    assert!(diff.is_empty(), "unexpected schema report: {diff:?}");

    fs::remove_dir_all(directory).unwrap();
}
