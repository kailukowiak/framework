use crate::common::*;
use framework_core::*;

fn setup(replacement: &str) -> (Store, FrameObject, FrameObject) {
    let mut store = Store::new(Document::blank("Rename"));
    store
        .apply(Operation::AddFrame {
            name: "Data".into(),
            grid: vec![
                vec!["A".into(), "B".into(), "C".into()],
                vec!["1".into(), "2".into(), "3".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddDictionary {
            name: "Names".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let mapping = frame_named(store.document(), "Names").clone();
    store
        .apply(Operation::PasteCells {
            frame_id: mapping.id.clone(),
            row_id: mapping.rows[0].id.clone(),
            column_id: mapping.columns[0].id.clone(),
            grid: vec![
                vec!["A".into(), replacement.into()],
                vec!["B".into(), "A".into()],
            ],
        })
        .unwrap();
    let target = frame_named(store.document(), "Data").clone();
    (store, target, mapping)
}

fn rename(target: &FrameObject, mapping: &FrameObject) -> Operation {
    Operation::RenameColumnsUsingMapping {
        frame_id: target.id.clone(),
        mapping_frame_id: mapping.id.clone(),
        key_column_id: mapping.columns[0].id.clone(),
        value_column_id: mapping.columns[1].id.clone(),
    }
}

#[test]
fn header_mapping_swaps_names_preserves_ids_and_undo_restores_the_batch() {
    let (mut store, target, mapping) = setup("B");
    store.apply(rename(&target, &mapping)).unwrap();
    let renamed = frame_named(store.document(), "Data");
    assert_eq!(
        renamed
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["B", "A", "C"]
    );
    assert_eq!(
        renamed.columns.iter().map(|c| &c.id).collect::<Vec<_>>(),
        target.columns.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    store.undo();
    assert_eq!(
        frame_named(store.document(), "Data").columns,
        target.columns
    );
}

#[test]
fn header_mapping_rejects_missing_names_and_collisions_atomically() {
    for invalid in ["", "C"] {
        let (mut store, target, mapping) = setup(invalid);
        assert!(store.apply(rename(&target, &mapping)).is_err());
        assert_eq!(
            frame_named(store.document(), "Data").columns,
            target.columns
        );
    }
}

#[test]
fn header_mapping_reads_a_derived_mapping_and_is_a_one_time_edit() {
    let (mut store, target, mapping) = setup("B");
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: mapping.id.clone(),
            name: "Live names".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let live = frame_named(store.document(), "Live names").clone();
    store
        .apply(Operation::SetUniqueKey {
            frame_id: live.id.clone(),
            column_ids: vec![live.columns[0].id.clone()],
            enabled: true,
        })
        .unwrap();
    store.apply(rename(&target, &live)).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: mapping.id.clone(),
            row_id: mapping.rows[0].id.clone(),
            column_id: mapping.columns[1].id.clone(),
            raw: "Changed later".into(),
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Data").columns[0].name, "B");
}
