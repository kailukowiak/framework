//! Undo as inverse operations rather than document snapshots.
//!
//! The two properties that changed: a remote edit no longer voids the undo
//! stack, and undoing reverts one edit rather than every difference between
//! two snapshots. The rest is the ordinary contract — that an inverse puts
//! back exactly what its edit took, including the things a naive
//! `AddColumn` + `SetCells` pair would quietly lose.

use crate::common::*;
use framework_core::*;
use std::fs;
use uuid::Uuid;

fn frame_id(store: &Store, name: &str) -> Id {
    frame_named(store.document(), name).id.clone()
}

fn cell(store: &Store, frame_id: &str, row: usize, column: usize) -> String {
    let frame = store.document().frame(frame_id).unwrap();
    frame.rows[row].cells[&frame.columns[column].id].raw.clone()
}

/// Somebody else's edit, arriving the way one really does: through the
/// shared journal, which is the only path that marks an event as not this
/// writer's to undo.
fn merge_remote_rename(store: &mut Store, directory: &std::path::Path, name: &str, sequence: u64) {
    let path = directory.join("shared.fw");
    store.save(&path).unwrap();
    let journal = EventJournal::open(&path, &store.document().id.clone()).unwrap();
    journal
        .append(&OperationEvent::new(
            store.document().id.clone(),
            REMOTE_WRITER.to_string(),
            sequence,
            VersionVector::new(),
            ReplicatedOperation::RenameDocument { name: name.into() },
        ))
        .unwrap();
    journal.merge_into(store).unwrap();
}

/// Fixed so a test's remote events share one writer and stay in sequence.
const REMOTE_WRITER: &str = "6f1f7a4e-0b6a-4a1e-9a1a-2c9d3b5e7f01";

/// A remote event landing between an edit and its undo used to clear the
/// stack outright, so undo silently died in any collaborative session.
#[test]
fn a_remote_edit_no_longer_voids_the_undo_stack() {
    let mut store = demo_store();
    let orders = frame_id(&store, "Orders");
    let before = cell(&store, &orders, 0, 0);

    store
        .apply(Operation::SetCell {
            frame_id: orders.clone(),
            row_id: store.document().frame(&orders).unwrap().rows[0].id.clone(),
            column_id: store.document().frame(&orders).unwrap().columns[0]
                .id
                .clone(),
            raw: "41".into(),
        })
        .unwrap();
    assert!(store.view().can_undo);

    // Somebody else renames the document. Nothing to do with the cell.
    let directory = temporary_test_directory("undo-remote");
    merge_remote_rename(&mut store, &directory, "Theirs", 1);

    assert!(
        store.view().can_undo,
        "a remote edit must not take the local undo stack with it"
    );
    store.undo();
    assert_eq!(cell(&store, &orders, 0, 0), before);
    // And undo reverted the local edit only — the remote rename stands.
    assert_eq!(store.document().name, "Theirs");
    std::fs::remove_dir_all(directory).unwrap();
}

/// Snapshot undo reverted every difference between two documents. An
/// inverse touches only what its own edit touched.
#[test]
fn undo_reverts_one_edit_and_leaves_later_ones_alone() {
    let mut store = demo_store();
    let orders = frame_id(&store, "Orders");
    let row = store.document().frame(&orders).unwrap().rows[0].id.clone();
    let first_column = store.document().frame(&orders).unwrap().columns[0]
        .id
        .clone();
    let second_column = store.document().frame(&orders).unwrap().columns[1]
        .id
        .clone();
    let second_before = cell(&store, &orders, 0, 1);

    store
        .apply(Operation::SetCell {
            frame_id: orders.clone(),
            row_id: row.clone(),
            column_id: first_column,
            raw: "41".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: orders.clone(),
            row_id: row,
            column_id: second_column,
            raw: "9.75".into(),
        })
        .unwrap();

    store.undo();
    assert_eq!(cell(&store, &orders, 0, 1), second_before);
    assert_eq!(
        cell(&store, &orders, 0, 0),
        "41",
        "undoing the second edit must not reach back into the first"
    );

    store.redo();
    assert_eq!(cell(&store, &orders, 0, 1), "9.75");
}

/// Deleting a column takes its summaries and its cells' one-off overrides
/// with it. `AddColumn` + `SetCells` would put back a column that had
/// quietly lost both, which is why the inverse restores the frame.
#[test]
fn undoing_a_column_delete_restores_its_summary_and_cell_overrides() {
    let mut store = demo_store();
    let orders = frame_id(&store, "Orders");
    // Total is the demo's summarised column, and nothing reads it, so it
    // is the one that can be dropped.
    let total = store.document().frame(&orders).unwrap().columns[2]
        .id
        .clone();
    let row = store.document().frame(&orders).unwrap().rows[0].id.clone();

    store
        .apply(Operation::SetCellOverride {
            frame_id: orders.clone(),
            row_id: row.clone(),
            column_id: total.clone(),
            formula: Some("41 + 1".into()),
        })
        .unwrap();
    let summaries = store.document().frame(&orders).unwrap().summaries.clone();
    assert!(summaries.iter().any(|summary| summary.column_id == total));

    store
        .apply(Operation::DeleteColumn {
            frame_id: orders.clone(),
            column_id: total.clone(),
        })
        .unwrap();
    assert!(
        !store
            .document()
            .frame(&orders)
            .unwrap()
            .summaries
            .iter()
            .any(|summary| summary.column_id == total)
    );

    store.undo();

    let restored = store.document().frame(&orders).unwrap();
    assert!(restored.columns.iter().any(|column| column.id == total));
    assert_eq!(restored.summaries, summaries);
    assert!(
        restored
            .rows
            .iter()
            .find(|candidate| candidate.id == row)
            .unwrap()
            .cells[&total]
            .override_formula
            .is_some(),
        "the one-off override went with the column and has to come back"
    );
}

/// Deleting an object removes the cards that showed it, so the inverse has
/// to bring the canvas back as well as the object.
#[test]
fn undoing_an_object_delete_restores_its_card() {
    let mut store = demo_store();
    let regions = frame_id(&store, "Regions");
    let view = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == regions)
        .unwrap()
        .clone();

    store
        .apply(Operation::DeleteObject {
            object_id: regions.clone(),
        })
        .unwrap();
    assert!(store.document().frame(&regions).is_err());
    assert!(
        !store
            .document()
            .views
            .iter()
            .any(|candidate| candidate.id == view.id)
    );

    store.undo();

    assert!(store.document().frame(&regions).is_ok());
    let restored = store.document().view(&view.id).unwrap();
    assert_eq!((restored.x, restored.y), (view.x, view.y));
    assert_eq!(restored.object_id, view.object_id);
}

/// Tidying moves every window at once, so undoing it puts every window back
/// — one edit in, one edit out.
#[test]
fn undoing_a_tidy_restores_every_windows_position() {
    let mut store = demo_store();
    let before: Vec<(Id, u64, u64)> = store
        .document()
        .views
        .iter()
        .map(|view| (view.id.clone(), view.x.to_bits(), view.y.to_bits()))
        .collect();

    store.apply(Operation::TidyLayout).unwrap();
    assert_ne!(
        before,
        store
            .document()
            .views
            .iter()
            .map(|view| (view.id.clone(), view.x.to_bits(), view.y.to_bits()))
            .collect::<Vec<_>>()
    );

    store.undo();

    assert_eq!(
        before,
        store
            .document()
            .views
            .iter()
            .map(|view| (view.id.clone(), view.x.to_bits(), view.y.to_bits()))
            .collect::<Vec<_>>()
    );
}

/// A branched tab adds an object and rearranges a strip; undoing it has to
/// do both, or the card is left showing a frame that no longer exists.
#[test]
fn undoing_a_branched_tab_removes_it_and_reselects_the_original() {
    let mut store = demo_store();
    let orders = frame_id(&store, "Orders");
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == orders)
        .unwrap()
        .id
        .clone();

    store
        .apply(Operation::BranchFrame {
            view_id: view_id.clone(),
            frame_id: orders.clone(),
        })
        .unwrap();
    assert_eq!(store.document().view(&view_id).unwrap().tabs().len(), 2);

    store.undo();

    let view = store.document().view(&view_id).unwrap();
    assert_eq!(view.tabs(), std::slice::from_ref(&orders));
    assert_eq!(view.object_id, orders);
    assert_eq!(
        store.document().objects.len(),
        demo_store().document().objects.len()
    );
}

/// Redo replays the edit itself, so a round trip has to land exactly where
/// it started — including for the edits whose inverse is a restore.
#[test]
fn undo_then_redo_round_trips_a_structural_edit() {
    let mut store = demo_store();
    let orders = frame_id(&store, "Orders");
    let total = store.document().frame(&orders).unwrap().columns[2]
        .id
        .clone();

    store
        .apply(Operation::DeleteColumn {
            frame_id: orders.clone(),
            column_id: total.clone(),
        })
        .unwrap();
    let after_delete = store.document().frame(&orders).unwrap().clone();

    store.undo();
    assert!(
        store
            .document()
            .frame(&orders)
            .unwrap()
            .columns
            .iter()
            .any(|column| column.id == total)
    );

    store.redo();
    assert_eq!(store.document().frame(&orders).unwrap(), &after_delete);
}

/// Undo is only ever the local writer's. Events merged in from the shared
/// journal give this replica nothing to undo — you cannot undo somebody
/// else's edit, only stop your own from being lost by it.
#[test]
fn events_merged_from_the_journal_are_not_undoable() {
    let directory = temporary_test_directory("undo-merge");
    let path = directory.join("shared.fw");
    let document = Document::demo();
    let document_id = document.id.clone();
    let mut store = Store::new(document);
    store.save(&path).unwrap();

    let journal = EventJournal::open(&path, &document_id).unwrap();
    journal
        .append(&OperationEvent::new(
            document_id,
            Uuid::new_v4().to_string(),
            1,
            VersionVector::new(),
            ReplicatedOperation::RenameDocument {
                name: "Theirs".into(),
            },
        ))
        .unwrap();
    journal.merge_into(&mut store).unwrap();

    assert_eq!(store.document().name, "Theirs");
    assert!(!store.view().can_undo);
    std::fs::remove_dir_all(directory).unwrap();
}

/// Undo reaches back ten edits, not forever. Every step still reachable is a
/// version of the data the document has to be able to reproduce.
#[test]
fn undo_stops_ten_edits_back() {
    let mut store = demo_store();
    let block_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) => Some(block.id.clone()),
            _ => None,
        })
        .expect("the demo has a block to edit");

    for step in 0..14 {
        store
            .apply(Operation::SetBlockSource {
                block_id: block_id.clone(),
                source: format!("Tax rate = {step}"),
                editing: None,
            })
            .unwrap();
    }
    for _ in 0..14 {
        store.undo();
    }
    let view = store.view();
    assert!(!view.can_undo, "the stack is empty, not endless");
    assert_eq!(
        view.computed_blocks[&block_id].source, "Tax rate = 3",
        "ten steps back from the fourteenth edit, and no further"
    );
}

/// An artifact-backed frame ("Rows", columns Name/Score/Region) with an
/// empty step chain, plus the three column ids a Filter/Select chain needs.
/// Shared by the two tests below, which reproduce the desktop menu bug from
/// the user's chair — does the engine's own `can_undo` flag go true after a
/// chain edit lands on an imported frame? — from opposite ends: one after
/// each step, one after undoing them.
fn artifact_backed_pipeline_fixture(
    directory_name: &str,
) -> (std::path::PathBuf, Store, Id, Id, Id, Id) {
    let directory = temporary_test_directory(directory_name);
    let source = directory.join("rows.csv");
    fs::write(
        &source,
        "Name,Score,Region\nA,10,East\nB,20,West\nC,30,East\n",
    )
    .unwrap();

    let mut store = Store::new(Document::blank("Pipeline undo"));
    let artifact = create_data_artifact(&source, &directory.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Rows".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    assert!(
        frame.artifact.is_some(),
        "the fixture must be artifact-backed for this to reproduce the reported bug"
    );
    let name_id = frame.columns[0].id.clone();
    let score_id = frame.columns[1].id.clone();
    let region_id = frame.columns[2].id.clone();
    (directory, store, frame.id, name_id, score_id, region_id)
}

/// `SetFramePipeline` (Filter, Rearrange = `Select` reordered, Delete
/// columns = `Select` with one left out) resolves to `SetFrameSteps`
/// regardless of whether the frame's rows are typed in or read from an
/// imported artifact — `Store::apply_replicated_with_history` pushes every
/// successful edit onto the undo stack the same way, and `DocumentView`
/// reports `can_undo` from that stack alone, never from the row source. If
/// this ever fails, the defect has moved into the engine and the desktop
/// menu-sync fix in `src-tauri` is no longer sufficient on its own.
#[test]
fn chain_edits_on_an_artifact_backed_frame_each_enable_undo() {
    let (directory, mut store, frame_id, name_id, score_id, region_id) =
        artifact_backed_pipeline_fixture("artifact-pipeline-undo-enables");
    let filter = || FrameStepInput::Filter {
        predicates: vec!["`Score` > 5".into()],
        match_all: true,
    };

    // A Filter step, same as the reported repro.
    let view = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![filter()],
        })
        .unwrap();
    assert!(view.can_undo, "a filter step must enable undo");

    // A Rearrange-columns step (Select, same columns reordered).
    let view = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![
                filter(),
                FrameStepInput::Select {
                    column_ids: vec![region_id.clone(), name_id.clone(), score_id.clone()],
                },
            ],
        })
        .unwrap();
    assert!(view.can_undo, "a rearrange-columns step must enable undo");

    // A Delete-columns step (Select, one column left out).
    let view = store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![
                filter(),
                FrameStepInput::Select {
                    column_ids: vec![region_id.clone(), name_id.clone(), score_id.clone()],
                },
                FrameStepInput::Select {
                    column_ids: vec![region_id, name_id],
                },
            ],
        })
        .unwrap();
    assert!(view.can_undo, "a delete-columns step must enable undo");
    assert_eq!(
        frame_named(&view.document, "Rows").steps.len(),
        3,
        "all three chain edits must have landed"
    );

    fs::remove_dir_all(directory).unwrap();
}

/// Undo reverts a chain edit on an artifact-backed frame the same way it
/// reverts a `SetCell` on an owned one: one step at a time, per-edit, with
/// the frame's own row source (and the earlier steps) left alone — the same
/// contract `undo_reverts_one_edit_and_leaves_later_ones_alone` proves above.
#[test]
fn undo_reverts_one_chain_edit_at_a_time_on_an_artifact_backed_frame() {
    let (directory, mut store, frame_id, name_id, score_id, region_id) =
        artifact_backed_pipeline_fixture("artifact-pipeline-undo-reverts");
    let filter = || FrameStepInput::Filter {
        predicates: vec!["`Score` > 5".into()],
        match_all: true,
    };
    let rearranged = || FrameStepInput::Select {
        column_ids: vec![region_id.clone(), name_id.clone(), score_id.clone()],
    };

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![filter()],
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![filter(), rearranged()],
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id,
            steps: vec![
                filter(),
                rearranged(),
                FrameStepInput::Select {
                    column_ids: vec![region_id, name_id],
                },
            ],
        })
        .unwrap();

    // Undo reverts the delete-columns step only.
    let view = store.undo();
    let after_first_undo = frame_named(&view.document, "Rows");
    assert_eq!(
        after_first_undo.steps.len(),
        2,
        "undo must remove exactly the last chain edit"
    );
    assert!(
        after_first_undo.artifact.is_some(),
        "undoing a chain edit must not touch the frame's own row source"
    );
    assert!(
        view.can_undo,
        "two more chain edits (and the import) are still behind this one"
    );

    // Undo again removes the rearrange step, leaving only the filter.
    let view = store.undo();
    assert_eq!(
        frame_named(&view.document, "Rows").steps.len(),
        1,
        "the second undo must remove the rearrange step and stop there"
    );

    fs::remove_dir_all(directory).unwrap();
}
