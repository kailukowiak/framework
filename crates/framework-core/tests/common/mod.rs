use framework_core::{DataObject, Document, FrameObject, Operation, Store};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[allow(dead_code)]
pub fn demo_store() -> Store {
    Store::new(Document::demo())
}

/// A container to put a value, a result, or a list in.
///
/// Those three have no home on the bare canvas any more — a loose constant
/// belongs on a line of a formula block — so a test that wants one as an
/// object rather than as a line has to say where it lives. This is that
/// somewhere, and it is one call so the tests keep reading as tests.
#[allow(dead_code)]
pub fn a_container(store: &mut Store) -> String {
    // One per store, made on first ask: a member's name is written through
    // its container, so a fresh container per value would put a different
    // qualifier in front of every reference and make the formulas under test
    // unreadable.
    if let Some(existing) = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Holder")
    {
        return existing.id().to_string();
    }
    store
        .apply(Operation::AddContainer {
            name: "Holder".into(),
            x: 0.0,
            y: 0.0,
            container_id: None,
        })
        .unwrap();
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Holder")
        .unwrap()
        .id()
        .to_string()
}

#[allow(dead_code)]
pub fn frame_named<'a>(document: &'a Document, name: &str) -> &'a FrameObject {
    document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == name => Some(frame),
            _ => None,
        })
        .unwrap()
}

#[allow(dead_code)]
pub fn temporary_test_directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("framework-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}
