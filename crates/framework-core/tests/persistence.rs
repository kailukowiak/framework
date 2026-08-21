use crate::common::*;
use framework_core::*;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn fw_files_are_versioned_clickable_documents_with_collaboration_directories() {
    let directory = temporary_test_directory("fw-file");
    let path = directory.join("Orders.fw");
    let mut store = demo_store();
    let document_id = store.document().id.clone();

    store.save(&path).unwrap();
    let serialized: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(serialized["format"], FRAMEWORK_FILE_FORMAT);
    assert_eq!(serialized["formatVersion"], FRAMEWORK_FILE_VERSION);
    assert!(serialized.get("tutorialVersion").is_none());
    assert_eq!(serialized["document"]["id"], document_id);

    let collaboration = CollaborationPaths::for_document(&path, &document_id).unwrap();
    assert!(collaboration.events.is_dir());
    assert!(collaboration.checkpoints.is_dir());
    assert!(collaboration.sessions.is_dir());

    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::SetBlockSource {
            block_id,
            source: "Tax rate = 0.12".into(),
            editing: None,
        })
        .unwrap();
    store.save(&path).unwrap();

    let loaded = Store::load(&path).unwrap();
    assert_eq!(loaded.document(), store.document());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn legacy_json_documents_remain_readable() {
    let directory = temporary_test_directory("legacy-file");
    let path = directory.join("document.json");
    let document = Document::demo();
    fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).unwrap();

    let loaded = Store::load(&path).unwrap();
    assert_eq!(loaded.document(), Store::new(document).document());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn legacy_single_property_color_scales_promote_to_one_independent_ramp() {
    let legacy_text: FrameStyleScale = serde_json::from_value(json!({
        "property": "text",
        "low": "#315cbb",
        "mid": null,
        "high": "#b43ca8"
    }))
    .unwrap();
    assert_eq!(
        legacy_text,
        FrameStyleScale {
            text: Some(FrameStyleColorScale {
                low: "#315cbb".into(),
                mid: None,
                high: "#b43ca8".into(),
            }),
            fill: None,
        }
    );

    let written = serde_json::to_value(&legacy_text).unwrap();
    assert!(written.get("property").is_none());
    assert_eq!(written["text"]["low"], "#315cbb");
    assert!(written.get("fill").is_none());
}

#[test]
fn a_frame_card_with_no_recorded_tabs_shows_its_own_frame() {
    let mut json = serde_json::to_value(Document::demo()).unwrap();
    for view in json["views"].as_array_mut().unwrap() {
        view.as_object_mut().unwrap().remove("tabFrameIds");
    }
    let document: Document = serde_json::from_value(json).unwrap();
    let store = Store::new(document);
    let view = store
        .document()
        .views
        .iter()
        .find(|view| {
            store.document().objects.iter().any(
                |object| matches!(object, DataObject::Frame(frame) if frame.id == view.object_id),
            )
        })
        .unwrap();
    // No strip recorded means one implicit tab: the card's own frame, in the
    // standard orientation.
    assert_eq!(view.tabs(), std::slice::from_ref(&view.object_id));
    assert_eq!(
        store
            .document()
            .frame(&view.object_id)
            .unwrap()
            .display
            .orientation,
        FrameViewOrientation::RecordsAsRows
    );
}

#[test]
fn documents_without_column_formats_deserialize_with_none() {
    let document = Document::demo();
    let mut serialized = serde_json::to_value(&document).unwrap();
    for object in serialized["objects"].as_array_mut().unwrap() {
        let Some(columns) = object.get_mut("columns") else {
            continue;
        };
        for column in columns.as_array_mut().unwrap() {
            let removed = column.as_object_mut().unwrap().remove("format");
            assert!(removed.is_some());
        }
    }
    let loaded: Document = serde_json::from_value(serialized).unwrap();
    assert_eq!(loaded, document);
    assert!(loaded.objects.iter().all(|object| match object {
        DataObject::Frame(frame) => {
            frame.columns.iter().all(|column| column.format.is_none())
        }
        _ => true,
    }));
}

/// A column reference as it sits in a saved document.
///
/// Hand-written rather than parsed because these tests are about the shape
/// around the expression, and going through the parser would make them
/// depend on a frame existing to parse against. The key is `column_id`, not
/// `columnId`: `Expr` renames its variants to camel case but not the fields
/// inside them, and a document written any time in the last three years
/// says it that way.
fn saved_column(column_id: &str) -> serde_json::Value {
    json!({ "kind": "column", "column_id": column_id })
}

/// A derivation saved in the flat field layout still opens, and opens as
/// the chain it always described.
///
/// Before a derivation was a list of steps it was seven loose fields, and
/// `steps()` synthesized a chain out of them on every single read. The
/// fields are gone now and the synthesis happens once, at deserialization —
/// so what has to be defended is that it is the *same* synthesis. The order
/// is the part worth pinning: filter, then the projection with the select
/// that adopts what it minted, then the sort. Any other order is a
/// different frame.
#[test]
fn a_flat_derivation_deserializes_into_the_chain_it_described() {
    let stored = json!({
        "sourceFrameId": "orders",
        "filters": [saved_column("quantity")],
        "filterMatchAll": false,
        "projections": [{ "outputColumnId": "own-total", "expression": saved_column("total") }],
        "groupKeys": [],
        "aggregates": [],
        "sorts": [{ "columnId": "own-total", "descending": true }],
        "maintainOrder": true,
    });

    let derivation: FrameDerivation = serde_json::from_value(stored).unwrap();
    assert_eq!(derivation.source_frame_id, "orders");
    assert!(derivation.join.is_none());
    match derivation.steps.as_slice() {
        [
            FrameStep::Filter {
                predicates,
                match_all,
            },
            FrameStep::WithColumns { columns },
            FrameStep::Select { column_ids },
            FrameStep::Sort { keys },
        ] => {
            assert_eq!(predicates.len(), 1);
            assert!(!match_all, "the old match-all flag rides into the step");
            assert_eq!(columns[0].output_column_id, "own-total");
            assert_eq!(
                column_ids.as_slice(),
                ["own-total".to_string()],
                "the select adopts exactly the columns the projection minted"
            );
            assert!(keys[0].descending);
            assert_eq!(keys[0].column_id, "own-total");
        }
        other => panic!("expected filter, projection, select, sort, got {other:?}"),
    }

    // And the document written back carries only the modern shape: the
    // conversion is a migration, not a translation layer that keeps both
    // spellings alive on disk forever.
    let written = serde_json::to_value(&derivation).unwrap();
    for legacy in [
        "filters",
        "filterMatchAll",
        "projections",
        "groupKeys",
        "aggregates",
        "sorts",
        "maintainOrder",
    ] {
        assert!(
            written.get(legacy).is_none(),
            "a derivation saved now must not carry `{legacy}`"
        );
    }
    assert!(written.get("steps").is_some());
}

/// The other thing the flat layout could say: one summarize, with the
/// row-order flag it was written with.
///
/// Group keys with no aggregates was a reachable state and had to stay one
/// step, so the emptiness of `aggregates` is not what decides whether a
/// summarize appears — either list being non-empty does.
#[test]
fn flat_group_keys_and_aggregates_deserialize_into_one_summarize() {
    let stored = json!({
        "sourceFrameId": "orders",
        "filters": [],
        "filterMatchAll": true,
        "projections": [],
        "groupKeys": [{ "outputColumnId": "own-region", "expression": saved_column("region") }],
        "aggregates": [{ "outputColumnId": "own-total", "expression": saved_column("total") }],
        "sorts": [],
        "maintainOrder": false,
    });

    let derivation: FrameDerivation = serde_json::from_value(stored).unwrap();
    match derivation.steps.as_slice() {
        [
            FrameStep::Summarize {
                group_keys,
                aggregates,
                maintain_order,
            },
        ] => {
            assert_eq!(group_keys[0].output_column_id, "own-region");
            assert_eq!(aggregates[0].output_column_id, "own-total");
            assert!(
                !maintain_order,
                "the flag the frame was grouped under is part of what it means"
            );
        }
        other => panic!("expected a single summarize, got {other:?}"),
    }
}

/// A join saved beside stray flat fields is still only a join.
///
/// The old `steps()` returned the join and stopped, never looking at the
/// fields below it, so any document where both were somehow filled in has
/// been running as the join alone for its whole life. Reading it as a join
/// *and* a filter now would silently drop rows from a frame nobody edited.
#[test]
fn a_flat_join_ignores_the_fields_saved_beside_it() {
    let stored = json!({
        "sourceFrameId": "orders",
        "join": {
            "lookupFrameId": "customers",
            "primaryKeyColumnIds": ["orders-customer"],
            "lookupKeyColumnIds": ["customers-id"],
            "joinType": "left",
            "outputs": [{
                "outputColumnId": "own-name",
                "sourceFrameId": "customers",
                "sourceColumnId": "customers-name",
            }],
        },
        "filters": [saved_column("quantity")],
        "filterMatchAll": true,
        "projections": [],
        "groupKeys": [],
        "aggregates": [],
        "sorts": [{ "columnId": "own-name", "descending": false }],
        "maintainOrder": true,
    });

    let derivation: FrameDerivation = serde_json::from_value(stored).unwrap();
    assert!(
        derivation.steps.is_empty(),
        "the filter and sort saved beside a join were never part of it"
    );
    assert_eq!(
        derivation.join.as_ref().unwrap().lookup_frame_id,
        "customers"
    );
    match derivation.steps().as_ref() {
        [FrameStep::Join { join }] => assert_eq!(join.join_type, FrameJoinType::Left),
        other => panic!("a join derivation runs as exactly one join step, got {other:?}"),
    }
}

/// A derivation that already had a chain keeps it, whatever is saved beside
/// it. This is the precedence the old `steps()` had, and it is what makes
/// the two shapes safe to hold in one type.
#[test]
fn a_saved_chain_wins_over_the_flat_fields() {
    let stored = json!({
        "sourceFrameId": "orders",
        "steps": [{
            "kind": "sort",
            "keys": [{ "columnId": "own-total", "descending": true }],
        }],
        "filters": [saved_column("quantity")],
        "filterMatchAll": true,
        "projections": [],
        "groupKeys": [],
        "aggregates": [],
        "sorts": [],
        "maintainOrder": true,
    });

    let derivation: FrameDerivation = serde_json::from_value(stored).unwrap();
    match derivation.steps.as_slice() {
        [FrameStep::Sort { keys }] => assert_eq!(keys[0].column_id, "own-total"),
        other => panic!("expected the saved chain untouched, got {other:?}"),
    }
}

#[test]
#[ignore]
fn probe_open_document() {
    use std::time::Instant;

    let Ok(path) = std::env::var("FRAMEWORK_PROBE_DOCUMENT") else {
        println!("set FRAMEWORK_PROBE_DOCUMENT to a .fw path");
        return;
    };
    let started = Instant::now();
    let store = match Store::load(Path::new(&path)) {
        Ok(store) => store,
        Err(error) => {
            println!("load failed: {error}");
            return;
        }
    };
    println!("load: {:?}", started.elapsed());

    let started = Instant::now();
    let view = store.view();
    println!("view(): {:?}", started.elapsed());
    println!("document: {}", view.document.name);

    for object in &view.document.objects {
        let DataObject::Frame(frame) = object else {
            continue;
        };
        let computed = &view.computed_frames[&frame.id];
        let artifact = frame.artifact.as_ref().map(|artifact| {
            (
                artifact.row_count,
                Path::new(&artifact.path).exists(),
                artifact.path.clone(),
            )
        });
        println!(
            "\nframe {:?}: {} columns, paged={}, total_rows={:?}, derived={}, artifact={:?}",
            frame.name,
            frame.columns.len(),
            computed.paged,
            computed.total_rows,
            frame.derivation.is_some(),
            artifact,
        );
        let started = Instant::now();
        match store.get_frame_page(&frame.id, 0, 5) {
            Ok(page) => println!(
                "  page in {:?}: total_rows={}, {} rows, first={:?}",
                started.elapsed(),
                page.total_rows,
                page.rows.len(),
                page.rows.first(),
            ),
            Err(error) => println!("  page FAILED in {:?}: {error}", started.elapsed()),
        }
    }
}

/// A document and its data are one thing on disk: moving them together must
/// not cost the document its numbers, and Save As must leave a copy that
/// still works once the original is gone.
#[test]
fn documents_carry_their_data_by_relative_path_and_save_as_copies_all_of_it() {
    let original = temporary_test_directory("relative-artifacts");
    let source = original.join("ledger.csv");
    fs::write(
        &source,
        "Period,Debit\n2024-01,100\n2024-02,20\n2024-01,5\n",
    )
    .unwrap();
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Ledger".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let document_id = store.document().id.clone();
    let artifact = create_data_artifact(&source, &original.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id,
            name: "By period".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Period".into(),
                formula: "`Period`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Debit total".into(),
                formula: "`Debit`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let grouped_id = frame_named(store.document(), "By period").id.clone();
    store
        .materialize_frame(&grouped_id, &original.join("data"))
        .unwrap();

    let path = original.join("Ledger.fw");
    store.save(&path).unwrap();

    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let recorded = written["document"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|object| {
            let import = object["artifact"]["path"].as_str();
            let snapshot = object["materialization"]["artifact"]["path"].as_str();
            import.or(snapshot)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recorded.len(),
        2,
        "the import and the snapshot are both here"
    );
    for recorded in recorded {
        assert!(
            Path::new(recorded).is_relative(),
            "a path that names this machine does not survive being moved: {recorded}"
        );
    }

    // The whole thing moves — document, sidecar, data — the way a folder
    // gets dragged somewhere else or synced onto another machine.
    let moved = temporary_test_directory("relative-artifacts-moved").join("Ledger");
    fs::create_dir_all(moved.parent().unwrap()).unwrap();
    fs::rename(&original, &moved).unwrap();

    let mut reopened = Store::load(&moved.join("Ledger.fw")).unwrap();
    let page = reopened.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0][1].parse::<f64>().unwrap(), 105.0);

    // Save As, then delete everything it was copied from. What is left has
    // to stand on its own -- including the snapshot, which is data the
    // document owns just as much as the imported parquet.
    let copy_directory = temporary_test_directory("relative-artifacts-copy");
    let copy_path = copy_directory.join("Ledger copy.fw");
    reopened.save_as(&copy_path).unwrap();
    fs::remove_dir_all(moved.parent().unwrap()).unwrap();

    let copied = Store::load(&copy_path).unwrap();
    assert_eq!(copied.document().name, "Ledger copy");
    let page = copied.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(
        page.rows[0][1].parse::<f64>().unwrap(),
        105.0,
        "the copy reads its own snapshot, not one in a directory that is gone"
    );
    assert!(
        !copied.snapshot_is_stale(&grouped_id),
        "copying a document does not invalidate what it had computed"
    );
    let sidecar = CollaborationPaths::for_document(&copy_path, &document_id)
        .unwrap()
        .root
        .join("data");
    for artifact in fs::read_dir(&sidecar).unwrap() {
        assert!(artifact.unwrap().path().is_file());
    }
    assert_eq!(
        fs::read_dir(&sidecar).unwrap().count(),
        2,
        "both the imported bytes and the snapshot came along"
    );
    fs::remove_dir_all(copy_directory).unwrap();
}
