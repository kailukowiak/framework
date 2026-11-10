//! Column widths and text wrap, as display state.
//!
//! A width somebody dragged, or the one auto-fit measured, is a way of
//! looking at a frame, not a fact about its data — so it lives in the display
//! layer beside freezing and sorting, is accepted on every kind of frame, and
//! never touches rows, columns, or the chain.

use crate::common::*;
use framework_core::*;

fn layout(column_id: &str, width: Option<f64>, wrap: bool) -> FrameColumnLayoutUpdate {
    FrameColumnLayoutUpdate {
        column_id: column_id.into(),
        width,
        wrap,
    }
}

fn layout_of(store: &Store, frame_id: &str, column_id: &str) -> Option<FrameColumnLayout> {
    store
        .document()
        .frame(frame_id)
        .unwrap()
        .display
        .column_layout
        .get(column_id)
        .cloned()
}

/// Auto-fit over every column is one gesture, so it is one operation and
/// one undo; a later resize replaces one column's layout and leaves the
/// rest alone.
#[test]
fn several_columns_are_laid_out_in_one_undoable_edit() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let first = orders.columns[0].id.clone();
    let second = orders.columns[1].id.clone();

    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![
                layout(&first, Some(84.0), false),
                layout(&second, Some(120.5), true),
            ],
        })
        .unwrap();
    assert_eq!(
        layout_of(&store, &orders.id, &first),
        Some(FrameColumnLayout {
            width: Some(84.0),
            wrap: false
        })
    );
    assert_eq!(
        layout_of(&store, &orders.id, &second),
        Some(FrameColumnLayout {
            width: Some(120.5),
            wrap: true
        })
    );

    // A resize names one column and carries its whole layout: the wrap the
    // client already shows is sent back with the new width.
    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout(&second, Some(200.0), true)],
        })
        .unwrap();
    assert_eq!(
        layout_of(&store, &orders.id, &second).unwrap().width,
        Some(200.0)
    );
    assert_eq!(
        layout_of(&store, &orders.id, &first).unwrap().width,
        Some(84.0)
    );

    // Back to automatic, unwrapped: nothing is left stored for the column.
    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout(&second, None, false)],
        })
        .unwrap();
    assert_eq!(layout_of(&store, &orders.id, &second), None);

    store.undo();
    assert_eq!(
        layout_of(&store, &orders.id, &second).unwrap().width,
        Some(200.0)
    );
    store.undo();
    assert_eq!(
        layout_of(&store, &orders.id, &second).unwrap().width,
        Some(120.5)
    );
    // The auto-fit itself goes back in one step, both columns together.
    store.undo();
    assert_eq!(layout_of(&store, &orders.id, &first), None);
    assert_eq!(layout_of(&store, &orders.id, &second), None);
    assert!(
        store
            .document()
            .frame(&orders.id)
            .unwrap()
            .display
            .is_empty()
    );
}

#[test]
fn column_layout_survives_a_save_and_writes_nothing_when_automatic() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let json = serde_json::to_string(store.document()).unwrap();
    assert!(!json.contains("columnLayout"), "{json}");

    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout(&orders.columns[0].id, Some(96.0), true)],
        })
        .unwrap();
    let json = serde_json::to_string(store.document()).unwrap();
    let reopened: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(
        reopened
            .frame(&orders.id)
            .unwrap()
            .display
            .column_layout
            .get(&orders.columns[0].id),
        Some(&FrameColumnLayout {
            width: Some(96.0),
            wrap: true
        })
    );
    // And it reaches the interface on the document the view carries.
    let view = store.view();
    assert!(
        view.document
            .frame(&orders.id)
            .unwrap()
            .display
            .column_layout
            .contains_key(&orders.columns[0].id)
    );
}

/// A width is display state, so a frame that refuses edits to its data —
/// a derived tab here — still takes one, and its chain is untouched.
#[test]
fn a_derived_frame_takes_a_column_width() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let window_id = store
        .document()
        .views
        .iter()
        .find(|view| view.object_id == orders.id)
        .unwrap()
        .id
        .clone();
    let branched = store
        .apply(Operation::BranchFrame {
            view_id: window_id.clone(),
            frame_id: orders.id.clone(),
        })
        .unwrap();
    let copy_id = branched
        .document
        .view(&window_id)
        .unwrap()
        .object_id
        .clone();
    let copy = store.document().frame(&copy_id).unwrap().clone();
    assert!(copy.derivation.is_some());
    let column_id = copy.columns[0].id.clone();

    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: copy_id.clone(),
            columns: vec![layout(&column_id, Some(150.0), false)],
        })
        .unwrap();
    let after = store.document().frame(&copy_id).unwrap();
    assert_eq!(after.derivation, copy.derivation);
    assert_eq!(after.columns, copy.columns);
    assert_eq!(after.display.column_layout[&column_id].width, Some(150.0));
}

#[test]
fn a_layout_naming_a_missing_or_repeated_column_or_a_bad_width_is_refused() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    let first = orders.columns[0].id.clone();

    let missing = store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout("not-a-column", Some(80.0), false)],
        })
        .unwrap_err();
    assert!(matches!(missing, CoreError::ColumnNotFound), "{missing}");

    let twice = store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![
                layout(&first, Some(80.0), false),
                layout(&first, Some(90.0), false),
            ],
        })
        .unwrap_err()
        .to_string();
    assert!(twice.contains("more than once"), "{twice}");

    for width in [f64::NAN, f64::INFINITY, 0.0, -10.0] {
        let refusal = store
            .apply(Operation::SetFrameDisplayColumnLayout {
                frame_id: orders.id.clone(),
                columns: vec![layout(&first, Some(width), false)],
            })
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("width"), "{width}: {refusal}");
    }
    assert!(
        store
            .document()
            .frame(&orders.id)
            .unwrap()
            .display
            .is_empty()
    );

    // Sizes outside what a grid can draw are held to its range rather than
    // refused: a drag past the edge means "as narrow as it goes".
    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout(&first, Some(1.0), false)],
        })
        .unwrap();
    assert_eq!(
        layout_of(&store, &orders.id, &first).unwrap().width,
        Some(MIN_COLUMN_WIDTH)
    );
    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![layout(&first, Some(1.0e9), false)],
        })
        .unwrap();
    assert_eq!(
        layout_of(&store, &orders.id, &first).unwrap().width,
        Some(MAX_COLUMN_WIDTH)
    );
}

/// Deleting a column takes its layout with it, and undoing the delete
/// brings both back.
#[test]
fn deleting_a_column_forgets_its_layout() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();
    // The last column is the one nothing reads, so it can be deleted.
    let going = orders.columns.last().unwrap().id.clone();
    let staying = orders.columns[0].id.clone();
    store
        .apply(Operation::SetFrameDisplayColumnLayout {
            frame_id: orders.id.clone(),
            columns: vec![
                layout(&going, Some(140.0), true),
                layout(&staying, Some(60.0), false),
            ],
        })
        .unwrap();
    store
        .apply(Operation::DeleteColumn {
            frame_id: orders.id.clone(),
            column_id: going.clone(),
        })
        .unwrap();
    assert_eq!(layout_of(&store, &orders.id, &going), None);
    assert_eq!(
        layout_of(&store, &orders.id, &staying).unwrap().width,
        Some(60.0)
    );
    // A whole document round trip with the column gone still opens.
    let json = serde_json::to_string(store.document()).unwrap();
    serde_json::from_str::<Document>(&json).unwrap();

    store.undo();
    assert_eq!(
        layout_of(&store, &orders.id, &going),
        Some(FrameColumnLayout {
            width: Some(140.0),
            wrap: true
        })
    );
}
