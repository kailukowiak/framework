//! The value checklist's source: a column's distinct values with counts.
//!
//! The delicate behaviour is "before the trailing filter": the picker writes
//! its choice into the chain as a Filter step, so the list it re-opens with
//! must be read from the chain *without* that predicate, or a value the
//! person unchecked would disappear from the list and could never be checked
//! again.

use crate::common::*;
use framework_core::*;
use std::fs;

fn frame_from(store: &mut Store, name: &str, grid: &[&[&str]]) -> (Id, Vec<Column>) {
    store
        .apply(Operation::AddFrame {
            name: name.into(),
            grid: grid
                .iter()
                .map(|row| row.iter().map(|cell| cell.to_string()).collect())
                .collect(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), name);
    (frame.id.clone(), frame.columns.clone())
}

fn text(value: &str, count: usize) -> DistinctValue {
    DistinctValue {
        value: DistinctScalar::Text(value.into()),
        count,
    }
}

#[test]
fn text_comes_back_commonest_first_with_the_blank_group_kept() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Regional sales",
        &[
            &["Region", "Units"],
            &["West", "1"],
            &["East", "2"],
            &["East", "3"],
            &["", "4"],
            &["West", "5"],
            &["East", "6"],
            &["Cape", "7"],
            &["Bay", "8"],
        ],
    );
    let found = store
        .distinct_values(&id, &columns[0].id, Vec::new(), 50, None)
        .unwrap();
    // East 3, West 2, then the singletons by label with the blank last.
    assert_eq!(found.total, 5);
    assert_eq!(found.values[0], text("East", 3));
    assert_eq!(found.values[1], text("West", 2));
    assert_eq!(found.values[2], text("Bay", 1));
    assert_eq!(found.values[3], text("Cape", 1));
    assert_eq!(found.values[4].value, DistinctScalar::Null);
    assert_eq!(found.values[4].count, 1);
}

#[test]
fn limit_is_clamped_and_total_is_the_whole_distinct_count() {
    let mut store = demo_store();
    let mut grid: Vec<Vec<String>> = vec![vec!["Key".into()]];
    for index in 0..300 {
        grid.push(vec![format!("k{index:03}")]);
    }
    store
        .apply(Operation::AddFrame {
            name: "Many".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Many").clone();
    let column = &frame.columns[0].id;
    let small = store
        .distinct_values(&frame.id, column, Vec::new(), 2, None)
        .unwrap();
    assert_eq!((small.values.len(), small.total), (2, 300));
    let huge = store
        .distinct_values(&frame.id, column, Vec::new(), 100_000, None)
        .unwrap();
    assert_eq!((huge.values.len(), huge.total), (200, 300));
    let zero = store
        .distinct_values(&frame.id, column, Vec::new(), 0, None)
        .unwrap();
    assert_eq!(zero.values.len(), 1);
}

#[test]
fn numbers_keep_their_fractions_and_booleans_stay_booleans() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Mixed",
        &[
            &["Price", "Live"],
            &["1.5", "true"],
            &["1.5", "false"],
            &["19.99", "true"],
            &["-2", "true"],
        ],
    );
    let prices = store
        .distinct_values(&id, &columns[0].id, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(
        prices
            .values
            .iter()
            .map(|entry| (entry.value.clone(), entry.count))
            .collect::<Vec<_>>(),
        vec![
            (DistinctScalar::Number(1.5), 2),
            (DistinctScalar::Number(-2.0), 1),
            (DistinctScalar::Number(19.99), 1),
        ]
    );
    let live = store
        .distinct_values(&id, &columns[1].id, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(live.values[0].value, DistinctScalar::Boolean(true));
    assert_eq!(live.values[0].count, 3);
    assert_eq!(live.values[1].value, DistinctScalar::Boolean(false));
}

/// A decimal (accounting) column lists its amounts as numbers, and the
/// predicate the picker writes from them keeps exactly those rows.
#[test]
fn an_accounting_column_lists_amounts_and_the_written_predicate_matches_them() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Ledger",
        &[&["Amount"], &["19.99"], &["5.25"], &["19.99"]],
    );
    store
        .apply(Operation::SetColumnType {
            frame_id: id.clone(),
            column_id: columns[0].id.clone(),
            data_type: DataType::Accounting,
            scale: None,
        })
        .unwrap();
    let found = store
        .distinct_values(&id, &columns[0].id, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(found.values[0].value, DistinctScalar::Number(19.99));
    assert_eq!(found.values[0].count, 2);
    store
        .apply(Operation::SetFramePipeline {
            frame_id: id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Amount`.is_in([19.99])".into()],
                match_all: true,
            }],
        })
        .unwrap();
    assert_eq!(store.get_frame_page(&id, 0, 10).unwrap().total_rows, 2);
}

#[test]
fn the_list_is_read_from_before_the_trailing_filter_it_wrote() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Regional sales",
        &[
            &["Region", "Units"],
            &["East", "1"],
            &["West", "2"],
            &["North", "3"],
            &["East", "4"],
        ],
    );
    let region = columns[0].id.clone();
    let picked = FrameStepInput::Filter {
        predicates: vec![r#"`Region`.is_in(["East"])"#.into()],
        match_all: true,
    };
    store
        .apply(Operation::SetFramePipeline {
            frame_id: id.clone(),
            steps: vec![picked.clone()],
        })
        .unwrap();
    // Through the saved chain, the unchecked values are gone ...
    let filtered = store
        .distinct_values(&id, &region, vec![picked], 10, None)
        .unwrap();
    assert_eq!(filtered.total, 1);
    // ... which is why the picker asks for the chain without its predicate.
    let before = store
        .distinct_values(&id, &region, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(before.total, 3);
    assert_eq!(before.values[0], text("East", 2));
}

#[test]
fn the_displays_own_filter_does_not_hide_values() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Regional sales",
        &[&["Region"], &["East"], &["West"], &["East"]],
    );
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: id.clone(),
            filters: vec![r#"`Region` == "West""#.into()],
            filter_match_all: true,
        })
        .unwrap();
    let found = store
        .distinct_values(&id, &columns[0].id, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(found.total, 2);
}

#[test]
fn a_paged_import_lists_values_from_the_whole_file() {
    let directory = temporary_test_directory("distinct-paged");
    let source = directory.join("rows.csv");
    let mut contents = String::from("Tag,Score\n");
    for index in 0..3000 {
        contents.push_str(&format!("{},{}\n", ["a", "b", "c"][index % 3], index));
    }
    fs::write(&source, contents).unwrap();
    let mut store = Store::new(Document::blank("Paged"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Rows".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Rows").clone();
    assert!(store.view().computed_frames[&frame.id].paged);
    let found = store
        .distinct_values(&frame.id, &frame.columns[0].id, Vec::new(), 10, None)
        .unwrap();
    assert_eq!(found.total, 3);
    assert_eq!(found.values[0], text("a", 1000));
    // Score is unique per row, so the cap bites and the total still says so.
    let scores = store
        .distinct_values(&frame.id, &frame.columns[1].id, Vec::new(), 200, None)
        .unwrap();
    assert_eq!((scores.values.len(), scores.total), (200, 3000));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn dates_and_unknown_columns_are_refused() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Dated",
        &[&["When"], &["2026-01-02"], &["2026-02-03"]],
    );
    store
        .apply(Operation::SetColumnType {
            frame_id: id.clone(),
            column_id: columns[0].id.clone(),
            data_type: DataType::Date,
            scale: None,
        })
        .unwrap();
    assert!(
        store
            .distinct_values(&id, &columns[0].id, Vec::new(), 10, None)
            .is_err()
    );
    assert!(
        store
            .distinct_values(&id, "nope", Vec::new(), 10, None)
            .is_err()
    );
}

/// The checklist lists the commonest two hundred. A name further down —
/// "Zelda" among five thousand — has to be reachable by typing it, so the
/// query narrows the whole distinct list before the limit is taken, and
/// `total` then counts what matched.
#[test]
fn a_query_matches_the_whole_column_before_the_limit() {
    let mut store = demo_store();
    let mut grid: Vec<Vec<String>> = vec![vec!["Name".into()]];
    for index in 0..5_000 {
        grid.push(vec![format!("name{index:04}")]);
        // The common names fill the first page so the rare one is off it.
        if index < 300 {
            grid.push(vec![format!("name{index:04}")]);
        }
    }
    grid.push(vec!["Zelda".into()]);
    grid.push(vec!["".into()]);
    store
        .apply(Operation::AddFrame {
            name: "People".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "People").clone();
    let column = &frame.columns[0].id;

    let page = store
        .distinct_values(&frame.id, column, Vec::new(), 200, None)
        .unwrap();
    assert_eq!(page.total, 5_002);
    assert!(
        !page
            .values
            .iter()
            .any(|entry| entry.value == DistinctScalar::Text("Zelda".into()))
    );

    // Case-insensitive, substring, and the blank group has no text to match.
    let found = store
        .distinct_values(&frame.id, column, Vec::new(), 200, Some("ELD"))
        .unwrap();
    assert_eq!(found.total, 1);
    assert_eq!(found.values, vec![text("Zelda", 1)]);

    // Still commonest first, still capped, total is the matching count.
    let many = store
        .distinct_values(&frame.id, column, Vec::new(), 200, Some("name1"))
        .unwrap();
    assert_eq!(many.total, 1_000);
    assert_eq!(many.values.len(), 200);
    assert_eq!(many.values[0], text("name1000", 1));

    // An empty or blank query is no query.
    let blank = store
        .distinct_values(&frame.id, column, Vec::new(), 200, Some("  "))
        .unwrap();
    assert_eq!(blank.total, 5_002);
}

/// Numbers and booleans match on the text the checklist shows for them.
#[test]
fn a_query_matches_numbers_and_booleans_by_their_text() {
    let mut store = demo_store();
    let (id, columns) = frame_from(
        &mut store,
        "Typed",
        &[
            &["Price", "Live"],
            &["1.5", "true"],
            &["15", "false"],
            &["2", "true"],
        ],
    );
    let prices = store
        .distinct_values(&id, &columns[0].id, Vec::new(), 10, Some("5"))
        .unwrap();
    assert_eq!(prices.total, 2);
    let live = store
        .distinct_values(&id, &columns[1].id, Vec::new(), 10, Some("TRU"))
        .unwrap();
    assert_eq!(live.values.len(), 1);
    assert_eq!(live.values[0].value, DistinctScalar::Boolean(true));
}
