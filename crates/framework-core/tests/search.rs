//! Searching a frame nobody has in memory.
//!
//! The interesting rows are deliberately past the first page: a search that
//! only ever looked at the rows already fetched would pass a test written
//! against row three and fail every time someone actually used it.

use crate::common::*;
use framework_core::*;
use std::fs;
use std::path::PathBuf;

const ROW_COUNT: usize = 400;
/// Stored position of the one row whose name is not `Row{n}`.
const NEEDLE_ROW: usize = 317;

fn paged_search_fixture(directory_name: &str) -> (PathBuf, Store, Id, Id) {
    let directory = temporary_test_directory(directory_name);
    let source = directory.join("rows.csv");
    let mut contents = String::from("Name,Score\n");
    for index in 0..ROW_COUNT {
        let score = ROW_COUNT - index;
        if index == NEEDLE_ROW {
            contents.push_str(&format!("Fenwick Ltd,{score}\n"));
        } else {
            contents.push_str(&format!("Row{index},{score}\n"));
        }
    }
    fs::write(&source, contents).unwrap();

    let mut store = Store::new(Document::blank("Search"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Rows".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    assert!(
        store.view().computed_frames[&frame.id].paged,
        "the fixture must be read through pages, which is the case search exists for"
    );
    let name_id = frame.columns[0].id.clone();
    let score_id = frame.columns[1].id.clone();
    (directory, store, name_id, score_id)
}

fn frame_id(store: &Store) -> Id {
    frame_named(store.document(), "Rows").id.clone()
}

#[test]
fn finds_a_value_far_past_the_first_page_ignoring_case() {
    let (directory, store, name_id, _) = paged_search_fixture("search-past-first-page");
    let hits = store
        .search_frame_rows(&frame_id(&store), "FENWICK", 20)
        .unwrap();

    assert_eq!(hits.len(), 1, "one cell holds the name, so one hit");
    assert_eq!(hits[0].column_id, name_id);
    assert_eq!(hits[0].row_index, NEEDLE_ROW as u64);
    assert_eq!(hits[0].snippet, "Fenwick Ltd");
    assert_eq!(
        hits[0].row_id.as_deref(),
        Some(format!("source:{}:{}", frame_id(&store), NEEDLE_ROW).as_str()),
        "the identity must be spelled the way a page read spells it"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_substring_matches_across_columns_and_stops_at_the_limit() {
    let (directory, store, name_id, score_id) = paged_search_fixture("search-limit");
    // "31" is in `Row31`, `Row131`, … and in plenty of scores, so both
    // columns answer and there are far more than five hits available.
    let hits = store.search_frame_rows(&frame_id(&store), "31", 5).unwrap();

    assert_eq!(hits.len(), 5, "the limit is a count of hits, not of rows");
    assert!(
        hits.iter()
            .all(|hit| hit.column_id == name_id || hit.column_id == score_id),
        "every hit names a real column"
    );
    assert!(
        hits.iter().all(|hit| hit.snippet.contains("31")),
        "the snippet is the matching cell's own text"
    );
    assert!(
        hits.windows(2)
            .all(|pair| pair[0].row_index <= pair[1].row_index),
        "hits arrive in display order"
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_row_index_is_the_displayed_position_after_a_sort() {
    let (directory, mut store, _, score_id) = paged_search_fixture("search-sorted");
    let id = frame_id(&store);
    // Score counts down with the stored order, so sorting up on it reverses
    // the frame: the needle row is now counted from the other end.
    store
        .apply(Operation::SetFrameDisplaySort {
            frame_id: id.clone(),
            keys: vec![DerivedSort {
                column_id: score_id,
                descending: false,
            }],
        })
        .unwrap();

    let hits = store.search_frame_rows(&id, "fenwick", 20).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0].row_index,
        (ROW_COUNT - 1 - NEEDLE_ROW) as u64,
        "a hit says where to scroll, which is a position in the sorted frame"
    );
    // A frame read out of a file has no stored row identity to keep: its rows
    // are positions, and a sort moves them. What matters is that a hit spells
    // the position the same way the page the grid will load spells it, so the
    // focused cell is found rather than merely scrolled past.
    let display_position = ROW_COUNT - 1 - NEEDLE_ROW;
    let page = store.get_frame_page(&id, display_position, 1).unwrap();
    assert_eq!(page.rows[0][0], "Fenwick Ltd");
    assert_eq!(hits[0].row_id.as_deref(), Some(page.row_ids[0].as_str()));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn an_empty_query_matches_nothing_rather_than_everything() {
    let (directory, store, _, _) = paged_search_fixture("search-empty");
    assert!(
        store
            .search_frame_rows(&frame_id(&store), "   ", 20)
            .unwrap()
            .is_empty()
    );

    fs::remove_dir_all(directory).unwrap();
}
