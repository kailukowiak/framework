//! Entry columns: hand-entered values on computed frames, stored by key
//! rather than by row position — so regrowing the frame re-attaches them
//! instead of wiping them. The half of "app on FrameWork" that freezing a
//! copy could never give: a generated skeleton and a person's input,
//! coexisting.

use crate::common::*;
use framework_core::*;

/// A generated day list with an Hours entry column: the timesheet shape,
/// reduced to its bones.
fn day_frame_with_hours(store: &mut Store) -> (Id, Id, Id) {
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Days".into(),
            formula: "sequence(0, 5)".into(),
            column_name: Some("Day".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Days").clone();
    let day_column = frame.columns[0].id.clone();
    store
        .apply(Operation::SetUniqueKey {
            frame_id: frame.id.clone(),
            column_ids: vec![day_column.clone()],
            enabled: true,
        })
        .unwrap();
    store
        .apply(Operation::AddEntryColumn {
            frame_id: frame.id.clone(),
            name: "Hours".into(),
            data_type: DataType::Number,
            key_column_ids: vec![day_column.clone()],
        })
        .unwrap();
    let hours_column = frame_named(store.document(), "Days")
        .columns
        .iter()
        .find(|column| column.name == "Hours")
        .unwrap()
        .id
        .clone();
    (frame.id, day_column, hours_column)
}

fn hours_by_day(
    store: &Store,
    frame_id: &str,
    day_column: &str,
    hours_column: &str,
) -> Vec<(String, String)> {
    let view = store.view();
    let frame = view.document.frame(frame_id).unwrap().clone();
    frame
        .rows
        .iter()
        .map(|row| {
            (
                row.cells[day_column].raw.clone(),
                row.cells
                    .get(hours_column)
                    .map(|cell| cell.raw.clone())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn entries_land_on_their_key_and_survive_regrowth() {
    let mut store = demo_store();
    let (frame_id, day_column, hours_column) = day_frame_with_hours(&mut store);

    store
        .apply(Operation::SetEntryValue {
            frame_id: frame_id.clone(),
            column_id: hours_column.clone(),
            key: vec!["2".into()],
            raw: "7.5".into(),
        })
        .unwrap();

    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[2], ("2".to_string(), "7.5".to_string()));
    assert!(
        rows.iter()
            .filter(|(day, _)| day != "2")
            .all(|(_, hours)| hours.is_empty()),
        "no other row caught the entry: {rows:?}"
    );

    // Shrink the generator under the entry: its row is gone, the entry is
    // not. Regrow it: the entry is exactly where it was left.
    store
        .apply(Operation::SetFrameGenerator {
            frame_id: frame_id.clone(),
            formula: "sequence(0, 2)".into(),
        })
        .unwrap();
    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|(_, hours)| hours.is_empty()));

    store
        .apply(Operation::SetFrameGenerator {
            frame_id: frame_id.clone(),
            formula: "sequence(0, 8)".into(),
        })
        .unwrap();
    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows.len(), 8);
    assert_eq!(
        rows[2],
        ("2".to_string(), "7.5".to_string()),
        "the entry re-attached to its key after the frame regrew"
    );
}

#[test]
fn entries_undo_and_blank_removes() {
    let mut store = demo_store();
    let (frame_id, day_column, hours_column) = day_frame_with_hours(&mut store);

    for raw in ["4", "6.25"] {
        store
            .apply(Operation::SetEntryValue {
                frame_id: frame_id.clone(),
                column_id: hours_column.clone(),
                key: vec!["1".into()],
                raw: raw.into(),
            })
            .unwrap();
    }
    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows[1].1, "6.25");

    store.undo();
    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows[1].1, "4", "undo restores the previous entry");

    store
        .apply(Operation::SetEntryValue {
            frame_id: frame_id.clone(),
            column_id: hours_column.clone(),
            key: vec!["1".into()],
            raw: "".into(),
        })
        .unwrap();
    let rows = hours_by_day(&store, &frame_id, &day_column, &hours_column);
    assert_eq!(rows[1].1, "", "blank removes the entry");
    let stored = frame_named(store.document(), "Days")
        .entry_columns
        .iter()
        .find(|entry_column| entry_column.column_id == hours_column)
        .unwrap()
        .entries
        .len();
    assert_eq!(stored, 0, "a blank entry is absent, not empty");
}

#[test]
fn an_entry_column_needs_a_computed_frame_and_a_unique_key() {
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").clone();

    // A frame someone can type into needs no entry column.
    let failure = store.apply(Operation::AddEntryColumn {
        frame_id: orders.id.clone(),
        name: "Note".into(),
        data_type: DataType::String,
        key_column_ids: vec![orders.columns[0].id.clone()],
    });
    assert!(failure.is_err());

    // A computed frame without the key does not send anyone on a second
    // trip: the add mints the unique key itself, and undo gives it back.
    store
        .apply(Operation::AddGeneratorFrame {
            name: "Days".into(),
            formula: "sequence(0, 3)".into(),
            column_name: Some("Day".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Days").clone();
    assert!(frame.unique_keys.is_empty());
    store
        .apply(Operation::AddEntryColumn {
            frame_id: frame.id.clone(),
            name: "Hours".into(),
            data_type: DataType::Number,
            key_column_ids: vec![frame.columns[0].id.clone()],
        })
        .unwrap();
    let keyed = frame_named(store.document(), "Days").clone();
    assert_eq!(
        keyed.unique_keys.len(),
        1,
        "the add enforces its key columns unique itself"
    );
    assert_eq!(
        keyed.unique_keys[0].column_ids,
        vec![frame.columns[0].id.clone()]
    );
    store.undo();
    let unkeyed = frame_named(store.document(), "Days").clone();
    assert!(unkeyed.entry_columns.is_empty(), "undo removes the column");
    assert!(
        unkeyed.unique_keys.is_empty(),
        "undo returns the minted key"
    );

    // The self-minted key still refuses data that is not actually unique:
    // a constant column can never address rows.
    let constant_id = column_id("Constant");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: constant_id.clone(),
                    name: "Constant".into(),
                    formula: "1".into(),
                }],
            }],
        })
        .unwrap();
    let failure = store.apply(Operation::AddEntryColumn {
        frame_id: frame.id.clone(),
        name: "Hours".into(),
        data_type: DataType::Number,
        key_column_ids: vec![constant_id],
    });
    match failure {
        Err(CoreError::InvalidOperation(message)) => {
            assert!(message.contains("duplicate"), "said: {message}")
        }
        other => panic!("expected a duplicates refusal, got {other:?}"),
    }
}

/// The full timesheet shape: entry lines × a generated date range, entered
/// hours keyed by (line, date), and the anchor value moved — the case the
/// freeze-and-retype workflow could never survive.
/// The anchored period expansion the timesheet uses: an Anchor value, a
/// generated date range following it, and a Sheet expanding lines × dates.
fn anchored_sheet(store: &mut Store) -> Id {
    let holder = a_container(store);
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
            name: "Period".into(),
            formula: "sequence(`Anchor`.dt.month_start(), `Anchor` + 1)".into(),
            column_name: Some("Date".into()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let period_id = frame_named(store.document(), "Period").id.clone();

    store
        .apply(Operation::AddFrame {
            name: "Lines".into(),
            grid: vec![
                vec!["Line".into()],
                vec!["Admin".into()],
                vec!["Marketing".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let lines = frame_named(store.document(), "Lines").clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: lines.id.clone(),
            name: "Sheet".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sheet_id = frame_named(store.document(), "Sheet").id.clone();
    let view = store.view();
    let sheet = view.document.frame(&sheet_id).unwrap().clone();
    let rendered = view.computed_frames[&sheet_id].steps.clone();
    let mut steps: Vec<FrameStepInput> = rendered
        .iter()
        .map(|step| pass_through_input(step, &sheet))
        .collect();
    steps.push(FrameStepInput::Expand {
        frame_id: period_id,
    });
    store
        .apply(Operation::SetFramePipeline {
            frame_id: sheet_id.clone(),
            steps,
        })
        .unwrap();
    sheet_id
}

/// A column's id, looked up by the name a person knows it by.
fn column_named(store: &Store, frame_id: &str, name: &str) -> Id {
    store
        .view()
        .document
        .frame(frame_id)
        .unwrap()
        .columns
        .iter()
        .find(|column| column.name == name)
        .unwrap_or_else(|| panic!("no column named {name}"))
        .id
        .clone()
}

#[test]
fn a_timesheet_shaped_expansion_keeps_hours_across_the_period_change() {
    let mut store = demo_store();
    let sheet_id = anchored_sheet(&mut store);
    let line_column = column_named(&store, &sheet_id, "Line");
    let date_column = column_named(&store, &sheet_id, "Date");
    store
        .apply(Operation::SetUniqueKey {
            frame_id: sheet_id.clone(),
            column_ids: vec![line_column.clone(), date_column.clone()],
            enabled: true,
        })
        .unwrap();
    store
        .apply(Operation::AddEntryColumn {
            frame_id: sheet_id.clone(),
            name: "Hours".into(),
            data_type: DataType::Number,
            key_column_ids: vec![line_column.clone(), date_column.clone()],
        })
        .unwrap();
    let hours_column = column_named(&store, &sheet_id, "Hours");

    let first_line = store.view().document.frame(&sheet_id).unwrap().rows[0].cells[&line_column]
        .raw
        .clone();
    store
        .apply(Operation::SetEntryValue {
            frame_id: sheet_id.clone(),
            column_id: hours_column.clone(),
            key: vec![first_line.clone(), "2026-09-15".into()],
            raw: "8".into(),
        })
        .unwrap();

    let entered = |store: &Store| {
        store
            .view()
            .document
            .frame(&sheet_id)
            .unwrap()
            .rows
            .iter()
            .filter(|row| !row.cells[&hours_column].raw.is_empty())
            .map(|row| {
                (
                    row.cells[&line_column].raw.clone(),
                    row.cells[&date_column].raw.clone(),
                    row.cells[&hours_column].raw.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        entered(&store),
        vec![(first_line.clone(), "2026-09-15".into(), "8".into())]
    );

    // Move the anchor to October: September's hours leave the grid but stay
    // stored. Move it back: they are exactly where they were entered.
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
            object_id: anchor_id.clone(),
            raw: "2026-10-31".into(),
        })
        .unwrap();
    assert!(
        entered(&store).is_empty(),
        "October shows no September hours"
    );

    store
        .apply(Operation::SetValue {
            object_id: anchor_id,
            raw: "2026-09-30".into(),
        })
        .unwrap();
    assert_eq!(
        entered(&store),
        vec![(first_line, "2026-09-15".into(), "8".into())],
        "September's hours came back with September"
    );
}

/// The steps a rendered pass-through chain writes back.
fn pass_through_input(step: &RenderedFrameStep, frame: &FrameObject) -> FrameStepInput {
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
