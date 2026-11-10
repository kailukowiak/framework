//! Growing and restructuring a grid the way a spreadsheet can: a row inserted
//! where the pointer is, several rows deleted as one edit, a column inserted
//! at the far left, and a paste that brings its own columns with it.
//!
//! Each gesture is pinned on both kinds of frame it can meet — one whose rows
//! the document holds, and one whose rows are read from a file and edited as
//! patches — because the same key press means a different write on each, and
//! the refusal on the second has to say why in words.

use crate::common::*;
use framework_core::*;
use std::{collections::BTreeMap, fs, path::Path};

fn inputs_store() -> (Store, Id) {
    let mut store = demo_store();
    store
        .apply(Operation::AddFrame {
            name: "Inputs".into(),
            grid: vec![
                vec!["Qty".into(), "Price".into()],
                vec!["2".into(), "5".into()],
                vec!["3".into(), "7".into()],
                vec!["4".into(), "9".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Inputs").id.clone();
    (store, frame_id)
}

fn inputs(store: &Store) -> FrameObject {
    frame_named(store.document(), "Inputs").clone()
}

/// The first column's raw values, top to bottom, in stored order.
fn first_column(store: &Store) -> Vec<String> {
    let frame = inputs(store);
    let column = frame.columns[0].id.clone();
    frame
        .rows
        .iter()
        .map(|row| row.cells[&column].raw.clone())
        .collect()
}

fn values(frame: &FrameObject, raw: &str) -> BTreeMap<Id, String> {
    BTreeMap::from([(frame.columns[0].id.clone(), raw.to_string())])
}

fn open_file(dir: &Path) -> (Store, FrameObject) {
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\nC,3\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: Some(path.display().to_string()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    (store, frame)
}

fn page_column(store: &Store, frame_id: &str, index: usize) -> Vec<String> {
    store
        .get_frame_page(frame_id, 0, 1000)
        .unwrap()
        .rows
        .into_iter()
        .map(|row| row[index].clone())
        .collect()
}

// ---- Inserting a row at a position -------------------------------------

#[test]
fn a_row_inserted_before_another_lands_above_it_and_undo_takes_it_out() {
    let (mut store, frame_id) = inputs_store();
    let frame = inputs(&store);
    store
        .apply(Operation::AddRow {
            frame_id: frame_id.clone(),
            values: values(&frame, "9"),
            before_row_id: Some(frame.rows[1].id.clone()),
        })
        .unwrap();
    assert_eq!(first_column(&store), ["2", "9", "3", "4"]);
    // The page reader honours the stored order too: it is what the grid,
    // Scratchwork's ordinal references and every downstream frame read.
    assert_eq!(page_column(&store, &frame_id, 0), ["2", "9", "3", "4"]);

    store.undo();
    assert_eq!(first_column(&store), ["2", "3", "4"]);
    store.redo();
    assert_eq!(first_column(&store), ["2", "9", "3", "4"]);
}

#[test]
fn a_row_inserted_before_the_first_becomes_the_first() {
    let (mut store, frame_id) = inputs_store();
    let frame = inputs(&store);
    store
        .apply(Operation::AddRow {
            frame_id,
            values: values(&frame, "1"),
            before_row_id: Some(frame.rows[0].id.clone()),
        })
        .unwrap();
    assert_eq!(first_column(&store), ["1", "2", "3", "4"]);
}

#[test]
fn inserting_before_a_row_that_is_not_there_is_refused() {
    let (mut store, frame_id) = inputs_store();
    let frame = inputs(&store);
    let error = store
        .apply(Operation::AddRow {
            frame_id,
            values: values(&frame, "1"),
            before_row_id: Some("no-such-row".into()),
        })
        .unwrap_err();
    assert!(matches!(error, CoreError::RowNotFound), "{error:?}");
}

#[test]
fn undoing_the_delete_of_the_first_row_puts_it_back_first() {
    // The inverse used to name the row *after* which to restore, and the
    // first row has none, so it came back at the bottom.
    let (mut store, frame_id) = inputs_store();
    let first = inputs(&store).rows[0].id.clone();
    store
        .apply(Operation::DeleteRow {
            frame_id,
            row_id: first,
        })
        .unwrap();
    assert_eq!(first_column(&store), ["3", "4"]);
    store.undo();
    assert_eq!(first_column(&store), ["2", "3", "4"]);
}

#[test]
fn a_file_backed_frame_refuses_a_positioned_insert_but_still_appends() {
    let dir = temporary_test_directory("grid-structure-file-insert");
    let (mut store, frame) = open_file(&dir);
    let row_ids = store.get_frame_page(&frame.id, 0, 10).unwrap().row_ids;
    let error = store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::new(),
            before_row_id: Some(row_ids[1].clone()),
        })
        .unwrap_err();
    assert!(error.to_string().contains("added at the end"), "{error}");
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::new(),
            before_row_id: None,
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().total_rows,
        4
    );
}

/// An ordinal cell reference is spelled `head(n).last()`: it reads whatever
/// row is n-th, not a particular row. Deleting a row above it has always
/// moved it onto the next row; inserting one above it moves it the same way.
/// This pins that the two agree, and that neither rewrites the formula — a
/// stable cell address needs the row's id in the expression, which is the
/// change AGENTS.md describes, not this one.
#[test]
fn an_ordinal_cell_reference_follows_position_on_insert_as_it_does_on_delete() {
    let read_second = |store: &mut Store| {
        let holder = a_container(store);
        store
            .apply(Operation::AddResult {
                name: "Second".into(),
                formula: "`Inputs`.`Qty`.head(2).last()".into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(holder),
            })
            .unwrap();
        store
            .document()
            .objects
            .iter()
            .find(|object| object.name() == "Second")
            .unwrap()
            .id()
            .to_string()
    };
    let answer = |store: &Store, result: &str| store.view().computed_results[result].cell.value;

    let (mut store, frame_id) = inputs_store();
    let result = read_second(&mut store);
    assert_eq!(answer(&store, &result), Some(3.0));
    let frame = inputs(&store);
    store
        .apply(Operation::AddRow {
            frame_id: frame_id.clone(),
            values: values(&frame, "9"),
            before_row_id: Some(frame.rows[1].id.clone()),
        })
        .unwrap();
    assert_eq!(answer(&store, &result), Some(9.0));

    let (mut store, frame_id) = inputs_store();
    let result = read_second(&mut store);
    let first = inputs(&store).rows[0].id.clone();
    store
        .apply(Operation::DeleteRow {
            frame_id,
            row_id: first,
        })
        .unwrap();
    assert_eq!(answer(&store, &result), Some(4.0));
}

// ---- Deleting several rows as one edit ----------------------------------

#[test]
fn several_rows_go_in_one_edit_and_come_back_exactly_in_one_undo() {
    let (mut store, frame_id) = inputs_store();
    let before = inputs(&store);
    store
        .apply(Operation::DeleteRows {
            frame_id: frame_id.clone(),
            // Out of order and repeated, the way a selection gathered by
            // shift- and command-clicks can arrive.
            row_ids: vec![
                before.rows[2].id.clone(),
                before.rows[0].id.clone(),
                before.rows[2].id.clone(),
            ],
        })
        .unwrap();
    assert_eq!(first_column(&store), ["3"]);

    store.undo();
    let after = inputs(&store);
    assert_eq!(after.rows, before.rows);
}

#[test]
fn deleting_every_row_and_undoing_restores_the_order() {
    let (mut store, frame_id) = inputs_store();
    let before = inputs(&store);
    store
        .apply(Operation::DeleteRows {
            frame_id,
            row_ids: before.rows.iter().map(|row| row.id.clone()).collect(),
        })
        .unwrap();
    assert!(inputs(&store).rows.is_empty());
    store.undo();
    assert_eq!(inputs(&store).rows, before.rows);
}

#[test]
fn deleting_rows_refuses_an_unknown_row_and_an_empty_list() {
    let (mut store, frame_id) = inputs_store();
    let known = inputs(&store).rows[0].id.clone();
    let unknown = store
        .apply(Operation::DeleteRows {
            frame_id: frame_id.clone(),
            row_ids: vec![known, "no-such-row".into()],
        })
        .unwrap_err();
    assert!(matches!(unknown, CoreError::RowNotFound), "{unknown:?}");
    assert_eq!(first_column(&store), ["2", "3", "4"]);
    let empty = store
        .apply(Operation::DeleteRows {
            frame_id,
            row_ids: Vec::new(),
        })
        .unwrap_err();
    assert!(empty.to_string().contains("No rows"), "{empty}");
}

#[test]
fn several_rows_of_a_file_backed_frame_are_struck_out_as_one_edit() {
    let dir = temporary_test_directory("grid-structure-file-delete");
    let (mut store, frame) = open_file(&dir);
    let row_ids = store.get_frame_page(&frame.id, 0, 10).unwrap().row_ids;
    store
        .apply(Operation::DeleteRows {
            frame_id: frame.id.clone(),
            row_ids: vec![row_ids[2].clone(), row_ids[0].clone()],
        })
        .unwrap();
    assert_eq!(page_column(&store, &frame.id, 0), ["B"]);
    store.undo();
    assert_eq!(page_column(&store, &frame.id, 0), ["A", "B", "C"]);
}

// ---- Inserting a column before ------------------------------------------

#[test]
fn a_column_inserted_before_the_first_becomes_the_first() {
    let (mut store, frame_id) = inputs_store();
    let first = inputs(&store).columns[0].id.clone();
    store
        .apply(Operation::AddColumn {
            frame_id: frame_id.clone(),
            name: "Label".into(),
            data_type: DataType::String,
            after_column_id: None,
            before_column_id: Some(first.clone()),
            values: Default::default(),
        })
        .unwrap();
    let frame = inputs(&store);
    assert_eq!(frame.columns[0].name, "Label");
    assert_eq!(frame.columns[1].id, first);
    assert!(
        frame
            .rows
            .iter()
            .all(|row| row.cells.contains_key(&frame.columns[0].id))
    );
    store.undo();
    assert_eq!(inputs(&store).columns[0].id, first);
}

#[test]
fn a_column_cannot_be_both_before_and_after() {
    let (mut store, frame_id) = inputs_store();
    let frame = inputs(&store);
    let error = store
        .apply(Operation::AddColumn {
            frame_id,
            name: "Label".into(),
            data_type: DataType::String,
            after_column_id: Some(frame.columns[0].id.clone()),
            before_column_id: Some(frame.columns[1].id.clone()),
            values: Default::default(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("before or after"), "{error}");
}

// ---- A paste that widens the frame --------------------------------------

#[test]
fn a_wide_paste_adds_the_columns_it_needs_in_the_same_undo_entry() {
    let (mut store, frame_id) = inputs_store();
    let frame = inputs(&store);
    // Starting on Price, three columns wide and four rows tall: Price, two
    // new columns, and one new row.
    store
        .apply(Operation::PasteCells {
            frame_id: frame_id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[1].id.clone(),
            grid: vec![
                vec!["50".into(), "north".into(), "1.5".into()],
                vec!["70".into(), "south".into(), "2.5".into()],
                vec!["90".into(), "east".into(), "3".into()],
                vec!["10".into(), "west".into(), "4".into()],
            ],
        })
        .unwrap();
    let widened = inputs(&store);
    let names: Vec<_> = widened.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Qty", "Price", "New column", "New column 2"]);
    assert_eq!(widened.columns[2].data_type, DataType::String);
    assert!(
        matches!(widened.columns[3].data_type, DataType::Number),
        "{:?}",
        widened.columns[3].data_type
    );
    assert_eq!(widened.rows.len(), 4);
    let region = widened.columns[2].id.clone();
    let regions: Vec<_> = widened
        .rows
        .iter()
        .map(|row| row.cells[&region].raw.clone())
        .collect();
    assert_eq!(regions, ["north", "south", "east", "west"]);

    store.undo();
    assert_eq!(inputs(&store), frame);
}

#[test]
fn a_new_column_name_skips_names_already_taken() {
    let (mut store, frame_id) = inputs_store();
    store
        .apply(Operation::RenameColumn {
            frame_id: frame_id.clone(),
            column_id: inputs(&store).columns[0].id.clone(),
            name: "new column".into(),
        })
        .unwrap();
    let frame = inputs(&store);
    store
        .apply(Operation::PasteCells {
            frame_id,
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[1].id.clone(),
            grid: vec![vec!["1".into(), "2".into()]],
        })
        .unwrap();
    assert_eq!(inputs(&store).columns[2].name, "New column 2");
}

#[test]
fn a_wide_paste_on_a_file_backed_frame_is_still_refused_whole() {
    let dir = temporary_test_directory("grid-structure-file-wide-paste");
    let (mut store, frame) = open_file(&dir);
    let row_ids = store.get_frame_page(&frame.id, 0, 10).unwrap().row_ids;
    let error = store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: row_ids[0].clone(),
            column_id: frame.columns[1].id.clone(),
            grid: vec![vec!["5".into(), "extra".into()]],
        })
        .unwrap_err();
    assert!(error.to_string().starts_with("Nothing pasted"), "{error}");
    assert_eq!(page_column(&store, &frame.id, 1), ["1", "2", "3"]);
}

#[test]
fn a_wide_paste_over_a_calculated_column_is_still_refused_whole() {
    let (mut store, frame_id) = inputs_store();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "calc-total".into(),
                    name: "Total".into(),
                    formula: "`Qty` * `Price`".into(),
                }],
            }],
        })
        .unwrap();
    let frame = inputs(&store);
    let rows = store.get_frame_page(&frame_id, 0, 10).unwrap().row_ids;
    let error = store
        .apply(Operation::PasteCells {
            frame_id,
            row_id: rows[0].clone(),
            column_id: frame.columns[1].id.clone(),
            grid: vec![vec!["1".into(), "2".into(), "3".into()]],
        })
        .unwrap_err();
    assert!(error.to_string().contains("calculated"), "{error}");
    assert_eq!(inputs(&store).columns.len(), 3);
}

// ---- A single pasted number --------------------------------------------

#[test]
fn a_single_pasted_number_with_a_thousands_separator_is_one_cell() {
    let mut store = demo_store();
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "Pasted".into(),
            text: "1,234\n".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Pasted").columns.len(), 1);

    // Comma-separated text is still read as comma-separated.
    store
        .apply(Operation::AddFrameFromPastedText {
            name: "Csv".into(),
            text: "a,b\n1,2\n".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Csv").columns.len(), 2);
}

// ---- The paged count under a chain filter -------------------------------

#[test]
fn a_chain_filter_is_counted_on_a_file_backed_frame_with_added_rows() {
    let dir = temporary_test_directory("grid-structure-filter-count");
    let (mut store, frame) = open_file(&dir);
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: BTreeMap::new(),
            before_row_id: None,
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Count` > 1".into()],
                match_all: true,
            }],
        })
        .unwrap();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(page.rows.len(), 2);
    assert_eq!(page.total_rows, 2);
}
