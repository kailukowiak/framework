//! How far undo reaches, by what each step costs to keep.
//!
//! Ordinary edits — typing, formatting, moving cards — are cheap to keep, so
//! a long run of them stays undoable. An edit that changes which data file
//! the document reads pins a file version for as long as it is reachable,
//! and those are held to a short tail. Whatever is dropped is always the
//! oldest, so what undo reaches is one unbroken run back from now.

use crate::common::*;
use framework_core::*;
use std::fs;
use std::path::Path;

fn first_view(store: &Store) -> Id {
    store.document().views[0].id.clone()
}

fn move_to(store: &mut Store, view_id: &str, x: f64) {
    store
        .apply(Operation::MoveView {
            view_id: view_id.into(),
            x,
            y: 0.0,
        })
        .unwrap();
}

fn x_of(store: &Store, view_id: &str) -> f64 {
    store.document().view(view_id).unwrap().x
}

fn undo_everything(store: &mut Store) -> usize {
    let mut steps = 0;
    while store.view().can_undo {
        store.undo();
        steps += 1;
        assert!(steps < 10_000, "undo never ran out");
    }
    steps
}

#[test]
fn a_hundred_ordinary_edits_stay_undoable() {
    let mut store = demo_store();
    let view_id = first_view(&store);
    for step in 1..=150 {
        move_to(&mut store, &view_id, step as f64);
    }
    assert_eq!(undo_everything(&mut store), 100);
    // The oldest fifty went; the run that is left starts right after them.
    assert_eq!(x_of(&store, &view_id), 50.0);

    // Redo walks the same hundred back.
    let mut redone = 0;
    while store.view().can_redo {
        store.redo();
        redone += 1;
    }
    assert_eq!(redone, 100);
    assert_eq!(x_of(&store, &view_id), 150.0);
}

fn artifact(directory: &Path, name: &str, text: &str) -> DataArtifact {
    let path = directory.join(name);
    fs::write(&path, text).unwrap();
    create_data_artifact(&path, &directory.join("data")).unwrap()
}

/// Twelve refreshes, each followed by a cheap move. Only ten refreshes may
/// stay reachable, so the two oldest go — and with them every cheap edit
/// older than the oldest refresh that stays, because undo cannot skip an
/// edit in the middle of the stack. The cheap edits after it all survive.
#[test]
fn edits_that_pin_data_files_are_held_to_ten_and_evicted_oldest_first() {
    let directory = temporary_test_directory("undo-budget");
    let first = artifact(&directory, "a.csv", "Item,Count\nA,1\n");
    let second = artifact(&directory, "b.csv", "Item,Count\nA,1\nB,2\n");
    let mut store = demo_store();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact: first.clone(),
            // Refreshable, which is what a run of refreshes needs.
            connector: Some(ConnectorRecipe::File {
                source_path: directory.join("a.csv").display().to_string(),
            }),
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Stock").id.clone();
    let view_id = first_view(&store);

    let reading = |store: &Store| {
        store
            .document()
            .frame(&frame_id)
            .unwrap()
            .artifact
            .as_ref()
            .unwrap()
            .id
            .clone()
    };
    for step in 0..12 {
        let next = if step % 2 == 0 { &second } else { &first };
        store
            .apply(Operation::RefreshFrameArtifact {
                frame_id: frame_id.clone(),
                artifact: next.clone(),
            })
            .unwrap();
        move_to(&mut store, &view_id, 10.0 * (step + 1) as f64);
    }

    // The import, the first two refreshes and the move between them can no
    // longer be undone; what is left starts at the move after the second
    // refresh — twenty-one edits — so undoing all of it lands just after
    // that refresh, with the first move still in place.
    assert_eq!(undo_everything(&mut store), 21);
    assert_eq!(
        reading(&store),
        first.id,
        "the second refresh still applies"
    );
    assert_eq!(x_of(&store, &view_id), 10.0);

    let mut redone = 0;
    while store.view().can_redo {
        store.redo();
        redone += 1;
    }
    assert_eq!(redone, 21);
    assert_eq!(reading(&store), first.id);
    assert_eq!(x_of(&store, &view_id), 120.0);
}
