//! Fixtures shared by the in-crate tests. Integration tests use the parallel
//! set in `tests/common/`; these exist because a handful of tests reach
//! private internals and so must compile inside the crate.

use crate::*;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub(crate) fn demo_store() -> Store {
    Store::new(Document::demo())
}

pub(crate) fn frame_named<'a>(document: &'a Document, name: &str) -> &'a FrameObject {
    document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.name == name => Some(frame),
            _ => None,
        })
        .unwrap()
}

pub(crate) fn temporary_test_directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("framework-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}
