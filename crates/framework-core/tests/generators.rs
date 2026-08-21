//! Generated frames: rows grown from a rule instead of typed in or read
//! from a file. The table-shaped `for each` that Expand multiplies against,
//! without a hand-written offsets CSV to keep up to date.

use crate::common::*;
use framework_core::*;

fn generator_rows(store: &Store, frame_id: &str) -> Vec<String> {
    let view = store.view();
    let frame = view.document.frame(frame_id).unwrap().clone();
    let column_id = frame.columns[0].id.clone();
    frame
        .rows
        .iter()
        .map(|row| row.cells[&column_id].raw.clone())
        .collect()
}

#[test]
fn a_generator_frame_grows_its_rows_from_the_rule() {
    let mut store = demo_store();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Days".into(),
            formula: "sequence(0, 16)".into(),
            column_name: Some("Day offset".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();

    let frame = frame_named(store.document(), "Days").clone();
    assert!(frame.generator.is_some());
    assert_eq!(frame.columns.len(), 1);
    assert_eq!(frame.columns[0].name, "Day offset");
    assert_eq!(frame.columns[0].data_type, DataType::Integer);
    assert!(
        frame.rows.is_empty(),
        "the stored document holds the rule, never its output"
    );

    let rows = generator_rows(&store, &frame.id);
    assert_eq!(rows.len(), 16);
    assert_eq!(rows.first().unwrap(), "0");
    assert_eq!(rows.last().unwrap(), "15");

    // The rows are the rule's output, so they are not a place to type.
    let view = store.view();
    let computed = &view.computed_frames[&frame.id];
    assert!(!computed.editing.cells);
    let failure = store.apply(Operation::SetCell {
        frame_id: frame.id.clone(),
        row_id: format!("derived:{}:0", frame.id),
        column_id: frame.columns[0].id.clone(),
        raw: "99".into(),
    });
    assert!(failure.is_err(), "typing into a generated frame is refused");

    store.undo();
    assert!(
        store
            .document()
            .objects
            .iter()
            .all(|object| object.name() != "Days"),
        "undo removes the generated frame"
    );
}

/// The reason generators exist: bounds that name a value, so editing the
/// value regrows the frame. This is the timesheet case — a period calendar
/// following its anchor date — with no CSV standing in for `0..16`.
#[test]
fn a_generator_with_value_bounds_follows_the_value() {
    let mut store = demo_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Anchor".into(),
            raw: "2026-09-30".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();

    store
        .apply(Operation::AddGeneratorFrame {
            name: "Period days".into(),
            formula: "sequence(`Anchor`.dt.month_start(), `Anchor` + 1)".into(),
            column_name: Some("Date".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Period days").clone();
    assert_eq!(frame.columns[0].data_type, DataType::Date);

    let rows = generator_rows(&store, &frame.id);
    assert_eq!(rows.len(), 30, "September has 30 days");
    assert_eq!(rows.first().unwrap(), "2026-09-01");
    assert_eq!(rows.last().unwrap(), "2026-09-30");

    // Edit the anchor: the frame regrows without anyone touching it.
    let anchor_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Anchor")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetValue {
            object_id: anchor_id,
            raw: "2026-02-10".into(),
        })
        .unwrap();
    let rows = generator_rows(&store, &frame.id);
    assert_eq!(rows.len(), 10, "February 1st through the 10th anchor");
    assert_eq!(rows.first().unwrap(), "2026-02-01");
    assert_eq!(rows.last().unwrap(), "2026-02-10");
}

/// Expand against a generator is the whole point of having one: the cross
/// product that used to need a hand-written list now reads the rule.
#[test]
fn expanding_against_a_generator_multiplies_rows() {
    let mut store = demo_store();
    let source = frame_named(store.document(), "Orders").clone();
    let source_rows = store.view().document.frame(&source.id).unwrap().rows.len();
    assert!(source_rows > 0);

    store
        .apply(Operation::AddGeneratorFrame {
            name: "Offsets".into(),
            formula: "sequence(0, 4)".into(),
            column_name: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let generator_id = frame_named(store.document(), "Offsets").id.clone();

    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: source.id.clone(),
            name: "Order days".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let linked_id = frame_named(store.document(), "Order days").id.clone();
    // Keep the pass-through projection the linked frame was born with, and
    // add the expansion after it.
    let view = store.view();
    let linked = view.document.frame(&linked_id).unwrap().clone();
    let rendered = view.computed_frames[&linked_id].steps.clone();
    let mut steps: Vec<FrameStepInput> = rendered
        .iter()
        .map(|step| existing_step_input(step, &linked))
        .collect();
    steps.push(FrameStepInput::Expand {
        frame_id: generator_id.clone(),
    });
    store
        .apply(Operation::SetFramePipeline {
            frame_id: linked_id.clone(),
            steps,
        })
        .unwrap();

    let expanded_rows = store.view().document.frame(&linked_id).unwrap().rows.len();
    assert_eq!(expanded_rows, source_rows * 4);

    // Deleting the generator out from under the expansion is refused.
    let failure = store.apply(Operation::DeleteObject {
        object_id: generator_id,
    });
    assert!(failure.is_err());
}

/// Rewriting the rule re-types the column: a day-offset generator rewritten
/// as a date range *becomes* dates, and undo brings the offsets back.
#[test]
fn replacing_the_rule_retypes_the_column_and_undoes() {
    let mut store = demo_store();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Range".into(),
            formula: "sequence(0, 4)".into(),
            column_name: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Range").id.clone();
    assert_eq!(generator_rows(&store, &frame_id).len(), 4);

    store
        .apply(Operation::SetFrameGenerator {
            frame_id: frame_id.clone(),
            formula: "sequence(2026-01-01, 2026-01-08)".into(),
        })
        .unwrap();
    let frame = store.view().document.frame(&frame_id).unwrap().clone();
    assert_eq!(frame.columns[0].data_type, DataType::Date);
    assert_eq!(generator_rows(&store, &frame_id).len(), 7);

    store.undo();
    let frame = store.view().document.frame(&frame_id).unwrap().clone();
    assert_eq!(frame.columns[0].data_type, DataType::Integer);
    assert_eq!(generator_rows(&store, &frame_id), ["0", "1", "2", "3"]);

    // A rule that reads a frame column is refused with directions.
    let failure = store.apply(Operation::SetFrameGenerator {
        frame_id,
        formula: "`Orders`.`Total`".into(),
    });
    match failure {
        Err(CoreError::Formula(message)) => {
            assert!(message.contains("scratchpad"), "said: {message}")
        }
        other => panic!("a column-reading rule should be refused, got {other:?}"),
    }
}

/// The steps a rendered chain writes back, for tests that append to it.
fn existing_step_input(step: &RenderedFrameStep, frame: &FrameObject) -> FrameStepInput {
    let name_of = |output_column_id: &str| {
        frame
            .columns
            .iter()
            .find(|column| column.id == output_column_id)
            .map(|column| column.name.clone())
            .unwrap_or_else(|| output_column_id.to_string())
    };
    match step {
        RenderedFrameStep::WithColumns { columns } => FrameStepInput::WithColumns {
            columns: columns
                .iter()
                .map(|column| ExistingFormulaInput {
                    output_column_id: column.output_column_id.clone(),
                    name: name_of(&column.output_column_id),
                    formula: column.formula.clone(),
                })
                .collect(),
        },
        RenderedFrameStep::Select { column_ids } => FrameStepInput::Select {
            column_ids: column_ids.clone(),
        },
        other => panic!("this test only appends to a pass-through chain, found {other:?}"),
    }
}

/// The smoke test's frame-killer, pinned down.
///
/// A generated index decomposed into a date with one fused expression —
/// `month_start().dt.offset_by(Index - (Line - 1) * anchor.dt.day())` —
/// once *saved* fine and then failed on every read: the offset argument's
/// type was unknown to the compiler, so the integer-as-day-count reading
/// never engaged and a bare i64 flowed into a string slot. Polars only
/// notices at execution, which the schema walk never does. Two fixes meet
/// here: `.dt.day()` now declares itself an integer (so the fused argument
/// reads as a day count), and a pipeline save runs one row so anything
/// execution-only left is refused at save rather than detonating later.
#[test]
fn a_fused_offset_expression_saves_and_reads_instead_of_poisoning_the_frame() {
    let mut store = demo_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Anchor".into(),
            raw: "2026-09-30".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Grid".into(),
            formula: "sequence(0, 60)".into(),
            column_name: Some("Index".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Grid").clone();

    let line_id = column_id("Line");
    let date_id = column_id("Date");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: line_id.clone(),
                        name: "Line".into(),
                        formula: "`Index` // 30 + 1".into(),
                    }],
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: date_id.clone(),
                        name: "Date".into(),
                        formula: "`Anchor`.dt.month_start().dt.offset_by(\
                                  `Index` - (`Line` - 1) * `Anchor`.dt.day())"
                            .into(),
                    }],
                },
            ],
        })
        .unwrap();

    // The save proved one row runs; now every row has to. Day 0 of line 1
    // is September 1st, and day 0 of line 2 lands there too.
    let page = store
        .get_frame_page(&frame.id, 0, 61)
        .expect("the saved chain reads");
    assert_eq!(page.total_rows, 60);
    let date_at = |row: usize| {
        let column = frame_named(store.document(), "Grid")
            .columns
            .iter()
            .position(|column| column.id == date_id)
            .unwrap();
        page.rows[row][column].clone()
    };
    assert_eq!(date_at(0), "2026-09-01");
    assert_eq!(date_at(29), "2026-09-30");
    assert_eq!(date_at(30), "2026-09-01", "line 2 restarts the month");

    // The other half of the fix: a chain whose types line up but whose
    // values cannot be produced is refused at save, with the execution
    // error, instead of being written and failing on every later read.
    let failure = store.apply(Operation::SetFramePipeline {
        frame_id: frame.id.clone(),
        steps: vec![FrameStepInput::WithColumns {
            columns: vec![ExistingFormulaInput {
                output_column_id: column_id("Broken"),
                name: "Broken".into(),
                formula: "`Anchor`.dt.offset_by(\"not a duration\")".into(),
            }],
        }],
    });
    assert!(
        failure.is_err(),
        "a chain that cannot execute must be refused at save"
    );
    // And the refusal left the working chain in place.
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 61).unwrap().total_rows,
        60,
        "the refused save must not disturb the saved chain"
    );
}

/// Editing a value a frame reads must move that frame's fingerprint.
///
/// Every cache in and in front of the engine — sorted pages, the summary
/// footer, the interface's row windows — keys on the lineage fingerprint.
/// The serialized chain only names a value by id, so editing the value's
/// contents used to leave the digest still, and a timesheet's Sum footer
/// went on showing the previous period's total after the anchor moved.
#[test]
fn a_value_edit_moves_the_fingerprint_of_frames_that_read_it() {
    let mut store = demo_store();
    let holder = a_container(&mut store);
    store
        .apply(Operation::AddValue {
            name: "Anchor".into(),
            raw: "2026-09-15".into(),
            x: 0.0,
            y: 0.0,
            container_id: Some(holder),
        })
        .unwrap();
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Period".into(),
            formula: "sequence(`Anchor`.dt.month_start(), `Anchor` + 1)".into(),
            column_name: Some("Date".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Period").id.clone();
    let untouched_id = frame_named(store.document(), "Transactions").id.clone();

    let fingerprints = |store: &Store| {
        let view = store.view();
        (
            view.computed_frames[&frame_id].fingerprint.clone(),
            view.computed_frames[&untouched_id].fingerprint.clone(),
        )
    };
    let (before, untouched_before) = fingerprints(&store);
    let anchor_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Anchor")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetValue {
            object_id: anchor_id,
            raw: "2026-09-30".into(),
        })
        .unwrap();
    let (after, untouched_after) = fingerprints(&store);
    assert_ne!(before, after, "the reading frame's fingerprint must move");
    assert_eq!(
        untouched_before, untouched_after,
        "a frame that reads nothing from the value must keep its fingerprint — \
         lineage-scoped is the whole point of the hash"
    );
}
