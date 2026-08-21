//! The scratchpad: one card of text, one answer per line.

#[allow(unused_imports)]
use crate::common::*;
use framework_core::*;

fn blank_store() -> Store {
    Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Blocks".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    })
}

fn add_block(store: &mut Store, name: &str) -> String {
    store
        .apply(Operation::AddBlock {
            name: name.into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    block_named(store, name).id.clone()
}

fn block_named<'a>(store: &'a Store, name: &str) -> &'a BlockObject {
    store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Block(block) if block.name == name => Some(block),
            _ => None,
        })
        .unwrap()
}

fn line_id(store: &Store, block_name: &str, line_name: &str) -> String {
    block_named(store, block_name)
        .lines
        .iter()
        .find(|line| line.name == line_name)
        .unwrap()
        .id
        .clone()
}

fn type_into(store: &mut Store, block_id: &str, source: &str) -> Result<(), CoreError> {
    edit(store, block_id, source, None)
}

/// The same, with the cursor said to be on `line` — what the editor sends
/// while somebody is still typing there.
fn typing_on(
    store: &mut Store,
    block_id: &str,
    source: &str,
    line: usize,
) -> Result<(), CoreError> {
    edit(store, block_id, source, Some(line))
}

fn edit(
    store: &mut Store,
    block_id: &str,
    source: &str,
    editing: Option<usize>,
) -> Result<(), CoreError> {
    store
        .apply(Operation::SetBlockSource {
            block_id: block_id.into(),
            source: source.into(),
            editing,
        })
        .map(|_| ())
}

/// Every line's answer, in order, with an error shown as `!`.
fn answers(store: &Store, block_id: &str) -> Vec<String> {
    store.view().computed_blocks[block_id]
        .lines
        .iter()
        .map(|line| match &line.cell.error {
            Some(_) => "!".to_string(),
            None => line.cell.display.clone(),
        })
        .collect()
}

fn error_on(store: &Store, block_id: &str, line_name: &str) -> String {
    store.view().computed_blocks[block_id]
        .lines
        .iter()
        .find(|line| line.name == line_name)
        .unwrap()
        .cell
        .error
        .clone()
        .unwrap_or_default()
}

#[test]
fn a_block_starts_empty_and_takes_the_text_typed_into_it() {
    let mut store = blank_store();
    let block = add_block(&mut store, "General calculations");
    assert!(block_named(&store, "General calculations").lines.is_empty());

    type_into(&mut store, &block, "x = 10\ny = 30\nx + y").unwrap();
    assert_eq!(answers(&store, &block), ["10", "30", "40"]);

    // The names typed are the names kept, and the unnamed line still gets
    // one so that it can be referred to at all.
    let names: Vec<_> = block_named(&store, "General calculations")
        .lines
        .iter()
        .map(|line| line.name.clone())
        .collect();
    assert_eq!(names, ["x", "y", "line_1"]);

    // And the text comes back out as it went in.
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "x = 10\ny = 30\nx + y"
    );
}

#[test]
fn backticked_names_work_and_round_trip_like_wrangle_names() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Quoted calculations");
    type_into(
        &mut store,
        &block,
        "`down payment` = 40000\n`double payment` = `down payment` * 2\n`rate `` special` = 5%",
    )
    .unwrap();

    assert_eq!(answers(&store, &block), ["40000", "80000", "5%"]);
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "`down payment` = 40000\n`double payment` = `down payment` * 2\n`rate `` special` = 5%"
    );
    let lines = &block_named(&store, "Quoted calculations").lines;
    assert_eq!(lines[0].name, "down payment");
    assert!(lines.iter().all(|line| line.name_quoted));
}

#[test]
fn a_long_answer_can_be_read_and_copied_past_the_gutter_preview() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Long answer");
    type_into(&mut store, &block, "values = [1, 2, 3, 4, 5, 6, 7, 8]").unwrap();
    let line = line_id(&store, "Long answer", "values");

    assert!(answers(&store, &block)[0].contains("…"));
    let first = store.get_block_line_page(&block, &line, 0, 3).unwrap();
    assert_eq!(first.total_values, 8);
    assert_eq!(first.values, ["1", "2", "3"]);
    let second = store.get_block_line_page(&block, &line, 3, 10).unwrap();
    assert_eq!(second.values, ["4", "5", "6", "7", "8"]);
}

#[test]
fn sequence_makes_integer_and_decimal_lists_with_an_exclusive_stop() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Sequences");
    type_into(
        &mut store,
        &block,
        "counting = sequence(5)\nodd = sequence(1, 8, step=2)\nquarters = sequence(0, 1, 0.25)",
    )
    .unwrap();

    let counting = line_id(&store, "Sequences", "counting");
    assert_eq!(
        store
            .get_block_line_page(&block, &counting, 0, 20)
            .unwrap()
            .values,
        ["0", "1", "2", "3", "4"]
    );
    let odd = line_id(&store, "Sequences", "odd");
    assert_eq!(
        store
            .get_block_line_page(&block, &odd, 0, 20)
            .unwrap()
            .values,
        ["1", "3", "5", "7"]
    );
    let quarters = line_id(&store, "Sequences", "quarters");
    assert_eq!(
        store
            .get_block_line_page(&block, &quarters, 0, 20)
            .unwrap()
            .values,
        ["0.00", "0.25", "0.50", "0.75"]
    );
}

#[test]
fn sequence_uses_calendar_durations_for_dates() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Calendar");
    type_into(
        &mut store,
        &block,
        "months = sequence(2026-01-31, 2026-05-01, 1mo)\nbackwards = sequence(2026-05-30, 2026-01-31, -1mo)",
    )
    .unwrap();

    let months = line_id(&store, "Calendar", "months");
    assert_eq!(
        store
            .get_block_line_page(&block, &months, 0, 20)
            .unwrap()
            .values,
        ["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30"]
    );
    let backwards = line_id(&store, "Calendar", "backwards");
    assert_eq!(
        store
            .get_block_line_page(&block, &backwards, 0, 20)
            .unwrap()
            .values,
        ["2026-05-30", "2026-04-30", "2026-03-30", "2026-02-28"]
    );
}

#[test]
fn sequence_refuses_steps_that_cannot_reach_the_stop() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Bad sequences");
    type_into(
        &mut store,
        &block,
        "zero = sequence(0, 10, 0)\nwrong_way = sequence(10, 0, 1)\ndate_wrong_way = sequence(2026-01-01, 2026-04-01, -1mo)\ntoo_many = sequence(0, 1000002)",
    )
    .unwrap();

    assert!(error_on(&store, &block, "zero").contains("zero"));
    assert!(error_on(&store, &block, "wrong_way").contains("negative"));
    assert!(error_on(&store, &block, "date_wrong_way").contains("positive"));
    assert!(error_on(&store, &block, "too_many").contains("1000000"));
}

#[test]
fn a_line_that_does_not_parse_is_kept_and_complains_in_its_own_gutter() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    // The middle line is what half-typed looks like.
    type_into(&mut store, &block, "x = 10\ny = 3 *\nx * 2").unwrap();

    assert_eq!(answers(&store, &block), ["10", "!", "20"]);
    assert!(!error_on(&store, &block, "y").is_empty());
    // The text survived, which is the whole point of keeping it.
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "x = 10\ny = 3 *\nx * 2"
    );
}

#[test]
fn a_line_reading_a_broken_line_says_so_rather_than_going_quiet() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 3 *\ny = x + 1").unwrap();
    assert_eq!(answers(&store, &block), ["!", "!"]);
    assert!(error_on(&store, &block, "y").contains('x'));
}

#[test]
fn blank_lines_and_comments_are_kept_and_answer_nothing() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(
        &mut store,
        &block,
        "# balance read off the bank site, 2026-08-11\nbalance = 1200\n\nbalance / 2",
    )
    .unwrap();

    let computed = store.view().computed_blocks[&block].clone();
    assert!(computed.lines[0].comment);
    assert!(computed.lines[2].blank);
    assert_eq!(computed.lines[1].cell.display, "1200");
    assert_eq!(computed.lines[3].cell.display, "600.00");
    // Neither prose nor space is addressable.
    assert_eq!(computed.lines[0].name, "");
    assert_eq!(computed.lines[2].name, "");
}

#[test]
fn a_line_may_only_read_the_lines_above_it() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    // Reading downward is typed, kept, and refused an answer.
    type_into(&mut store, &block, "a = b + 1\nb = 2").unwrap();
    assert_eq!(answers(&store, &block), ["!", "2"]);
    assert!(error_on(&store, &block, "a").contains("above"));

    // Swap them and it computes, without anything having to be re-typed.
    type_into(&mut store, &block, "b = 2\na = b + 1").unwrap();
    assert_eq!(answers(&store, &block), ["2", "3"]);
}

#[test]
fn a_line_defined_in_terms_of_itself_is_refused_an_answer() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = x + 1").unwrap();
    assert_eq!(answers(&store, &block), ["!"]);
    assert!(error_on(&store, &block, "x").contains("itself"));
}

#[test]
fn a_loop_out_through_a_result_and_back_is_refused_an_answer() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "base = 10").unwrap();
    store
        .apply(Operation::AddResult {
            name: "Doubled".into(),
            formula: "`Scratch`.`base` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();

    // Now close the loop from the block's side. The edit is taken — this is
    // a draft surface — but the line does not run.
    type_into(&mut store, &block, "base = `Doubled` / 2").unwrap();
    assert_eq!(answers(&store, &block), ["!"]);
    assert!(error_on(&store, &block, "base").contains("itself"));
}

#[test]
fn renaming_a_line_rewrites_the_lines_that_read_it() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 10\ndouble = x * 2").unwrap();
    let kept = line_id(&store, "Scratch", "x");

    type_into(&mut store, &block, "price = 10\ndouble = x * 2").unwrap();

    // Same line, new name, and the line reading it now says the new name.
    assert_eq!(line_id(&store, "Scratch", "price"), kept);
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "price = 10\ndouble = price * 2"
    );
    assert_eq!(answers(&store, &block), ["10", "20"]);
}

/// The rewrite above is right, and doing it on every keystroke is not.
///
/// Somebody putting `10` on the end of `revenue` types `revenue1` before they
/// type `revenue10`, and a rename per prefix rewrites every line that reads
/// it, twice, under a cursor that is trying to hold still. So the line the
/// cursor is on keeps the name it already answered to, and the rename lands
/// once, when the cursor has gone.
#[test]
fn a_name_half_typed_is_not_yet_a_rename() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "revenue = 10\ndouble = revenue * 2").unwrap();
    let kept = line_id(&store, "Scratch", "revenue");

    // Both keystrokes, with the cursor on the line being typed. Nothing the
    // author did not type moves, and the block goes on working: the line
    // below still says `revenue`, and `revenue` is still what that line is
    // called until the cursor has gone somewhere else.
    for typed in ["revenue1 = 10", "revenue10 = 10"] {
        typing_on(
            &mut store,
            &block,
            &format!("{typed}\ndouble = revenue * 2"),
            0,
        )
        .unwrap();
        assert_eq!(
            store.view().computed_blocks[&block].source,
            "revenue = 10\ndouble = revenue * 2"
        );
        assert_eq!(line_id(&store, "Scratch", "revenue"), kept);
        assert_eq!(answers(&store, &block), ["10", "20"]);
    }

    // The cursor leaves, and now it is a rename.
    type_into(&mut store, &block, "revenue10 = 10\ndouble = revenue * 2").unwrap();
    assert_eq!(line_id(&store, "Scratch", "revenue10"), kept);
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "revenue10 = 10\ndouble = revenue10 * 2"
    );
    assert_eq!(answers(&store, &block), ["10", "20"]);
}

/// Holding a name is for the line under the cursor only — a rename finished
/// somewhere else on the way past still takes effect.
#[test]
fn a_name_is_only_held_on_the_line_being_typed() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 10\ndouble = x * 2\nspare = 1").unwrap();

    typing_on(
        &mut store,
        &block,
        "price = 10\ndouble = x * 2\nspare = 2",
        2,
    )
    .unwrap();

    assert_eq!(
        store.view().computed_blocks[&block].source,
        "price = 10\ndouble = price * 2\nspare = 2"
    );
}

/// A line nothing has read yet is named as it is typed. There is no older
/// name to protect, and waiting would only keep it out of its own block.
#[test]
fn a_new_line_is_named_as_it_is_typed() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    typing_on(&mut store, &block, "cost = 10", 0).unwrap();
    assert!(!line_id(&store, "Scratch", "cost").is_empty());
    assert_eq!(answers(&store, &block), ["10"]);
}

#[test]
fn a_line_keeps_its_identity_when_lines_move_around_it() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 10\ny = 20").unwrap();
    let x = line_id(&store, "Scratch", "x");
    let y = line_id(&store, "Scratch", "y");

    // A line inserted above both, and one edited: everything that was there
    // before is still the same line.
    type_into(&mut store, &block, "top = 1\nx = 11\ny = 20").unwrap();
    assert_eq!(line_id(&store, "Scratch", "x"), x);
    assert_eq!(line_id(&store, "Scratch", "y"), y);
    assert_eq!(answers(&store, &block), ["1", "11", "20"]);
}

#[test]
fn a_line_read_from_outside_the_block_cannot_be_typed_away() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "base = 10\nother = 2").unwrap();
    store
        .apply(Operation::AddResult {
            name: "Doubled".into(),
            formula: "`Scratch`.`base` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();

    let refused = type_into(&mut store, &block, "other = 2").unwrap_err();
    assert!(refused.to_string().contains("Doubled"));
    // Nothing moved.
    assert_eq!(answers(&store, &block), ["10", "2"]);

    // A line nothing outside reads goes without argument.
    type_into(&mut store, &block, "base = 10").unwrap();
    assert_eq!(answers(&store, &block), ["10"]);
}

#[test]
fn a_line_gets_the_whole_formula_surface_not_a_calculator_subset() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Rate".into(),
            raw: "0.08".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::AddResult {
            name: "Doubled rate".into(),
            formula: "`Rate` * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // A canvas value, a result, and a sibling — all in the same language
    // every other formula in this document is written in.
    type_into(
        &mut store,
        &block,
        "principal = 1000\ninterest = principal * `Rate`\ninterest / `Doubled rate`",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["1000", "80.00", "500.00"]);
}

#[test]
fn a_block_saved_before_it_was_text_still_reads_back_as_text() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 10\nx * 4").unwrap();

    // What a document written by the previous shape holds: a parsed formula
    // and no text at all.
    let mut document = store.document().clone();
    let DataObject::Block(stored) = document
        .objects
        .iter_mut()
        .find(|object| object.id() == block)
        .unwrap()
    else {
        panic!("the block is a block");
    };
    for line in &mut stored.lines {
        line.source = String::new();
    }
    let reopened = Store::new(document);

    assert_eq!(
        reopened.view().computed_blocks[&block].source,
        "x = 10\nline_1 = x * 4"
    );
    assert_eq!(answers(&reopened, &block), ["10", "40"]);
}

#[test]
fn integer_literals_keep_their_type_across_persistence() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "whole = 10\nexplicit_float = 10.0").unwrap();

    let directory = temporary_test_directory("integer-literal-persistence");
    let path = directory.join("integer-literals.fw");
    store.save(&path).unwrap();
    let reopened = Store::load(&path).unwrap();
    let lines = &reopened.view().computed_blocks[&block].lines;

    assert_eq!(lines[0].data_type, DataType::Integer);
    assert_eq!(lines[0].cell.display, "10");
    assert_eq!(lines[1].data_type, DataType::Number);
    assert_eq!(lines[1].cell.display, "10.00");
}

#[test]
fn a_line_reduces_a_frame_column_to_one_number() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Amount".into(), "Code".into()],
                vec!["100".into(), "A1".into()],
                vec!["20".into(), "B2".into()],
                vec!["5".into(), "C3".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Orders")
        .unwrap()
        .id()
        .to_string();
    let block = add_block(&mut store, "Scratch");

    // Scratchwork reads a document-owned frame live; a snapshot is not a
    // prerequisite for ordinary ad-hoc arithmetic.
    type_into(&mut store, &block, "`Orders`.`Amount`.sum()").unwrap();
    assert_eq!(answers(&store, &block), ["125"]);

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: frame_id,
            name: "Amounts".into(),
            group_keys: vec![
                NamedFormulaInput {
                    name: "Amount".into(),
                    formula: "`Amount`".into(),
                },
                NamedFormulaInput {
                    name: "Code".into(),
                    formula: "`Code`".into(),
                },
            ],
            aggregates: vec![NamedFormulaInput {
                name: "Rows".into(),
                formula: "`Amount`.count()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = block_free_frame(&store, "Amounts");
    let directory = temporary_test_directory("blocks-aggregate");
    store
        .materialize_frame(&derived_id, &directory.join("data"))
        .unwrap();

    // Reduced to one number, and usable as one: the line below reads it.
    type_into(
        &mut store,
        &block,
        "total = `Amounts`.`Amount`.sum()\ntotal / 5",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["125", "25.00"]);

    // And a column that cannot be added up says exactly that, without the
    // resolved Polars plan that used to be stapled to the end of it.
    type_into(&mut store, &block, "`Amounts`.`Code`.sum()").unwrap();
    assert_eq!(
        error_on(&store, &block, "line_1"),
        "`sum` operation not supported for dtype `str`"
    );
}

/// The id of a frame by name, without dragging in the shared helper's
/// document-shaped argument.
fn block_free_frame(store: &Store, name: &str) -> String {
    store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == name)
        .unwrap()
        .id()
        .to_string()
}

#[test]
fn retyping_a_block_undoes_in_one_step() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");
    type_into(&mut store, &block, "x = 10").unwrap();
    let original = line_id(&store, "Scratch", "x");

    type_into(&mut store, &block, "x = 10\ny = x * 4").unwrap();
    assert_eq!(answers(&store, &block), ["10", "40"]);

    store.undo();
    assert_eq!(answers(&store, &block), ["10"]);
    // The same line, not a copy of it — so anything pointing at it still is.
    assert_eq!(line_id(&store, "Scratch", "x"), original);

    store.redo();
    assert_eq!(answers(&store, &block), ["10", "40"]);
}

#[test]
fn a_line_reads_derived_frames_live_until_its_answer_is_explicitly_frozen() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["100".into()],
                vec!["20".into()],
                vec!["5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = block_free_frame(&store, "Orders");
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: frame_id.clone(),
            name: "Live".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Amount".into(),
                formula: "`Amount`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Rows".into(),
                formula: "`Amount`.count()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // Derived frames are live Scratchwork inputs too. Materialization and
    // freezing are performance/history choices, not requirements for seeing
    // the current calculation.
    type_into(&mut store, &block, "total = `Live`.`Amount`.sum()").unwrap();
    let line = line_id(&store, "Scratch", "total");
    assert_eq!(answers(&store, &block), ["125"]);

    let column_id = store.document().frame(&frame_id).unwrap().columns[0]
        .id
        .clone();
    store
        .apply(Operation::AddRow {
            frame_id: frame_id.clone(),
            values: [(column_id.clone(), "1000".to_string())]
                .into_iter()
                .collect(),
        })
        .unwrap();
    assert_eq!(answers(&store, &block), ["1125"]);

    let directory = temporary_test_directory("freeze-value");
    store.freeze_value(&line, &directory.join("data")).unwrap();
    assert_eq!(answers(&store, &block), ["1125"]);

    // And the card can say it is a written-down answer, and how old.
    let frozen = store.view().computed_blocks[&block].lines[0]
        .frozen
        .clone()
        .unwrap();
    assert!(!frozen.stale);
    assert!(!frozen.taken_at.is_empty());

    // Changing what it was taken from does not change the answer — it
    // reports that the answer is now stale.
    store
        .apply(Operation::AddRow {
            frame_id,
            values: [(column_id, "2000".to_string())].into_iter().collect(),
        })
        .unwrap();
    assert_eq!(answers(&store, &block), ["1125"]);
    assert!(
        store.view().computed_blocks[&block].lines[0]
            .frozen
            .as_ref()
            .unwrap()
            .stale
    );

    // Refreshing is freezing again.
    store.freeze_value(&line, &directory.join("data")).unwrap();
    assert_eq!(answers(&store, &block), ["3125"]);
    assert!(
        !store.view().computed_blocks[&block].lines[0]
            .frozen
            .as_ref()
            .unwrap()
            .stale
    );

    // Letting the recorded answer go returns to the current live calculation.
    store.thaw_value(&line).unwrap();
    assert_eq!(answers(&store, &block), ["3125"]);
}

#[test]
fn a_line_dependent_on_a_live_frame_aggregate_stays_live_too() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["100".into()],
                vec!["20".into()],
                vec!["5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = block_free_frame(&store, "Orders");
    let amount_id = store.document().frame(&frame_id).unwrap().columns[0]
        .id
        .clone();
    let block = add_block(&mut store, "Scratch");
    type_into(
        &mut store,
        &block,
        "total = `Orders`.`Amount`.sum()\ndouble = total * 2\namounts = `Orders`.`Amount`\ndoubled amounts = amounts * 2",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["125", "250", "[100, 20, 5]", "[200, 40, 10]"]
    );

    store
        .apply(Operation::AddRow {
            frame_id,
            values: [(amount_id, "75".to_string())].into_iter().collect(),
        })
        .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["200", "400", "[100, 20, 5, 75]", "[200, 40, 10, 150]"]
    );
}

#[test]
fn grouped_and_source_totals_subtract_to_one_value_and_survive_a_frame_rename() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Monthly sales".into(),
            grid: vec![
                vec!["Region".into(), "Revenue".into()],
                vec!["East".into(), "100".into()],
                vec!["West".into(), "20".into()],
                vec!["East".into(), "5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sales_id = block_free_frame(&store, "Monthly sales");
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: sales_id,
            name: "Monthly sales frame".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Region".into(),
                formula: "`Region`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Revenue Sum".into(),
                formula: "`Revenue`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let grouped_id = block_free_frame(&store, "Monthly sales frame");
    let block = add_block(&mut store, "Checks");
    type_into(&mut store, &block, "`Monthly sales frame`.`Revenue Sum`").unwrap();
    let line = line_id(&store, "Checks", "line_1");
    let directory = temporary_test_directory("grouped-total-check");
    store.freeze_value(&line, &directory.join("data")).unwrap();
    assert_eq!(answers(&store, &block), ["[105, 20]"]);

    // A recorded answer belongs to the expression that was recorded. Once
    // the expression changes, showing the old list beside the new pair of
    // aggregates is a wrong answer, even with a stale mark attached. The
    // edit thaws it, and the newly asked question evaluates live to a scalar.
    type_into(
        &mut store,
        &block,
        "`Monthly sales frame`.`Revenue Sum`.sum() - `Monthly sales`.`Revenue`.sum()",
    )
    .unwrap();
    assert!(!store.document().frozen_values.contains_key(&line));
    assert_eq!(answers(&store, &block), ["0"]);

    // Undo restores both the old formula and the answer frozen for it; redo
    // returns to the new, deliberately live calculation.
    store.undo();
    assert_eq!(answers(&store, &block), ["[105, 20]"]);
    assert!(store.document().frozen_values.contains_key(&line));
    store.redo();
    assert!(!store.document().frozen_values.contains_key(&line));
    assert_eq!(answers(&store, &block), ["0"]);

    store
        .apply(Operation::RenameObject {
            object_id: grouped_id,
            name: "Sales by Region".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "`Sales by Region`.`Revenue Sum`.sum() - `Monthly sales`.`Revenue`.sum()"
    );

    // A clicked cell from a row-stable, document-owned frame spells its row
    // selection in ordinary formula syntax, then folds to one value. The UI
    // does not offer this ordinal address for derived or otherwise moving
    // rows, though the formula language remains composable when written.
    type_into(
        &mut store,
        &block,
        "`Sales by Region`.`Revenue Sum`.sum() - `Monthly sales`.`Revenue`.sum()\n`Monthly sales`.`Revenue`.head(2).last()",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["0", "20"]);
}

#[test]
fn a_line_may_hold_a_list_and_broadcast_over_it() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Rates".into(),
            values: "1\n2\n3".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // A scalar meeting a list broadcasts, and the line holds the answer
    // rather than refusing to be one.
    type_into(&mut store, &block, "4 * `Rates`\n`Rates`.max()").unwrap();
    let computed = store.view().computed_blocks[&block].clone();
    assert_eq!(computed.lines[0].cell.error, None);
    assert_eq!(computed.lines[0].cell.display, "[4, 8, 12]");
    // Folding one down still gives a single answer.
    assert_eq!(computed.lines[1].cell.display, "3");

    // A list beside another list of the same length pairs up.
    type_into(&mut store, &block, "`Rates` + `Rates`").unwrap();
    assert_eq!(
        store.view().computed_blocks[&block].lines[0].cell.display,
        "[2, 4, 6]"
    );
}

#[test]
fn a_list_written_on_a_line_is_the_values_written_in_it() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");

    // Typing a list out is how a list gets made — there is no other way to
    // make one, and there is deliberately not going to be.
    type_into(&mut store, &block, "quarters = [1, 2, 3, 4]").unwrap();
    assert_eq!(answers(&store, &block), ["[1, 2, 3, 4]"]);

    // It is a list like any other, so everything a list already does, it
    // does: a scalar broadcasts over it, an aggregate folds it down, and a
    // list of the same length pairs up with it.
    type_into(
        &mut store,
        &block,
        "quarters = [1, 2, 3, 4]\nquarters * 10\nquarters.sum()\nquarters + [0, 0, 1, 1]",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[1, 2, 3, 4]", "[10, 20, 30, 40]", "10", "[1, 2, 4, 5]"]
    );

    // Dates are written the way dates are written everywhere else, and a
    // gap in a list is a gap rather than a value that closed up behind it.
    type_into(&mut store, &block, "[2026-01-01, 2026-04-01]\n[1, None, 3]").unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[2026-01-01, 2026-04-01]", "[1, —, 3]"]
    );
}

#[test]
fn a_list_holds_one_kind_of_value() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    let block = add_block(&mut store, "Scratch");

    // Values of one kind, and values of one family, both go in. Money and a
    // plain number are one number underneath and differ only in how they
    // are written, so a list of both is numbers and the writing is dropped
    // — the least-committed thing that is still true of every value in it.
    store
        .apply(Operation::AddValue {
            name: "Price".into(),
            raw: "$5".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    type_into(&mut store, &block, "[1, 2.5, 3]\n[`Price`, 1]").unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[1.00, 2.50, 3.00]", "[5.00, 1.00]"]
    );

    // Across families there is nothing true of both, so the line says so
    // where it was written rather than quietly making everything text.
    for (source, first, second) in [
        ("[1, \"a\"]", "an integer", "text"),
        ("[2026-01-01, 5]", "a date", "an integer"),
        ("[True, 1]", "a true/false value", "an integer"),
    ] {
        type_into(&mut store, &block, source).unwrap();
        let refused = error_on(&store, &block, "line_1");
        assert!(refused.contains(first), "{source}: {refused}");
        assert!(refused.contains(second), "{source}: {refused}");
    }

    // A gap is not a kind of value, so it disagrees with nothing.
    type_into(&mut store, &block, "[1, None, 3]").unwrap();
    assert_eq!(answers(&store, &block), ["[1, —, 3]"]);
}

#[test]
fn a_list_takes_the_methods_its_values_take() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");

    // A list handed an element-wise method is one object having something
    // done to it, not two objects lined up by position, so there is nothing
    // here for the alignment rule to prevent — and arithmetic on a list has
    // always been allowed, which is the same argument.
    type_into(
        &mut store,
        &block,
        "[1.4, 2.6].round(0)\n[-1, 2].abs()\n[1, 2, 3].is_in([1, 2])\n[\"a\", \"b\"].str.to_uppercase()",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[1.00, 3.00]", "[1, 2]", "[true, true, false]", "[A, B]"]
    );

    // Folding one down still answers with a single value.
    type_into(&mut store, &block, "[1, 2, 3].sum()").unwrap();
    assert_eq!(answers(&store, &block), ["6"]);
}

#[test]
fn text_joined_to_a_number_is_written_the_way_this_document_writes_it() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Price".into(),
            raw: "$5".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // The job this exists for: a label per value, from one line.
    type_into(
        &mut store,
        &block,
        "quarters = [1, 2, 3, 4]\nlabels = \"Q\" + quarters",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[1, 2, 3, 4]", "[Q1, Q2, Q3, Q4]"]
    );

    // And the writing is this document's, not Polars'. Polars would say
    // `1.0` for a whole number, `0.3333333333333333` for a third, and a
    // midnight timestamp for a date; the gutter says none of those, so
    // neither does a number inside a sentence. Money keeps its sign and a
    // rate keeps its, because those are the number and how it is written.
    type_into(
        &mut store,
        &block,
        "\"Q\" + 1\n\"x\" + 1/3\n\"as of \" + 2026-01-01\n\"is \" + True\n\"costs \" + `Holder`.`Price`\n\"up \" + 4.25%",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        [
            "Q1",
            "x0.3333",
            "as of 2026-01-01",
            "is true",
            "costs $5",
            "up 4.25%"
        ]
    );

    // A fold answers in the kind it folded, so the commonest scratch line
    // of all joins up too.
    type_into(&mut store, &block, "\"total \" + [10, 20, 30].sum()").unwrap();
    assert_eq!(answers(&store, &block), ["total 60"]);

    // Text on both sides is the join it always was, numbers on both sides
    // the arithmetic they always were.
    type_into(
        &mut store,
        &block,
        "\"a\" + \"b\"\n1 + 2\n\"v1.0\" + \"-beta\"",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["ab", "3", "v1.0-beta"]);
}

#[test]
fn a_line_can_write_down_a_whole_list_not_only_one_answer() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Ledger".into(),
            grid: vec![
                vec!["Difference".into()],
                vec!["10".into()],
                vec!["20".into()],
                vec!["30".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // Reading a column of a frame with no snapshot is a live list.
    type_into(
        &mut store,
        &block,
        "diffs = \"Row \" + `Ledger`.`Difference`",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["[Row 10, Row 20, Row 30]"]);

    // And what gets written down is the whole list. A line is the one place
    // an answer may be a list, so it is the one place a *recorded* answer
    // may be one — being told to fold twenty values into one is no use to
    // somebody who wanted the twenty.
    let line = line_id(&store, "Scratch", "diffs");
    let directory = temporary_test_directory("freeze-list");
    store.freeze_value(&line, &directory.join("data")).unwrap();
    assert_eq!(answers(&store, &block), ["[Row 10, Row 20, Row 30]"]);

    // Read back at its full length by anything that names it, too.
    type_into(
        &mut store,
        &block,
        "diffs = \"Row \" + `Ledger`.`Difference`\ndiffs.str.to_uppercase()",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["[Row 10, Row 20, Row 30]", "[ROW 10, ROW 20, ROW 30]"]
    );
}

#[test]
fn a_value_card_still_holds_only_one_answer() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddFrame {
            name: "Ledger".into(),
            grid: vec![
                vec!["Difference".into()],
                vec!["10".into()],
                vec!["20".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddResult {
            name: "Diffs".into(),
            formula: "`Ledger`.`Difference`".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    let result_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Result(result) => Some(result.id.clone()),
            _ => None,
        })
        .unwrap();

    // A card on the canvas is one value by construction, so writing a list
    // into one is refused where writing it into a line is not.
    let directory = temporary_test_directory("freeze-list-card");
    let refused = store
        .freeze_value(&result_id, &directory.join("data"))
        .unwrap_err();
    assert!(refused.to_string().contains("holds one value"), "{refused}");
    assert!(refused.to_string().contains("scratchpad"), "{refused}");
}

#[test]
fn cast_converts_and_format_fills_a_pattern() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratch");

    // Casting to text is a rendering, so it is the same writing `+` uses —
    // and once a number is text, every string method is available to it,
    // which is how a spelling gets controlled without a format language to
    // invent, learn and maintain.
    type_into(
        &mut store,
        &block,
        "quarters = [1, 2, 3]\nformat(\"Q{}\", quarters.cast(\"string\").str.zfill(2))",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["[1, 2, 3]", "[Q01, Q02, Q03]"]);

    // The conversions that are conversions rather than renderings.
    type_into(
        &mut store,
        &block,
        "\"42\".cast(\"number\") + 1\n\"2026-01-01\".cast(\"date\")\n2026-01-01.cast(\"string\")",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        ["43.00", "2026-01-01", "2026-01-01"]
    );

    // A pattern earns its place where there is more than one hole.
    type_into(
        &mut store,
        &block,
        "format(\"{} of {}\", 3, 12)\nformat(\"as of {}\", 2026-01-01)",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["3 of 12", "as of 2026-01-01"]);

    // And a pattern that does not match what follows it says so, rather
    // than quietly dropping a value or leaving a hole open.
    for (source, said) in [
        ("format(\"{} {}\", 1)", "have to match"),
        ("format(1, 2)", "pattern in quotes"),
        ("[1, 2].cast(\"nonsense\")", "not a type"),
    ] {
        type_into(&mut store, &block, source).unwrap();
        let refused = store.view().computed_blocks[&block].lines[0]
            .cell
            .error
            .clone()
            .unwrap_or_default();
        assert!(refused.contains(said), "{source}: {refused}");
    }
}

#[test]
fn a_written_list_stays_refused_outside_the_scratchpad() {
    let mut store = blank_store();
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![vec!["Amount".into()], vec!["100".into()], vec!["20".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = block_free_frame(&store, "Orders");

    // Making a list writable does not make it alignable. A column has row
    // identity and a written list has none, so a column formula refuses one
    // exactly as it did before — the scratchpad is where a line is allowed
    // to be a list, and it is the only place.
    let refused = store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "Nope".into(),
            formula: "`Amount` * [1, 2, 3]".into(),
            after_column_id: None,
        })
        .unwrap_err();
    assert!(refused.to_string().contains("position"), "{refused}");
}

#[test]
fn an_aggregate_may_meet_a_list_but_a_column_may_not() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddSeries {
            name: "Rates".into(),
            values: "1\n2\n3".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder.clone()),
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["100".into()],
                vec!["20".into()],
                vec!["5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = block_free_frame(&store, "Orders");
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: frame_id,
            name: "Amounts".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Amount".into(),
                formula: "`Amount`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Rows".into(),
                formula: "`Amount`.count()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let derived_id = block_free_frame(&store, "Amounts");
    let directory = temporary_test_directory("blocks-shapes");
    store
        .materialize_frame(&derived_id, &directory.join("data"))
        .unwrap();
    let block = add_block(&mut store, "Scratch");

    // The aggregate is one number, so it broadcasts over the list — this is
    // the case shape inference had to get right.
    type_into(&mut store, &block, "`Amounts`.`Amount`.sum() * `Rates`").unwrap();
    assert_eq!(
        store.view().computed_blocks[&block].lines[0].cell.display,
        "[125, 250, 375]"
    );

    // The column itself still may not be paired with the list by position.
    type_into(&mut store, &block, "`Amounts`.`Amount` * `Rates`").unwrap();
    let refused = error_on(&store, &block, "line_1");
    assert!(refused.contains("position"), "{refused}");
}

#[test]
fn one_block_reads_another_by_name() {
    let mut store = blank_store();
    let assumptions = add_block(&mut store, "Assumptions");
    let model = add_block(&mut store, "Block 2");

    // A constant is a line like any other line — there is nowhere else on the
    // canvas to put one, so the block has to be somewhere worth putting it.
    type_into(&mut store, &assumptions, "rate = 0.08\nyears = 5").unwrap();
    type_into(
        &mut store,
        &model,
        "principal = 1000\ngrowth = principal * (1 + `Assumptions`.rate) ** `Assumptions`.years",
    )
    .unwrap();

    assert_eq!(answers(&store, &assumptions), ["0.08", "5"]);
    assert_eq!(answers(&store, &model), ["1000", "1469.3281"]);
}

/// A refusal that names the block and not the line has barely narrowed it
/// down: a block is forty lines, and one of them is the one to go and look
/// at.
#[test]
fn a_block_held_in_place_says_which_line_is_holding_it() {
    let mut store = blank_store();
    let assumptions = add_block(&mut store, "Assumptions");
    let model = add_block(&mut store, "Model");
    type_into(&mut store, &assumptions, "rate = 0.08").unwrap();
    type_into(
        &mut store,
        &model,
        "principal = 1000\nnoise = 1\ntax = principal * `Assumptions`.rate",
    )
    .unwrap();

    let refused = store.apply(Operation::DeleteObject {
        object_id: assumptions,
    });
    let Err(CoreError::ReferencedByFormula(message)) = refused else {
        panic!("a block a formula reads cannot be deleted");
    };
    assert!(message.contains("‘tax’ in ‘Model’"), "{message}");
    assert!(message.contains("‘Assumptions’"), "{message}");
}

#[test]
fn a_block_renamed_is_still_the_block_the_other_one_reads() {
    let mut store = blank_store();
    let source = add_block(&mut store, "Block 1");
    let reader = add_block(&mut store, "Block 2");
    type_into(&mut store, &source, "base = 10").unwrap();
    type_into(&mut store, &reader, "doubled = `Block 1`.base * 2").unwrap();
    assert_eq!(answers(&store, &reader), ["20"]);

    // References travel by the line's id, so the qualifier is only how the
    // reference is written down. Renaming the block rewrites the text of
    // every line that named it and changes no answer.
    store
        .apply(Operation::RenameObject {
            object_id: source.clone(),
            name: "Assumptions".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_blocks[&reader].source,
        "doubled = `Assumptions`.`base` * 2"
    );
    assert_eq!(answers(&store, &reader), ["20"]);

    // And the rewritten text is text that parses, which is the whole reason
    // it had to be rewritten: the next keystroke re-reads the block.
    type_into(
        &mut store,
        &reader,
        "doubled = `Assumptions`.`base` * 2\ntripled = `Assumptions`.base * 3",
    )
    .unwrap();
    assert_eq!(answers(&store, &reader), ["20", "30"]);

    // Undo puts the name and the text that named it back together.
    store.undo();
    store.undo();
    assert_eq!(
        store.view().computed_blocks[&reader].source,
        "doubled = `Block 1`.base * 2"
    );
}

#[test]
fn a_line_two_blocks_away_still_cannot_be_typed_away() {
    let mut store = blank_store();
    let source = add_block(&mut store, "Block 1");
    let reader = add_block(&mut store, "Block 2");
    type_into(&mut store, &source, "base = 10\nspare = 1").unwrap();
    type_into(&mut store, &reader, "doubled = `Block 1`.base * 2").unwrap();

    let refused = type_into(&mut store, &source, "spare = 1").unwrap_err();
    assert!(refused.to_string().contains("Block 2"), "{refused}");
    assert_eq!(answers(&store, &source), ["10", "1"]);
}

/// The canvas has one place to put a number, and it is a line of a block.
/// A value, a result, and a list are each a card holding one thing, and a
/// page of scratch arithmetic made of those is the density problem the block
/// exists to answer — so the canvas stopped offering them.
#[test]
fn a_loose_value_has_nowhere_on_the_canvas_to_go() {
    let mut store = blank_store();

    let refusals = [
        store.apply(Operation::AddValue {
            name: "Rate".into(),
            raw: "0.05".into(),
            x: 0.0,
            y: 0.0,
            container_id: None,
        }),
        store.apply(Operation::AddResult {
            name: "Doubled".into(),
            formula: "2 * 2".into(),
            x: 0.0,
            y: 0.0,
            container_id: None,
        }),
        store.apply(Operation::AddSeries {
            name: "Rates".into(),
            values: "1, 2, 3".into(),
            x: 0.0,
            y: 0.0,
            container_id: None,
        }),
    ];
    for refused in refusals {
        let message = refused.unwrap_err().to_string();
        assert!(message.contains("formula block"), "{message}");
    }
    assert!(store.document().objects.is_empty());

    // The line that replaces all three, and it is one card rather than three.
    let block = add_block(&mut store, "Assumptions");
    type_into(
        &mut store,
        &block,
        "rate = 0.05\ndoubled = rate * 2\n[1, 2, 3]",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["0.05", "0.10", "[1, 2, 3]"]);
}

/// A container is the one place left: there a value is part of an
/// arrangement somebody laid out, not a card that drifted loose.
#[test]
fn a_container_is_still_somewhere_a_value_can_live() {
    let mut store = blank_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Rate".into(),
            raw: "0.05".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();

    let block = add_block(&mut store, "Scratchwork");
    type_into(&mut store, &block, "`Holder`.`Rate` * 2").unwrap();
    assert_eq!(answers(&store, &block), ["0.10"]);
}

/// A rate is written the way a rate is written. Making somebody type
/// `0.0425` for `4.25%` is asking them to do the conversion the machine is
/// for, and it loses the fact that the number is a rate at all.
#[test]
fn a_percentage_is_written_with_a_sign_and_reads_back_with_one() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(
        &mut store,
        &block,
        "rate = 4.25%\nfee = 5%\non = 20000 * rate\nboth = rate + fee",
    )
    .unwrap();
    // The value is the fraction and the arithmetic is on the fraction; only
    // the writing carries the sign.
    assert_eq!(answers(&store, &block), ["4.25%", "5%", "850.00", "9.25%"]);

    // A percentage of a percentage is still a percentage; a percentage
    // times a plain number is the plain number's kind, because that is what
    // the multiplication produced.
    type_into(&mut store, &block, "half = 50% * 50%\ncut = 200 * 10%").unwrap();
    assert_eq!(answers(&store, &block), ["25%", "20.00"]);
}

/// The sign binds to a number and nowhere else, so the remainder operator
/// survives — with `%%` for the one spelling that would otherwise be a
/// coin toss.
#[test]
fn the_remainder_still_has_a_spelling() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(
        &mut store,
        &block,
        "10 % 3\n10 %% 3\n10%%3\nn = 10\nn % 3\nn%3",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), ["1", "1", "1", "10", "1", "1"]);

    // `10%3` is the one that changed, and it says so rather than picking.
    type_into(&mut store, &block, "10%3").unwrap();
    let refused = error_on(&store, &block, "line_1");
    assert!(refused.contains("%%"), "{refused}");
}

/// A list of rates is a list of rates: the promotion tree joins a type with
/// itself without dropping to a bare number.
#[test]
fn a_list_of_percentages_stays_percentages() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(&mut store, &block, "[5%, 10%, 12.5%]").unwrap();
    assert_eq!(answers(&store, &block), ["[5%, 10%, 12.5%]"]);

    // Mixed with a plain number they have nothing in common but the number,
    // so that is what the list is — the writing is dropped rather than
    // guessed at.
    type_into(&mut store, &block, "[5%, 1]").unwrap();
    assert_eq!(answers(&store, &block), ["[0.05, 1.00]"]);
}

/// The notation travels through the arithmetic, which no spreadsheet does.
///
/// Money is a dimension and a percentage is a way of writing a pure ratio,
/// and that one sentence settles every case: applying a rate to an amount
/// spends the rate and keeps the amount's kind, and dividing money by money
/// cancels the dimension and leaves the ratio.
#[test]
fn how_a_number_is_written_travels_through_the_arithmetic() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(
        &mut store,
        &block,
        "revenue = $250000\ncost = $155000\nrate = 4.25%\n\
         margin = revenue - cost\nshare = margin / revenue\ntax = revenue * rate\n\
         each = revenue / 4\ndouble = revenue * 2",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        [
            "$250000.00",
            "$155000.00",
            "4.25%",
            // Money less money is money.
            "$95000.00",
            // Money over money cancels to the ratio — the line this exists
            // for, and the one a spreadsheet answers with 0.38.
            "38%",
            // A rate applied to an amount is an amount of the same kind.
            "$10625.00",
            // Sharing money out leaves money; scaling it does too.
            "$62500.00",
            "$500000.00"
        ]
    );
}

/// The two cases the dimensions do not decide, because both sides are
/// dimensionless and only the writing is in question. Both ties are broken
/// toward the reading that cannot come out absurd.
#[test]
fn a_rate_scaled_is_a_number_and_a_rate_shared_out_is_a_rate() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(
        &mut store,
        &block,
        "annual = 4.8%\nmonthly = annual / 12\napplied = 20000 * annual\nscaled = annual * 12",
    )
    .unwrap();
    assert_eq!(
        answers(&store, &block),
        [
            "4.8%", // An annual rate over the months of the year is still a rate.
            "0.4%",
            // The commonest percentage line there is, and the reason the
            // multiplication goes the other way: this must not say 96000%.
            "960.00", // The cost of that choice, and what the override is for.
            "0.576"
        ]
    );
}

/// Overridable, and the override breaks the chain: everything above reads
/// the `.show` rather than working one out from the arithmetic under it.
#[test]
fn how_a_number_is_written_can_be_said_out_loud() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Scratchwork");

    type_into(
        &mut store,
        &block,
        "annual = 4.8%\nscaled = (annual * 12).show(\"percent\")\n\
         half = scaled / 2\nfee = $80\nplain = fee.show(\"plain\")\n\
         count = 12\nas_money = count.show(\"money\")",
    )
    .unwrap();
    // `57.6%` is the override believed over the arithmetic, and `28.8%` is
    // the chain starting again from it. `80` is the same lever pulled the
    // other way, which is the case the client complains about: technically
    // money, and not wanted as money.
    assert_eq!(
        answers(&store, &block),
        ["4.8%", "57.6%", "28.8%", "$80.00", "80.00", "12", "$12.00"]
    );

    // Only a number is written as money or a rate. Saying it of a piece of
    // text is a promise the gutter could not keep, so it is refused rather
    // than quietly ignored.
    type_into(&mut store, &block, "\"ninety\".show(\"money\")").unwrap();
    let refused = error_on(&store, &block, "line_1");
    assert!(refused.contains("Only a number"), "{refused}");

    type_into(&mut store, &block, "n = 12\nn.show(\"euros\")").unwrap();
    let refused = error_on(&store, &block, "line_1");
    assert!(refused.contains("\"money\""), "{refused}");
}

/// Alt+Return's shape: an indented physical line is the same calculation
/// still going, not a next one. Newline only means "new line" at the
/// margin.
#[test]
fn an_indented_line_continues_the_one_above() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Working");
    type_into(&mut store, &block, "revenue = 10\n  + 5\nrevenue * 2").unwrap();

    // Two calculations, not three: the continuation folded into the first.
    assert_eq!(answers(&store, &block), vec!["15", "30"]);

    // And the author's layout comes back byte for byte.
    assert_eq!(
        store.view().computed_blocks[&block].source,
        "revenue = 10\n  + 5\nrevenue * 2"
    );
}

/// Extending a line with a continuation is an edit to that line, not a new
/// line landing under it — the lines below keep reading the same name.
#[test]
fn a_continuation_keeps_the_identity_of_the_line_it_extends() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Working");
    type_into(&mut store, &block, "x = 10\nx * 2").unwrap();
    assert_eq!(answers(&store, &block), vec!["10", "20"]);

    type_into(&mut store, &block, "x = 10\n  + 1\nx * 2").unwrap();
    assert_eq!(answers(&store, &block), vec!["11", "22"]);
}

/// A continuation never joins a blank line: the blank would vanish into the
/// join and the text would stop being what was typed.
#[test]
fn an_indented_line_after_a_blank_stands_alone() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Working");
    type_into(&mut store, &block, "x = 10\n\n  5").unwrap();
    assert_eq!(answers(&store, &block), vec!["10", "!", "5"]);
}

/// Parentheses are both a visible multiline boundary and an ordinary value:
/// closing them at the margin ends no calculation, and a terminal method may
/// consume the grouped value exactly as it would any other expression.
#[test]
fn a_parenthesized_multiline_value_accepts_a_terminal_method() {
    let mut store = blank_store();
    let block = add_block(&mut store, "Working");
    type_into(
        &mut store,
        &block,
        "total = (\n  [10, 20, 30]\n).sum()\ntotal / 2",
    )
    .unwrap();
    assert_eq!(answers(&store, &block), vec!["60", "30.00"]);
}

/// One value in a card named for it is one addressable thing.
///
/// Three cold-agent smoke runs tripped over the same toll from different
/// directions: a "single value" was a block name *and* a line name, and
/// every formula outside had to pay both. Now a single-line block answers
/// to the block's name — bare, and under method calls — and a named line
/// resolves from anywhere its name is unique, with ambiguity refused by
/// name rather than resolved by luck.
#[test]
fn a_named_scalar_is_one_addressable_thing() {
    let mut store = demo_store();
    store
        .apply(Operation::AddBlock {
            name: "Timesheet date".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Timesheet date")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id,
            source: "2026-09-30".into(),
            editing: None,
        })
        .unwrap();

    // The single unnamed line answers to the block's name, bare and under
    // a method chain — the exact formula the smoke agents kept writing.
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Period".into(),
            formula: "sequence(`Timesheet date`.dt.month_start(), `Timesheet date` + 1)".into(),
            column_name: Some("Date".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let period_id = frame_named(store.document(), "Period").id.clone();
    let page = store.get_frame_page(&period_id, 0, 40).unwrap();
    assert_eq!(page.total_rows, 30, "September 1st through the 30th");

    // A named line resolves bare from anywhere while its name is unique...
    store
        .apply(Operation::AddBlock {
            name: "Params".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let params_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Params")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: params_id,
            source: "Anchor = 5\nSpare = 6".into(),
            editing: None,
        })
        .unwrap();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Counts".into(),
            formula: "sequence(0, `Anchor`)".into(),
            column_name: Some("N".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let counts_id = frame_named(store.document(), "Counts").id.clone();
    assert_eq!(
        store.get_frame_page(&counts_id, 0, 10).unwrap().total_rows,
        5
    );

    // ...and stops resolving the moment a second block claims it, with the
    // claimants named.
    store
        .apply(Operation::AddBlock {
            name: "Rival".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let rival_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Rival")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id: rival_id,
            source: "Anchor = 9".into(),
            editing: None,
        })
        .unwrap();
    let failure = store.apply(Operation::AddGeneratorFrame {
        name: "Torn".into(),
        formula: "sequence(0, `Anchor`)".into(),
        column_name: None,
        x: 0.0,
        y: 0.0,
    });
    match failure {
        Err(CoreError::Formula(message)) => {
            assert!(
                message.contains("Params") && message.contains("Rival"),
                "the ambiguity should name both blocks, said: {message}"
            );
        }
        other => panic!("expected an ambiguity refusal, got {other:?}"),
    }
}
