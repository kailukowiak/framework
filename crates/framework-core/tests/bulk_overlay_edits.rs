//! Bulk cell edits on a frame whose rows live in a file.
//!
//! A single typed value over a file-backed frame has been a patch against the
//! base read for a while; the range gestures — clearing a selection, ⌘D and ⌘R,
//! cut, a pasted block, a formula-bar commit over a range — send `SetCells` or
//! `PasteCells`, and those looked the row up among the document's own rows,
//! which such a frame has none of. So the frame accepted one edit and refused
//! the same edit made twenty at a time. These tests hold the two together:
//! wherever `SetCell` works, the bulk forms work, as one undoable entry, and
//! undo restores the patches exactly — including the difference between a
//! cell nobody touched and a cell somebody cleared.

use crate::common::*;
use framework_core::*;
use std::{fs, path::Path};

fn open_file(dir: &Path, text: &[u8]) -> (Store, FrameObject) {
    let path = dir.join("stock.csv");
    fs::write(&path, text).unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: Some(path.display().to_string()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    (store, frame)
}

fn page(store: &Store, frame_id: &str) -> FramePage {
    store.get_frame_page(frame_id, 0, 1000).unwrap()
}

fn column(store: &Store, frame_id: &str, index: usize) -> Vec<String> {
    page(store, frame_id)
        .rows
        .into_iter()
        .map(|row| row[index].clone())
        .collect()
}

fn update(row_id: &str, column_id: &str, raw: &str) -> CellUpdate {
    CellUpdate {
        row_id: row_id.into(),
        column_id: column_id.into(),
        raw: raw.into(),
    }
}

#[test]
fn set_cells_on_a_file_backed_frame_is_one_undoable_patch_set() {
    let dir = temporary_test_directory("bulk-overlay-set-cells");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\nC,3\n");
    let count = frame.columns[1].id.clone();
    let ids = page(&store, &frame.id).row_ids;
    let bytes_before = fs::read(&frame.artifact.as_ref().unwrap().path).unwrap();

    // The single-cell form is the baseline the bulk form must match: row B
    // carries a correction before the range edit begins.
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: ids[1].clone(),
            column_id: count.clone(),
            raw: "20".into(),
        })
        .unwrap();

    // Clearing a range: A was never patched, B was. Both become patches *to*
    // nothing, and C is overwritten.
    store
        .apply(Operation::SetCells {
            frame_id: frame.id.clone(),
            cells: vec![
                update(&ids[0], &count, ""),
                update(&ids[1], &count, ""),
                update(&ids[2], &count, "30"),
            ],
        })
        .expect("a range edit is accepted wherever a single edit is");
    assert_eq!(column(&store, &frame.id, 1), ["", "", "30"]);
    assert_eq!(
        fs::read(&frame.artifact.as_ref().unwrap().path).unwrap(),
        bytes_before,
        "the file is untouched; the edit is a set of patches"
    );

    // One undo takes back the whole range, and restores each cell's prior
    // patch state exactly: A back to no patch (the base value, not an empty
    // one), B back to its own correction.
    store.undo();
    assert_eq!(column(&store, &frame.id, 1), ["1", "20", "3"]);
    let restored = frame_named(store.document(), "Stock");
    assert_eq!(restored.cell_overlay.len(), 1);
    assert_eq!(
        restored.cell_overlay[0]
            .entries
            .iter()
            .map(|entry| (entry.row_ordinal, entry.raw.as_str()))
            .collect::<Vec<_>>(),
        [(1, "20")],
        "only B's correction survives; A has no patch at all"
    );

    store.redo();
    assert_eq!(column(&store, &frame.id, 1), ["", "", "30"]);
    store.undo();
    store.undo();
    assert!(
        frame_named(store.document(), "Stock")
            .cell_overlay
            .is_empty(),
        "and undoing the first correction too leaves no patch behind"
    );
}

#[test]
fn a_cleared_patch_and_no_patch_survive_undo_as_different_things() {
    let dir = temporary_test_directory("bulk-overlay-presence");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\n");
    let count = frame.columns[1].id.clone();
    let ids = page(&store, &frame.id).row_ids;

    // A is cleared on its own — a patch to nothing.
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: ids[0].clone(),
            column_id: count.clone(),
            raw: "".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCells {
            frame_id: frame.id.clone(),
            cells: vec![update(&ids[0], &count, "5"), update(&ids[1], &count, "6")],
        })
        .unwrap();
    assert_eq!(column(&store, &frame.id, 1), ["5", "6"]);
    store.undo();
    assert_eq!(
        column(&store, &frame.id, 1),
        ["", "2"],
        "A goes back to cleared, not to the base's 1; B goes back to the base"
    );
}

#[test]
fn a_paste_on_a_file_backed_frame_writes_its_rectangle_and_grows_past_the_end() {
    let dir = temporary_test_directory("bulk-overlay-paste");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\nC,3\n");
    let sku = frame.columns[0].id.clone();
    let ids = page(&store, &frame.id).row_ids;

    store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: ids[1].clone(),
            column_id: sku,
            grid: vec![
                vec!["b".into(), "22".into()],
                vec!["c".into(), "33".into()],
                vec!["d".into(), "44".into()],
            ],
        })
        .expect("a paste is accepted wherever a typed value is");
    assert_eq!(
        page(&store, &frame.id).rows,
        [["A", "1"], ["b", "22"], ["c", "33"], ["d", "44"]],
        "the block lands from the anchor, and the row it ran past is added"
    );
    let pasted = frame_named(store.document(), "Stock");
    assert_eq!(pasted.appended_rows, 1);

    store.undo();
    let undone = frame_named(store.document(), "Stock");
    assert_eq!(undone.appended_rows, 0, "one undo takes the added row too");
    assert!(undone.cell_overlay.is_empty());
    assert_eq!(
        page(&store, &frame.id).rows,
        [["A", "1"], ["B", "2"], ["C", "3"]]
    );
}

/// The pager counted an unsorted file-backed frame from the file alone, so a
/// row added past the end was recorded and never shown. Held separately from
/// the paste test because `AddRow` is the plainer way to reach it.
#[test]
fn a_row_added_past_the_end_is_on_the_page() {
    let dir = temporary_test_directory("bulk-overlay-added-row-page");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\n");
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: Default::default(),
            before_row_id: None,
        })
        .unwrap();
    let added = page(&store, &frame.id);
    assert_eq!(added.total_rows, 3);
    assert_eq!(added.row_ids.len(), 3);
}

/// A row struck out is not part of the result, so a pasted block steps over
/// it exactly as it would over a row that was never there — the rectangle
/// is the rows somebody can see, in the order the file holds them.
#[test]
fn a_paste_steps_over_a_struck_out_row() {
    let dir = temporary_test_directory("bulk-overlay-paste-deleted");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\nC,3\n");
    let count = frame.columns[1].id.clone();
    let ids = page(&store, &frame.id).row_ids;
    store
        .apply(Operation::DeleteRow {
            frame_id: frame.id.clone(),
            row_id: ids[1].clone(),
        })
        .unwrap();
    store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: ids[0].clone(),
            column_id: count.clone(),
            grid: vec![vec!["10".into()], vec!["30".into()]],
        })
        .unwrap();
    assert_eq!(page(&store, &frame.id).rows, [["A", "10"], ["C", "30"]]);
    assert_eq!(
        frame_named(store.document(), "Stock").appended_rows,
        0,
        "two visible rows held two values; nothing needed adding"
    );

    // And a struck-out row cannot be addressed directly either: it is not on
    // the page, so an id naming it is a stale one.
    assert!(
        store
            .apply(Operation::SetCells {
                frame_id: frame.id.clone(),
                cells: vec![update(&ids[1], &count, "9")],
            })
            .is_err()
    );
}

#[test]
fn a_bulk_edit_touching_a_calculated_column_is_refused_whole() {
    let dir = temporary_test_directory("bulk-overlay-calculated");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,1\nB,2\n");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "doubled~bulk".into(),
                    name: "Doubled".into(),
                    formula: "`Count` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    let count = frame.columns[1].id.clone();
    let doubled = frame
        .columns
        .iter()
        .find(|column| column.name == "Doubled")
        .unwrap()
        .id
        .clone();
    let ids = page(&store, &frame.id).row_ids;

    assert!(
        store
            .apply(Operation::SetCells {
                frame_id: frame.id.clone(),
                cells: vec![update(&ids[0], &count, "5"), update(&ids[0], &doubled, "7")],
            })
            .is_err(),
        "a calculation is edited as a formula, even inside a range"
    );
    assert!(
        store
            .apply(Operation::PasteCells {
                frame_id: frame.id.clone(),
                row_id: ids[0].clone(),
                column_id: count.clone(),
                grid: vec![vec!["5".into(), "7".into()]],
            })
            .is_err(),
        "a paste is one rectangle, not a best-effort collection of edits"
    );
    assert!(
        frame_named(store.document(), "Stock")
            .cell_overlay
            .is_empty(),
        "and neither refusal left part of itself behind"
    );

    // The stored column alone is still editable in bulk.
    store
        .apply(Operation::SetCells {
            frame_id: frame.id.clone(),
            cells: vec![update(&ids[0], &count, "5"), update(&ids[1], &count, "6")],
        })
        .unwrap();
    assert_eq!(column(&store, &frame.id, 1), ["5", "6"]);
    assert_eq!(column(&store, &frame.id, 2), ["10", "12"]);
}

/// The ids a sorted page hands out are the ordinals the rows were read at,
/// not their places on the page, so a range edit made over a sorted view
/// lands on the rows somebody selected.
#[test]
fn a_range_edit_over_a_sorted_view_lands_on_the_rows_selected() {
    let dir = temporary_test_directory("bulk-overlay-sorted");
    let (mut store, frame) = open_file(&dir, b"SKU,Count\nA,3\nB,1\nC,2\n");
    let count = frame.columns[1].id.clone();
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: frame.id.clone(),
            keys: vec![DerivedSort {
                column_id: count.clone(),
                descending: false,
            }],
        })
        .unwrap();
    let sorted = page(&store, &frame.id);
    assert_eq!(sorted.rows, [["B", "1"], ["C", "2"], ["A", "3"]]);
    assert_eq!(
        sorted.row_ids,
        [
            format!("source:{}:1", frame.id),
            format!("source:{}:2", frame.id),
            format!("source:{}:0", frame.id),
        ],
        "each id is the row's ordinal in the file, before the sort moved it"
    );

    // Select the first two rows of the sorted view — B and C — and type over
    // their SKU.
    let sku = frame.columns[0].id.clone();
    store
        .apply(Operation::SetCells {
            frame_id: frame.id.clone(),
            cells: vec![
                update(&sorted.row_ids[0], &sku, "b"),
                update(&sorted.row_ids[1], &sku, "c"),
            ],
        })
        .unwrap();
    assert_eq!(
        page(&store, &frame.id).rows,
        [["b", "1"], ["c", "2"], ["A", "3"]]
    );
}
