use crate::common::*;
use framework_core::*;
use std::fs;
use uuid::Uuid;

#[test]
fn prepared_creation_operations_replay_with_identical_ids() {
    let document = Document::demo();
    let mut first = Store::new(document.clone());
    let mut second = Store::new(document);
    let prepared = first
        .prepare_operation(Operation::AddFrame {
            name: "Imported".into(),
            grid: vec![
                vec!["Item".into(), "Amount".into()],
                vec!["A".into(), "12".into()],
                vec!["B".into(), "30".into()],
            ],
            x: 120.0,
            y: 240.0,
        })
        .unwrap();
    let serialized = serde_json::to_string(&prepared).unwrap();
    let replayed: ReplicatedOperation = serde_json::from_str(&serialized).unwrap();

    first.apply_replicated(prepared).unwrap();
    second.apply_replicated(replayed).unwrap();
    assert_eq!(first.document(), second.document());
}

#[test]
fn operation_events_keep_formula_references_bound_to_ids() {
    let document = Document::demo();
    let frame_id = document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();
    let mut first = Store::new(document.clone());
    let mut second = Store::new(document);
    let prepared = first
        .prepare_operation(Operation::AddComputedColumn {
            frame_id,
            name: "Double quantity".into(),
            formula: "`Quantity` * 2".into(),
            after_column_id: None,
        })
        .unwrap();
    let event = OperationEvent::new(
        first.document().id.clone(),
        Uuid::new_v4().to_string(),
        1,
        VersionVector::new(),
        prepared,
    );
    let serialized = serde_json::to_string(&event).unwrap();
    let replayed: OperationEvent = serde_json::from_str(&serialized).unwrap();

    first.apply_event(&event).unwrap();
    second.apply_event(&replayed).unwrap();
    assert_eq!(first.document(), second.document());
}

#[test]
fn event_journal_merges_causal_files_and_persists_deduplication_state() {
    let directory = temporary_test_directory("event-journal");
    let path = directory.join("Shared.fw");
    let document = Document::demo();
    let document_id = document.id.clone();
    let block_id = document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();
    let writer_id = Uuid::new_v4().to_string();
    let mut origin = Store::new(document.clone());
    let first = origin
        .prepare_event(
            &writer_id,
            Operation::SetBlockSource {
                block_id: block_id.clone(),
                source: "Tax rate = 0.10".into(),
                editing: None,
            },
        )
        .unwrap();
    origin.apply_event(&first).unwrap();
    let second = origin
        .prepare_event(
            &writer_id,
            Operation::SetBlockSource {
                block_id,
                source: "Tax rate = 0.20".into(),
                editing: None,
            },
        )
        .unwrap();

    let mut replica = Store::new(document);
    replica.save(&path).unwrap();
    let journal = EventJournal::open(&path, &document_id).unwrap();
    journal.append(&second).unwrap();
    assert_eq!(
        journal.merge_into(&mut replica).unwrap(),
        MergeResult {
            applied: 0,
            pending: 1
        }
    );
    journal.append(&first).unwrap();
    assert_eq!(
        journal.merge_into(&mut replica).unwrap(),
        MergeResult {
            applied: 2,
            pending: 0
        }
    );
    assert_eq!(replica.version_vector()[&writer_id], 2);
    assert!(!replica.view().can_undo);

    replica.save(&path).unwrap();
    let mut reopened = Store::load(&path).unwrap();
    assert_eq!(
        journal.merge_into(&mut reopened).unwrap(),
        MergeResult {
            applied: 0,
            pending: 0
        }
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn event_journal_refuses_to_replace_an_existing_writer_sequence() {
    let directory = temporary_test_directory("immutable-event");
    let path = directory.join("Shared.fw");
    let store = demo_store();
    let writer_id = Uuid::new_v4().to_string();
    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();
    store.save(&path).unwrap();
    let journal = EventJournal::open(&path, &store.document().id).unwrap();
    let first = store
        .prepare_event(
            &writer_id,
            Operation::SetBlockSource {
                block_id: block_id.clone(),
                source: "Tax rate = 0.10".into(),
                editing: None,
            },
        )
        .unwrap();
    let conflicting = store
        .prepare_event(
            &writer_id,
            Operation::SetBlockSource {
                block_id,
                source: "Tax rate = 0.99".into(),
                editing: None,
            },
        )
        .unwrap();

    journal.append(&first).unwrap();
    assert!(matches!(
        journal.append(&conflicting),
        Err(CoreError::Persistence(message)) if message.contains("immutable event")
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_operations_are_rejected_before_event_publication() {
    let store = demo_store();
    let result = store.prepare_event(
        &Uuid::new_v4().to_string(),
        Operation::RenameObject {
            object_id: Uuid::new_v4().to_string(),
            name: "Missing".into(),
        },
    );
    assert!(matches!(result, Err(CoreError::ObjectNotFound)));
}

/// A journal is not allowed to lock a document shut just because it holds an
/// operation this build no longer has a name for.
///
/// Renaming an operation is a real thing that happens — `setActiveFrameView`
/// became `setActiveTab` when tabs became frames — and every event written
/// before the rename is already folded into the snapshot and the version
/// vector. Replaying one was never going to happen, so refusing to parse it
/// must not reject the whole log.
#[test]
fn a_journal_event_this_build_cannot_parse_is_skipped_once_it_is_already_applied() {
    let directory = temporary_test_directory("journal-unknown-operation");
    let path = directory.join("Renamed.fw");
    let mut store = Store::new(Document::demo());
    let writer_id = Uuid::new_v4().to_string();
    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .unwrap();

    let journal = EventJournal::open(&path, &store.document().id).unwrap();
    let event = store
        .prepare_event(
            &writer_id,
            Operation::SetBlockSource {
                block_id,
                source: "Tax rate = 0.42".into(),
                editing: None,
            },
        )
        .unwrap();
    journal.append(&event).unwrap();
    store.apply_event(&event).unwrap();
    store.save(&path).unwrap();

    // Rewrite the stored event as one from a build that named the operation
    // differently — exactly what an on-disk journal looks like after a rename.
    let event_path = CollaborationPaths::for_document(&path, &store.document().id)
        .unwrap()
        .events
        .join(&writer_id)
        .join(format!("{:020}.json", event.event_id.sequence));
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&event_path).unwrap()).unwrap();
    raw["operation"]["type"] = serde_json::Value::String("setActiveFrameView".into());
    fs::write(&event_path, serde_json::to_string(&raw).unwrap()).unwrap();

    // The snapshot already carries the edit, so the merge has nothing to do
    // and never has to understand the stale event.
    let mut reopened = Store::load(&path).unwrap();
    let merged = EventJournal::open(&path, &reopened.document().id)
        .unwrap()
        .merge_into(&mut reopened)
        .expect("a journal of already-applied events must not fail to merge");
    assert_eq!((merged.applied, merged.pending), (0, 0));

    // A replica that has *not* applied it still reports the problem rather
    // than silently dropping an operation it genuinely needed.
    let mut fresh = Store::new(reopened.document().clone());
    let unreadable = EventJournal::open(&path, &fresh.document().id)
        .unwrap()
        .merge_into(&mut fresh);
    assert!(matches!(unreadable, Err(CoreError::InvalidEvent(_))));

    fs::remove_dir_all(directory).unwrap();
}
