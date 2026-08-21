//! Refreshing a chain in place: a pivot's columns are baked when the step
//! is written, so following a parameter change used to mean re-authoring
//! the step. `RefreshFramePipeline` is the re-write without the authoring.

use crate::common::*;
use framework_core::*;

/// The timesheet's period pivot, end to end: dates generated from an
/// anchor, expanded, pivoted into date columns — then the anchor moves.
/// Lines × an anchored date range, pivoted wide: the timesheet's period
/// pivot, built through the same operations the editor would send.
fn pivoted_sheet(store: &mut Store) -> Id {
    let holder = a_container(store);
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
    let lines_id = frame_named(store.document(), "Lines").id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: lines_id,
            name: "Sheet".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let sheet_id = frame_named(store.document(), "Sheet").id.clone();

    // Expand dates, give the pivot something to hold, pivot dates wide.
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
            steps: steps.clone(),
        })
        .unwrap();
    let sheet = store.view().document.frame(&sheet_id).unwrap().clone();
    let date_column = sheet
        .columns
        .iter()
        .find(|column| column.name == "Date")
        .unwrap()
        .id
        .clone();
    let hours_id = column_id("Hours");
    steps.push(FrameStepInput::WithColumns {
        columns: vec![ExistingFormulaInput {
            output_column_id: hours_id.clone(),
            name: "Hours".into(),
            formula: "null.cast(\"number\")".into(),
        }],
    });
    steps.push(FrameStepInput::Pivot {
        names_column_id: date_column,
        values_column_id: hours_id,
        aggregate: PivotAggregate::First,
    });
    store
        .apply(Operation::SetFramePipeline {
            frame_id: sheet_id.clone(),
            steps,
        })
        .unwrap();
    sheet_id
}

#[test]
fn refreshing_rebakes_pivot_columns_and_keeps_surviving_ids() {
    let mut store = demo_store();
    let sheet_id = pivoted_sheet(&mut store);

    let date_columns = |store: &Store| {
        store
            .view()
            .document
            .frame(&sheet_id)
            .unwrap()
            .columns
            .iter()
            .filter(|column| column.name.starts_with("2026-"))
            .map(|column| (column.name.clone(), column.id.clone()))
            .collect::<Vec<_>>()
    };
    let before = date_columns(&store);
    assert_eq!(before.len(), 15, "September 1st through the 15th");
    assert_eq!(before.first().unwrap().0, "2026-09-01");

    // The anchor moves to month end. The baked pivot deliberately does not
    // follow — that is the schema-stability contract — until the refresh.
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
    assert_eq!(date_columns(&store).len(), 15, "baked columns hold still");

    store
        .apply(Operation::RefreshFramePipeline {
            frame_id: sheet_id.clone(),
        })
        .unwrap();
    let after = date_columns(&store);
    assert_eq!(after.len(), 30, "the refresh re-baked the full month");
    // The fifteen dates that were already columns kept their ids, so
    // formulas written against them survive the refresh.
    for (name, id) in &before {
        let kept = after.iter().find(|(after_name, _)| after_name == name);
        assert_eq!(
            kept.map(|(_, after_id)| after_id),
            Some(id),
            "{name} should keep its column id across the refresh"
        );
    }

    // A frame with no chain has nothing to refresh, and says so.
    let failure = store.apply(Operation::RefreshFramePipeline {
        frame_id: frame_named(store.document(), "Lines").id.clone(),
    });
    assert!(failure.is_err());
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
