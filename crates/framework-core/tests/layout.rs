use crate::common::*;
use framework_core::*;
use std::collections::HashMap;

fn frame_id(store: &Store, name: &str) -> Id {
    frame_named(store.document(), name).id.clone()
}

fn placement(document: &Document, object_id: &str) -> (f64, f64) {
    let view = document
        .views
        .iter()
        .find(|view| view.tabs().iter().any(|tab| tab == object_id))
        .unwrap();
    (view.x, view.y)
}

/// The columns are the point: a frame you derived should sit to the right of
/// what it derives from, so the lineage cords run one way instead of
/// crossing the canvas.
#[test]
fn tidying_puts_derived_frames_to_the_right_of_their_sources() {
    let mut store = demo_store();
    let transactions = frame_id(&store, "Transactions");
    let customers = frame_id(&store, "Customers");

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: transactions.clone(),
            name: "Transaction detail".into(),
            // Deliberately placed left of its own source, so tidying has
            // something to actually fix.
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let document = store.apply(Operation::TidyLayout).unwrap().document;

    let derived = frame_id(&store, "Transaction detail");
    let (source_x, _) = placement(&document, &transactions);
    let (derived_x, _) = placement(&document, &derived);
    assert!(
        derived_x > source_x,
        "a derived frame should land right of its source: {derived_x} vs {source_x}"
    );
    // Every root frame shares the first column.
    assert_eq!(placement(&document, &customers).0, source_x);
}

/// A plot reads a frame, so it is downstream of that frame and belongs in
/// the next column over — same rule as a derived frame, same reason.
#[test]
fn tidying_treats_a_plot_as_downstream_of_the_frame_it_reads() {
    let mut store = demo_store();
    let transactions = frame_id(&store, "Transactions");
    store
        .apply(Operation::AddPlot {
            name: "Units by sale".into(),
            source_frame_id: transactions.clone(),
            spec: serde_json::json!({"mark": "bar"}),
            x: 0.0,
            y: 0.0,
            view_id: None,
        })
        .unwrap();
    let document = store.apply(Operation::TidyLayout).unwrap().document;

    let plot = document
        .objects
        .iter()
        .find(|object| object.name() == "Units by sale")
        .unwrap()
        .id()
        .to_string();
    assert!(placement(&document, &plot).0 > placement(&document, &transactions).0);
}

/// Windows in one column must not overlap, and the gap between them has to
/// be the same everywhere — that even rhythm is the whole visible result.
#[test]
fn tidied_windows_stack_without_overlapping_and_share_one_gutter() {
    let mut store = demo_store();
    let transactions = frame_id(&store, "Transactions");
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: transactions,
            name: "Transaction detail".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let document = store.apply(Operation::TidyLayout).unwrap().document;

    let heights: HashMap<&str, f64> = document
        .views
        .iter()
        .map(|view| (view.id.as_str(), view.height))
        .collect();
    let mut columns: HashMap<u64, Vec<&CanvasView>> = HashMap::new();
    for view in &document.views {
        columns.entry(view.x.to_bits()).or_default().push(view);
    }
    assert!(columns.len() > 1, "the demo has derived frames to column");

    let mut gaps = Vec::new();
    for views in columns.values_mut() {
        views.sort_by(|left, right| left.y.partial_cmp(&right.y).unwrap());
        for pair in views.windows(2) {
            let bottom = pair[0].y + heights[pair[0].id.as_str()];
            assert!(
                pair[1].y >= bottom,
                "windows in a column must not overlap: {} then {}",
                pair[0].y,
                pair[1].y
            );
            gaps.push(pair[1].y - bottom);
        }
    }
    assert!(!gaps.is_empty());
    for gap in &gaps {
        assert_eq!(gap, &gaps[0], "every gutter should be identical: {gaps:?}");
    }
}

/// Tidying is a pure function of the document, so asking twice must not
/// drift. A layout that crept a little each time would be worse than none.
#[test]
fn tidying_twice_changes_nothing_the_second_time() {
    let mut store = demo_store();
    let once = store.apply(Operation::TidyLayout).unwrap().document;
    let twice = store.apply(Operation::TidyLayout).unwrap().document;

    let positions = |document: &Document| {
        let mut all: Vec<(Id, u64, u64)> = document
            .views
            .iter()
            .map(|view| (view.id.clone(), view.x.to_bits(), view.y.to_bits()))
            .collect();
        all.sort();
        all
    };
    assert_eq!(positions(&once), positions(&twice));
}

/// A collapsed card draws a title bar, not its stored height. Stacking it
/// against the full height would leave exactly the hole collapsing it was
/// meant to close.
#[test]
fn a_collapsed_window_only_claims_the_height_it_draws() {
    let mut store = demo_store();
    let tidy = store.apply(Operation::TidyLayout).unwrap().document;

    // Use two frames so the assertion remains independent of the other root
    // objects the demo may add over time.
    let mut frames: Vec<&CanvasView> = tidy
        .views
        .iter()
        .filter(|view| tidy.frame(&view.object_id).is_ok())
        .collect();
    frames.sort_by(|left, right| {
        (left.x.to_bits(), left.y.to_bits()).cmp(&(right.x.to_bits(), right.y.to_bits()))
    });
    let stacked = frames
        .windows(2)
        .find(|pair| pair[0].x == pair[1].x)
        .expect("two frames in one column");
    let (top_id, follower_id, follower_before) =
        (stacked[0].id.clone(), stacked[1].id.clone(), stacked[1].y);

    store
        .apply(Operation::SetViewCollapsed {
            view_id: top_id,
            collapsed: true,
        })
        .unwrap();
    let collapsed = store.apply(Operation::TidyLayout).unwrap().document;
    let follower_after = collapsed
        .views
        .iter()
        .find(|view| view.id == follower_id)
        .unwrap()
        .y;

    assert!(
        follower_after < follower_before,
        "collapsing the window above should pull the next one up: \
         {follower_before} -> {follower_after}"
    );
}

#[test]
fn a_collapsed_block_also_only_claims_its_title_bar() {
    let mut store = demo_store();
    let tidy = store.apply(Operation::TidyLayout).unwrap().document;
    let block_id = tidy
        .objects
        .iter()
        .find(|object| matches!(object, DataObject::Block(_)))
        .expect("demo block")
        .id()
        .to_string();
    let block = tidy
        .views
        .iter()
        .find(|view| view.object_id == block_id)
        .unwrap();
    let follower = tidy
        .views
        .iter()
        .filter(|view| view.x == block.x && view.y > block.y)
        .min_by(|left, right| left.y.partial_cmp(&right.y).unwrap())
        .expect("a window beneath the block");
    let (block_view_id, follower_id, follower_before) =
        (block.id.clone(), follower.id.clone(), follower.y);

    store
        .apply(Operation::SetViewCollapsed {
            view_id: block_view_id,
            collapsed: true,
        })
        .unwrap();
    let collapsed = store.apply(Operation::TidyLayout).unwrap().document;
    let follower_after = collapsed
        .views
        .iter()
        .find(|view| view.id == follower_id)
        .unwrap()
        .y;

    assert!(follower_after < follower_before);
}
