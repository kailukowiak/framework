//! Several operations as one edit: one revision, one undo, all or nothing.

use crate::common::*;
use framework_core::*;

fn orders_window(store: &Store) -> (Id, Id) {
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    let view_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == frame_id)
        .unwrap()
        .id
        .clone();
    (frame_id, view_id)
}

/// The gesture that asked for batches: a paste that also grows its card.
/// One ⌘Z takes back both — the data and the size — rather than shrinking
/// the card and leaving the paste behind.
#[test]
fn a_paste_and_the_resize_that_fits_it_undo_together() {
    let mut store = demo_store();
    let (frame_id, view_id) = orders_window(&store);
    let frame = frame_named(store.document(), "Orders").clone();
    let before_cells = frame.rows[0].cells.clone();
    let before_view = store.document().view(&view_id).unwrap().clone();
    let revision = store.document().revision;

    let view = store
        .apply(Operation::Batch {
            operations: vec![
                Operation::PasteCells {
                    frame_id: frame_id.clone(),
                    row_id: frame.rows[0].id.clone(),
                    column_id: frame.columns[0].id.clone(),
                    grid: vec![vec!["7".into(), "11".into()]],
                },
                Operation::ResizeView {
                    view_id: view_id.clone(),
                    width: before_view.width + 120.0,
                    height: before_view.height + 40.0,
                },
            ],
        })
        .unwrap();
    assert_eq!(
        view.document.revision,
        revision + 1,
        "one edit, one revision"
    );
    let pasted = frame_named(store.document(), "Orders");
    assert_ne!(pasted.rows[0].cells, before_cells);
    assert_eq!(
        store.document().view(&view_id).unwrap().width,
        before_view.width + 120.0
    );

    let undone = store.undo();
    assert_eq!(
        frame_named(&undone.document, "Orders").rows[0].cells,
        before_cells
    );
    assert_eq!(
        undone.document.view(&view_id).unwrap().width,
        before_view.width
    );
    assert!(!undone.can_undo, "the batch was the only entry");

    let redone = store.redo();
    assert_ne!(
        frame_named(&redone.document, "Orders").rows[0].cells,
        before_cells
    );
    assert_eq!(
        redone.document.view(&view_id).unwrap().width,
        before_view.width + 120.0
    );
}

/// Each part is prepared against what the parts before it leave, so a
/// later one may read a column an earlier one made.
#[test]
fn a_later_part_sees_what_an_earlier_part_made() {
    let mut store = demo_store();
    let (frame_id, _) = orders_window(&store);
    let steps_before = frame_named(store.document(), "Orders").steps.len();
    store
        .apply(Operation::Batch {
            operations: vec![
                Operation::AddColumn {
                    frame_id: frame_id.clone(),
                    name: "Margin".into(),
                    data_type: DataType::Number,
                    after_column_id: None,
                    before_column_id: None,
                    values: Default::default(),
                },
                Operation::SetFrameDisplayFilter {
                    frame_id: frame_id.clone(),
                    filters: vec!["`Margin`.is_null()".into()],
                    filter_match_all: true,
                },
            ],
        })
        .unwrap();
    let frame = frame_named(store.document(), "Orders");
    assert!(frame.columns.iter().any(|column| column.name == "Margin"));
    // A header filter is a step of the frame's own chain now.
    assert_eq!(frame.steps.len(), steps_before + 1);

    store.undo();
    let frame = frame_named(store.document(), "Orders");
    assert!(!frame.columns.iter().any(|column| column.name == "Margin"));
    assert_eq!(frame.steps.len(), steps_before);
}

/// A part that fails leaves nothing behind: not the parts before it, not a
/// revision, not an undo entry.
#[test]
fn a_batch_applies_all_of_itself_or_none() {
    let mut store = demo_store();
    let (frame_id, view_id) = orders_window(&store);
    let last_column = frame_named(store.document(), "Orders")
        .columns
        .last()
        .unwrap()
        .id
        .clone();
    let before = store.document().clone();

    // The second delete is prepared after the first has removed the column,
    // so it names a column that is no longer there.
    let refusal = store
        .apply(Operation::Batch {
            operations: vec![
                Operation::MoveView {
                    view_id: view_id.clone(),
                    x: 900.0,
                    y: 900.0,
                },
                Operation::DeleteColumn {
                    frame_id: frame_id.clone(),
                    column_id: last_column.clone(),
                },
                Operation::DeleteColumn {
                    frame_id,
                    column_id: last_column,
                },
            ],
        })
        .unwrap_err();
    assert!(matches!(refusal, CoreError::ColumnNotFound), "{refusal}");
    assert_eq!(
        serde_json::to_value(store.document()).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    assert!(!store.view().can_undo);
}

#[test]
fn an_empty_or_nested_batch_is_refused() {
    let mut store = demo_store();
    let empty = store
        .apply(Operation::Batch {
            operations: Vec::new(),
        })
        .unwrap_err()
        .to_string();
    assert!(empty.contains("at least one operation"), "{empty}");
    let nested = store
        .apply(Operation::Batch {
            operations: vec![Operation::Batch {
                operations: vec![Operation::TidyLayout],
            }],
        })
        .unwrap_err()
        .to_string();
    assert!(nested.contains("cannot contain another batch"), "{nested}");
}

/// A batch is one event, so a peer gets the whole of it in one step and
/// lands on the same document.
#[test]
fn a_batch_replicates_as_one_event() {
    let mut local = demo_store();
    let mut peer = Store::new(local.document().clone());
    let (_, view_id) = orders_window(&local);
    let writer = "3f2b8c4e-1d6a-4b9e-8c7f-5a2e9d1b0c34";
    let event = local
        .prepare_event(
            writer,
            Operation::Batch {
                operations: vec![
                    Operation::MoveView {
                        view_id: view_id.clone(),
                        x: 300.0,
                        y: 40.0,
                    },
                    Operation::RenameDocument {
                        name: "Together".into(),
                    },
                ],
            },
        )
        .unwrap();
    local.apply_event(&event).unwrap();
    let json = serde_json::to_string(&event).unwrap();
    let received: OperationEvent = serde_json::from_str(&json).unwrap();
    peer.apply_event(&received).unwrap();
    assert_eq!(peer.document().name, "Together");
    assert_eq!(peer.document().view(&view_id).unwrap().x, 300.0);
}

/// Safe mode refuses model fitting; a batch is not a way around it.
#[test]
fn safe_mode_looks_inside_a_batch() {
    let mut store = demo_store();
    store.set_safe_mode(true);
    let refusal = store
        .apply(Operation::Batch {
            operations: vec![Operation::FitModel {
                model_id: "anything".into(),
            }],
        })
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("Enable evaluation"), "{refusal}");
}
