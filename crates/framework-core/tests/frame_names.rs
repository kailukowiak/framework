use framework_core::*;
use std::collections::HashSet;

fn frame_names(store: &Store) -> Vec<&str> {
    store
        .document()
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.name.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn frame_creation_and_renaming_keep_formula_addresses_unique() {
    let mut store = Store::new(Document::blank("Frame names"));
    for name in [
        "Frame 1",
        "Frame 1",
        "Orders",
        "Orders",
        "Orders",
        "Warehouse",
    ] {
        store
            .apply(Operation::AddFrame {
                name: name.into(),
                grid: vec![vec!["Value".into()]],
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }

    assert_eq!(
        frame_names(&store),
        [
            "Frame 1",
            "Frame 2",
            "Orders",
            "Orders_2",
            "Orders_3",
            "Warehouse",
        ]
    );

    let warehouse_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == "Warehouse" => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();
    store
        .apply(Operation::RenameObject {
            object_id: warehouse_id.clone(),
            name: "Orders".into(),
        })
        .unwrap();
    assert_eq!(
        store.document().frame(&warehouse_id).unwrap().name,
        "Orders_4"
    );

    // Keeping an existing name is not a collision with the frame itself.
    store
        .apply(Operation::RenameObject {
            object_id: warehouse_id.clone(),
            name: "Orders_4".into(),
        })
        .unwrap();
    assert_eq!(
        store.document().frame(&warehouse_id).unwrap().name,
        "Orders_4"
    );
}

#[test]
fn opening_an_older_document_repairs_duplicate_frame_names_in_place() {
    let mut document = Document::demo();
    for frame in document
        .objects
        .iter_mut()
        .filter_map(|object| match object {
            DataObject::Frame(frame) => Some(frame),
            _ => None,
        })
    {
        frame.name = "Imported".into();
    }

    let store = Store::new(document);
    let names = frame_names(&store);
    assert_eq!(names[0], "Imported");
    assert_eq!(names[1], "Imported_2");
    assert_eq!(names[2], "Imported_3");
    assert_eq!(
        names.len(),
        names.iter().copied().collect::<HashSet<_>>().len()
    );
}
