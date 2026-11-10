//! Formatting a dragged selection as one edit.
//!
//! Bold over a rectangle is one gesture, so it is one record and one undo —
//! a set of rows by a set of columns rather than a record per cell — and the
//! newest format of a cell wins whichever gesture made it.

use crate::common::*;
use framework_core::*;

fn bold(value: bool) -> FrameCellStyle {
    FrameCellStyle {
        bold: Some(value),
        ..FrameCellStyle::default()
    }
}

fn fill(color: &str) -> FrameCellStyle {
    FrameCellStyle {
        fill_color: Some(color.into()),
        ..FrameCellStyle::default()
    }
}

fn range(rows: &[Id], columns: &[Id]) -> FrameStyleTarget {
    FrameStyleTarget::Range {
        row_ids: rows.to_vec(),
        column_ids: columns.to_vec(),
    }
}

fn styles(store: &Store, frame_id: &str) -> Vec<FrameStyle> {
    store
        .document()
        .frame(frame_id)
        .unwrap()
        .display
        .styles
        .clone()
}

fn orders(store: &Store) -> (Id, Vec<Id>, Vec<Id>) {
    let frame = frame_named(store.document(), "Orders");
    (
        frame.id.clone(),
        frame.rows.iter().map(|row| row.id.clone()).collect(),
        frame
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect(),
    )
}

#[test]
fn a_selection_is_formatted_by_one_record_and_one_undo() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    assert!(rows.len() >= 2 && columns.len() >= 2);
    let target = range(&rows[..2], &columns[..2]);

    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: target.clone(),
            style: bold(true),
        })
        .unwrap();
    assert_eq!(
        styles(&store, &frame_id),
        vec![FrameStyle {
            target,
            style: bold(true)
        }]
    );
    let json = serde_json::to_string(store.document()).unwrap();
    assert!(json.contains(r#""kind":"range""#), "{json}");
    serde_json::from_str::<Document>(&json).unwrap();

    store.undo();
    assert!(styles(&store, &frame_id).is_empty());
    store.redo();
    assert_eq!(styles(&store, &frame_id).len(), 1);
}

/// A cell made bold on its own, then un-bolded as part of a selection, is
/// not bold: the range takes the property the cell format set, and leaves
/// the cell's other properties alone.
#[test]
fn a_range_takes_its_properties_from_the_cell_formats_beneath_it() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    let cell = FrameStyleTarget::Cell {
        row_id: rows[0].clone(),
        column_id: columns[0].clone(),
    };
    let outside = FrameStyleTarget::Cell {
        row_id: rows[1].clone(),
        column_id: columns[1].clone(),
    };
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: cell.clone(),
            style: FrameCellStyle {
                bold: Some(true),
                fill_color: Some("#ffeeaa".into()),
                ..FrameCellStyle::default()
            },
        })
        .unwrap();
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: outside.clone(),
            style: bold(true),
        })
        .unwrap();
    let before = styles(&store, &frame_id);

    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: range(&rows[..1], &columns[..1]),
            style: bold(false),
        })
        .unwrap();
    let after = styles(&store, &frame_id);
    let cell_style = &after
        .iter()
        .find(|entry| entry.target == cell)
        .unwrap()
        .style;
    assert_eq!(cell_style.bold, None, "the range now decides bold");
    assert_eq!(cell_style.fill_color.as_deref(), Some("#ffeeaa"));
    assert_eq!(
        after
            .iter()
            .find(|entry| entry.target == outside)
            .unwrap()
            .style,
        bold(true),
        "a cell outside the range is untouched"
    );

    // Undo puts every trimmed format back as it was, in one step.
    store.undo();
    assert_eq!(styles(&store, &frame_id), before);
}

#[test]
fn a_later_range_wins_over_an_earlier_one_where_they_overlap() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    let wide = range(&rows, &columns);
    let narrow = range(&rows[..1], &columns[..1]);
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: narrow.clone(),
            style: fill("#aaaaaa"),
        })
        .unwrap();
    // Wholly covered, the earlier range gives up the property the newer one
    // sets and, having nothing left, goes.
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: wide.clone(),
            style: fill("#bbbbbb"),
        })
        .unwrap();
    assert_eq!(
        styles(&store, &frame_id),
        vec![FrameStyle {
            target: wide.clone(),
            style: fill("#bbbbbb")
        }]
    );
    // Partly covered, it stays — it still paints the cells outside — and the
    // newer one is after it, which is what makes it win the overlap.
    let overlapping = range(&rows[..1], &columns);
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: overlapping.clone(),
            style: bold(true),
        })
        .unwrap();
    let entries = styles(&store, &frame_id);
    assert_eq!(
        entries
            .iter()
            .map(|entry| &entry.target)
            .collect::<Vec<_>>(),
        vec![&wide, &overlapping]
    );
}

#[test]
fn an_empty_range_style_clears_the_formats_beneath_it() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    let row_format = FrameStyleTarget::Row {
        row_id: rows[0].clone(),
    };
    for (target, style) in [
        (
            FrameStyleTarget::Cell {
                row_id: rows[0].clone(),
                column_id: columns[0].clone(),
            },
            bold(true),
        ),
        (range(&rows[..1], &columns[..2]), fill("#cccccc")),
        (row_format.clone(), bold(true)),
    ] {
        store
            .apply(Operation::SetFrameStyle {
                frame_id: frame_id.clone(),
                target,
                style,
            })
            .unwrap();
    }
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: range(&rows[..1], &columns),
            style: FrameCellStyle::default(),
        })
        .unwrap();
    // Cell formats and ranges inside go; a row format is not a cell format,
    // so the selection falls back to it.
    assert_eq!(
        styles(&store, &frame_id),
        vec![FrameStyle {
            target: row_format,
            style: bold(true)
        }]
    );
}

#[test]
fn deleting_rows_and_columns_trims_a_range_and_drops_it_when_empty() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    let last_column = columns.last().unwrap().clone();
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: range(&rows[..2], &[columns[0].clone(), last_column.clone()]),
            style: bold(true),
        })
        .unwrap();
    store
        .apply(Operation::DeleteColumn {
            frame_id: frame_id.clone(),
            column_id: last_column,
        })
        .unwrap();
    store
        .apply(Operation::DeleteRows {
            frame_id: frame_id.clone(),
            row_ids: vec![rows[0].clone()],
        })
        .unwrap();
    assert_eq!(
        styles(&store, &frame_id)[0].target,
        range(&rows[1..2], &columns[..1])
    );
    store
        .apply(Operation::DeleteRows {
            frame_id: frame_id.clone(),
            row_ids: vec![rows[1].clone()],
        })
        .unwrap();
    assert!(styles(&store, &frame_id).is_empty());
}

#[test]
fn a_range_naming_nothing_or_something_missing_is_refused() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = orders(&store);
    let refusal = |store: &mut Store, target| {
        store
            .apply(Operation::SetFrameStyle {
                frame_id: frame_id.clone(),
                target,
                style: bold(true),
            })
            .unwrap_err()
    };
    let empty = refusal(&mut store, range(&[], &columns)).to_string();
    assert!(empty.contains("at least one row and one column"), "{empty}");
    assert!(matches!(
        refusal(&mut store, range(&rows, &["nope".into()])),
        CoreError::ColumnNotFound
    ));
    // An owned frame knows its rows, so it can say one is not among them.
    assert!(matches!(
        refusal(&mut store, range(&["nope".into()], &columns)),
        CoreError::RowNotFound
    ));
    assert!(styles(&store, &frame_id).is_empty());
}

/// A frame whose rows are positions — a derived tab here — takes a range
/// too, and ten thousand rows by three columns is still one record of
/// ten thousand and three ids.
#[test]
fn a_large_selection_on_a_derived_frame_is_one_record() {
    let mut store = demo_store();
    let (orders_id, _, _) = orders(&store);
    let window_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == orders_id)
        .unwrap()
        .id
        .clone();
    let copy_id = store
        .apply(Operation::BranchFrame {
            view_id: window_id.clone(),
            frame_id: orders_id,
        })
        .unwrap()
        .document
        .view(&window_id)
        .unwrap()
        .object_id
        .clone();
    let copy = store.document().frame(&copy_id).unwrap().clone();
    let rows: Vec<Id> = (0..10_000)
        .map(|index| format!("derived:{copy_id}:{index}"))
        .collect();
    let columns: Vec<Id> = copy
        .columns
        .iter()
        .map(|column| column.id.clone())
        .collect();
    store
        .apply(Operation::SetFrameStyle {
            frame_id: copy_id.clone(),
            target: range(&rows, &columns),
            style: fill("#ddeeff"),
        })
        .unwrap();
    let entries = styles(&store, &copy_id);
    assert_eq!(entries.len(), 1);
    match &entries[0].target {
        FrameStyleTarget::Range {
            row_ids,
            column_ids,
        } => {
            assert_eq!(row_ids.len(), 10_000);
            assert_eq!(column_ids.len(), columns.len());
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        store.document().frame(&copy_id).unwrap().derivation,
        copy.derivation
    );
}

/// A five-by-three frame of its own, so the partial-overlap tests can name
/// a corner without depending on the demo's shape.
fn grid(store: &mut Store) -> (Id, Vec<Id>, Vec<Id>) {
    store
        .apply(Operation::AddFrame {
            name: "Grid".into(),
            grid: vec![
                vec!["A".into(), "B".into(), "C".into()],
                vec!["1".into(), "2".into(), "3".into()],
                vec!["4".into(), "5".into(), "6".into()],
                vec!["7".into(), "8".into(), "9".into()],
                vec!["10".into(), "11".into(), "12".into()],
                vec!["13".into(), "14".into(), "15".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Grid");
    (
        frame.id.clone(),
        frame.rows.iter().map(|row| row.id.clone()).collect(),
        frame
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect(),
    )
}

fn set(store: &mut Store, frame_id: &str, target: FrameStyleTarget, style: FrameCellStyle) {
    store
        .apply(Operation::SetFrameStyle {
            frame_id: frame_id.into(),
            target,
            style,
        })
        .unwrap();
}

fn entry(target: FrameStyleTarget, style: FrameCellStyle) -> FrameStyle {
    FrameStyle { target, style }
}

/// "Clear formatting" on A1:A2 of a bold A1:B5 used to do nothing: the
/// earlier range was only touched when wholly covered. Now it gives the
/// cleared cells up and keeps painting the rest.
#[test]
fn clearing_some_rows_of_a_range_leaves_the_other_rows_formatted() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    set(
        &mut store,
        &frame_id,
        range(&rows, &columns[..2]),
        bold(true),
    );
    let before = styles(&store, &frame_id);
    set(
        &mut store,
        &frame_id,
        range(&rows[..2], &columns[..2]),
        FrameCellStyle::default(),
    );
    assert_eq!(
        styles(&store, &frame_id),
        vec![entry(range(&rows[2..], &columns[..2]), bold(true))]
    );
    store.undo();
    assert_eq!(styles(&store, &frame_id), before);
    store.redo();
    assert_eq!(
        styles(&store, &frame_id),
        vec![entry(range(&rows[2..], &columns[..2]), bold(true))]
    );
}

#[test]
fn clearing_some_columns_of_a_range_leaves_the_other_columns_formatted() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    set(
        &mut store,
        &frame_id,
        range(&rows, &columns[..2]),
        bold(true),
    );
    let before = styles(&store, &frame_id);
    set(
        &mut store,
        &frame_id,
        range(&rows, &columns[..1]),
        FrameCellStyle::default(),
    );
    assert_eq!(
        styles(&store, &frame_id),
        vec![entry(range(&rows, &columns[1..2]), bold(true))]
    );
    store.undo();
    assert_eq!(styles(&store, &frame_id), before);
}

/// A clear over a corner splits the range in two: the rows it missed, at
/// full width, and the rows it hit, at the columns it missed. A later
/// range keeps its place after both, so it still wins where it overlaps.
#[test]
fn clearing_a_corner_of_a_range_splits_it_and_keeps_the_order() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    let later = range(&rows[4..], &columns);
    set(
        &mut store,
        &frame_id,
        range(&rows, &columns[..2]),
        bold(true),
    );
    set(&mut store, &frame_id, later.clone(), fill("#eeeeee"));
    let before = styles(&store, &frame_id);
    // B1:C2 — overhangs the bold range into column C.
    set(
        &mut store,
        &frame_id,
        range(&rows[..2], &columns[1..]),
        FrameCellStyle::default(),
    );
    assert_eq!(
        styles(&store, &frame_id),
        vec![
            entry(range(&rows[2..], &columns[..2]), bold(true)),
            entry(range(&rows[..2], &columns[..1]), bold(true)),
            entry(later, fill("#eeeeee")),
        ]
    );
    store.undo();
    assert_eq!(styles(&store, &frame_id), before);
}

/// A patch over part of an earlier range needs no split: the later range
/// is after it in the list, so it wins the overlap property by property and
/// the earlier one's other properties still show through.
#[test]
fn a_patch_over_part_of_a_range_wins_by_order_without_splitting_it() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    let earlier = range(&rows, &columns[..2]);
    let patch = range(&rows[..2], &columns[1..]);
    set(&mut store, &frame_id, earlier.clone(), bold(true));
    set(&mut store, &frame_id, patch.clone(), bold(false));
    assert_eq!(
        styles(&store, &frame_id),
        vec![entry(earlier, bold(true)), entry(patch, bold(false))]
    );
}

/// Somebody else's edit, arriving through the shared journal the way one
/// really does — the only path that marks an event as not this writer's to
/// undo.
fn merge_remote(store: &mut Store, operation: ReplicatedOperation) {
    let directory = temporary_test_directory("range-styles-remote");
    let path = directory.join("shared.fw");
    store.save(&path).unwrap();
    let journal = EventJournal::open(&path, &store.document().id.clone()).unwrap();
    journal
        .append(&OperationEvent::new(
            store.document().id.clone(),
            "6f1f7a4e-0b6a-4a1e-9a1a-2c9d3b5e7f02".to_string(),
            1,
            VersionVector::new(),
            operation,
        ))
        .unwrap();
    journal.merge_into(store).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

/// Undoing a range used to put back the whole style list as it was, so a
/// collaborator's column format made in between vanished with it.
#[test]
fn undoing_a_range_leaves_a_peers_later_format_alone() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    let cell = FrameStyleTarget::Cell {
        row_id: rows[0].clone(),
        column_id: columns[0].clone(),
    };
    set(
        &mut store,
        &frame_id,
        cell.clone(),
        FrameCellStyle {
            bold: Some(true),
            italic: Some(true),
            ..FrameCellStyle::default()
        },
    );
    let ranged = range(&rows[..2], &columns[..2]);
    set(&mut store, &frame_id, ranged.clone(), bold(false));
    let peer_column = FrameStyleTarget::Column {
        column_id: columns[2].clone(),
    };
    merge_remote(
        &mut store,
        ReplicatedOperation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: peer_column.clone(),
            style: fill("#ffcc00"),
        },
    );

    store.undo();
    assert_eq!(
        styles(&store, &frame_id),
        vec![
            entry(
                cell.clone(),
                FrameCellStyle {
                    bold: Some(true),
                    italic: Some(true),
                    ..FrameCellStyle::default()
                }
            ),
            entry(peer_column.clone(), fill("#ffcc00")),
        ],
        "the range and its trimming are undone; the peer's column format stays"
    );
    store.redo();
    assert_eq!(
        styles(&store, &frame_id),
        vec![
            entry(
                cell,
                FrameCellStyle {
                    italic: Some(true),
                    ..FrameCellStyle::default()
                }
            ),
            entry(peer_column, fill("#ffcc00")),
            entry(ranged, bold(false)),
        ]
    );
}

/// When a peer has since changed a format the range trimmed, undo leaves
/// the peer's version: restoring ours would overwrite an edit that came
/// later and was not ours to take back.
#[test]
fn undoing_a_range_keeps_a_peers_later_change_to_a_format_it_trimmed() {
    let mut store = demo_store();
    let (frame_id, rows, columns) = grid(&mut store);
    let cell = FrameStyleTarget::Cell {
        row_id: rows[0].clone(),
        column_id: columns[0].clone(),
    };
    set(&mut store, &frame_id, cell.clone(), bold(true));
    let ranged = range(&rows[..2], &columns[..2]);
    // Clear removes the cell format outright.
    set(&mut store, &frame_id, ranged, FrameCellStyle::default());
    merge_remote(
        &mut store,
        ReplicatedOperation::SetFrameStyle {
            frame_id: frame_id.clone(),
            target: cell.clone(),
            style: fill("#00ff00"),
        },
    );
    store.undo();
    assert_eq!(
        styles(&store, &frame_id),
        vec![entry(cell, fill("#00ff00"))]
    );
}
