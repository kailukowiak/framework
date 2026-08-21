//! Comments and prose: remarks in a wrangle chain, a remark pinned to a
//! frame, and text cards whose `{{…}}` holes print live values.

#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Comments".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    })
}

fn add_rows_frame(store: &mut Store) -> FrameObject {
    store
        .apply(Operation::AddFrame {
            name: "Rows".into(),
            grid: vec![vec!["Value".into()], vec!["2".into()], vec!["1".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    frame_named(store.document(), "Rows").clone()
}

fn add_value(store: &mut Store, name: &str, raw: &str) {
    let holder = a_container(store);
    store
        .apply(Operation::AddValue {
            name: name.into(),
            raw: raw.into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
}

fn text_card(store: &Store) -> TextObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Text(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap()
}

fn computed_text(store: &Store) -> ComputedText {
    let id = text_card(store).id;
    store.view().computed_texts.get(&id).cloned().unwrap()
}

#[test]
fn a_chain_comment_changes_nothing_and_renders_back() {
    let mut store = blank_store();
    let frame = add_rows_frame(&mut store);

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Comment {
                    text: "Keep only the big rows".into(),
                },
                FrameStepInput::Filter {
                    predicates: vec!["`Value` > 1".into()],
                    match_all: true,
                },
            ],
        })
        .unwrap();

    // The engine sees only the filter.
    let page = store.get_frame_page(&frame.id, 0, 100).unwrap();
    assert_eq!(page.rows, vec![vec!["2".to_string()]]);

    // The editor sees the remark, in its position, verbatim — stored and
    // rendered back.
    let stored = frame_named(store.document(), "Rows");
    assert!(matches!(
        &stored.steps[0],
        FrameStep::Comment { text } if text == "Keep only the big rows"
    ));
    let computed = store
        .view()
        .computed_frames
        .get(&frame.id)
        .cloned()
        .unwrap();
    assert!(matches!(
        &computed.steps[0],
        RenderedFrameStep::Comment { text } if text == "Keep only the big rows"
    ));
}

#[test]
fn a_comment_alone_keeps_a_source_frame_editable() {
    let mut store = blank_store();
    let frame = add_rows_frame(&mut store);

    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Comment {
                text: "Pasted from the March export".into(),
            }],
        })
        .unwrap();

    // A remark computes nothing, so the cells are still inputs.
    let stored = frame_named(store.document(), "Rows").clone();
    store
        .apply(Operation::SetCell {
            frame_id: stored.id.clone(),
            row_id: stored.rows[0].id.clone(),
            column_id: stored.columns[0].id.clone(),
            raw: "5".into(),
        })
        .unwrap();
    let page = store.get_frame_page(&stored.id, 0, 100).unwrap();
    assert_eq!(
        page.rows,
        vec![vec!["5".to_string()], vec!["1".to_string()]]
    );
}

#[test]
fn a_frame_comment_sets_clears_and_undoes() {
    let mut store = blank_store();
    let frame = add_rows_frame(&mut store);

    store
        .apply(Operation::SetFrameComment {
            frame_id: frame.id.clone(),
            comment: Some("Source of truth for the forecast".into()),
        })
        .unwrap();
    assert_eq!(
        frame_named(store.document(), "Rows").comment.as_deref(),
        Some("Source of truth for the forecast")
    );

    // Blank is no comment: deleting the text and choosing "remove" are the
    // same edit.
    store
        .apply(Operation::SetFrameComment {
            frame_id: frame.id.clone(),
            comment: Some("   ".into()),
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Rows").comment, None);

    store.undo();
    assert_eq!(
        frame_named(store.document(), "Rows").comment.as_deref(),
        Some("Source of truth for the forecast")
    );
}

#[test]
fn a_text_card_holds_prose_and_live_values() {
    let mut store = blank_store();
    add_value(&mut store, "Rate", "0.08");
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);

    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "Doubled, the rate is {{`Rate` * 2}}.".into(),
        })
        .unwrap();

    let computed = computed_text(&store);
    assert_eq!(computed.segments.len(), 3);
    assert!(matches!(
        &computed.segments[0],
        ComputedTextSegment::Literal { text } if text == "Doubled, the rate is "
    ));
    let ComputedTextSegment::Value { cell, .. } = &computed.segments[1] else {
        panic!("expected a value segment, got {:?}", computed.segments[1]);
    };
    assert_eq!(cell.value, Some(0.16));

    // The hole answers live: edit the input, the sentence changes.
    add_value(&mut store, "Rate", "0.08"); // no-op guard: container reused
    let holder_value_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Rate")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetValue {
            object_id: holder_value_id,
            raw: "0.5".into(),
        })
        .unwrap();
    let ComputedTextSegment::Value { cell, .. } = &computed_text(&store).segments[1] else {
        panic!("expected a value segment");
    };
    assert_eq!(cell.value, Some(1.0));
}

#[test]
fn a_text_card_aggregate_tracks_a_live_frame() {
    let mut store = blank_store();
    let frame = add_rows_frame(&mut store);
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);
    store
        .apply(Operation::SetTextSource {
            object_id: card.id,
            source: "Total: {{`Rows`.`Value`.sum()}}".into(),
        })
        .unwrap();

    let answer = |store: &Store| match &computed_text(store).segments[1] {
        ComputedTextSegment::Value { cell, .. } => cell.display.clone(),
        segment => panic!("expected a value segment, got {segment:?}"),
    };
    assert_eq!(answer(&store), "3");

    store
        .apply(Operation::SetCell {
            frame_id: frame.id,
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[0].id.clone(),
            raw: "5".into(),
        })
        .unwrap();
    assert_eq!(answer(&store), "6");
}

#[test]
fn a_broken_hole_keeps_its_text_and_complaint() {
    let mut store = blank_store();
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);

    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "See {{`Nothing here`}} for details.".into(),
        })
        .unwrap();

    let computed = computed_text(&store);
    let ComputedTextSegment::Broken { source, error } = &computed.segments[1] else {
        panic!("expected a broken segment, got {:?}", computed.segments[1]);
    };
    assert_eq!(source, "`Nothing here`");
    assert!(!error.is_empty());
    // The editable text still says what was typed.
    assert_eq!(computed.source, "See {{`Nothing here`}} for details.");
}

#[test]
fn an_unclosed_hole_reads_as_prose() {
    let mut store = blank_store();
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);

    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "Half-typed {{ and left".into(),
        })
        .unwrap();

    let computed = computed_text(&store);
    assert_eq!(computed.segments.len(), 1);
    assert_eq!(computed.source, "Half-typed {{ and left");
}

#[test]
fn renaming_a_value_rewrites_the_holes_that_read_it() {
    let mut store = blank_store();
    add_value(&mut store, "Rate", "0.08");
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);
    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "The rate is {{`Rate`}}.".into(),
        })
        .unwrap();

    let rate_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Rate")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::RenameObject {
            object_id: rate_id,
            name: "Growth".into(),
        })
        .unwrap();

    // The hole holds the value by id, so the reconstructed source follows
    // the rename without this card being edited.
    let source = computed_text(&store).source;
    assert!(source.contains("Growth"), "source was: {source}");
    assert!(!source.contains("Rate"), "source was: {source}");
}

#[test]
fn undo_restores_what_a_text_card_said() {
    let mut store = blank_store();
    store.apply(Operation::AddText { x: 0.0, y: 0.0 }).unwrap();
    let card = text_card(&store);

    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "First thought".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetTextSource {
            object_id: card.id.clone(),
            source: "Second thought".into(),
        })
        .unwrap();
    store.undo();
    assert_eq!(computed_text(&store).source, "First thought");
}

#[test]
fn a_legacy_text_card_reads_as_one_literal_and_survives_an_edit() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Legacy".into(),
        revision: 0,
        objects: vec![DataObject::Text(TextObject {
            id: "note".into(),
            name: "Note".into(),
            text: "An old plain note".into(),
            segments: Vec::new(),
        })],
        views: Vec::new(),
        frozen_values: Default::default(),
    });

    assert_eq!(computed_text(&store).source, "An old plain note");

    store
        .apply(Operation::SetTextSource {
            object_id: "note".into(),
            source: "A newer note".into(),
        })
        .unwrap();
    assert_eq!(computed_text(&store).source, "A newer note");

    // Undo restores what the card said, even though what it said was held
    // in the legacy field.
    store.undo();
    assert_eq!(computed_text(&store).source, "An old plain note");
}
