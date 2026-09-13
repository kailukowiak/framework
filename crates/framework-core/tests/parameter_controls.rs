use super::financial_functions::{close, evaluate};

#[test]
fn frozen_variables_do_not_offer_or_accept_controls() {
    let mut store = workbook("growth = slider(0,1,0.1)");
    let variable = store.view().parameter_inputs[0].id.clone();
    let directory = std::env::temp_dir().join(format!("framework-frozen-control-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    store.freeze_value(&variable, &directory).unwrap();
    assert!(store.view().parameter_inputs.is_empty());
    assert!(
        store
            .apply(Operation::SetParameterValue {
                object_id: variable,
                value: ScalarValue::Number(0.5)
            })
            .is_err()
    );
    std::fs::remove_dir_all(directory).unwrap();
}
use framework_core::*;

fn workbook(source: &str) -> Store {
    let mut store = Store::new(Document::blank("Controls"));
    store
        .apply(Operation::AddBlock {
            name: "Inputs".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::SetBlockSource {
            block_id: store.document().objects[0].id().into(),
            source: source.into(),
            editing: None,
        })
        .unwrap();
    store
}

#[test]
fn constructors_are_scalars_with_literal_configuration() {
    for (source, expected) in [
        ("growth = slider(0, 1, 0.01)\ngrowth + 1", 1.0),
        (
            "growth = slider(start=-1, stop=1, step=0.1, value=0.25)\ngrowth * 100",
            25.0,
        ),
        (
            "choice = dropdown(options=[10,20,30], value=20)\nchoice * 2",
            40.0,
        ),
        ("choice = dropdown([10,20])\nchoice", 10.0),
        (
            "cutoff = date_input(date(2026,12,31))\ncutoff.dt.year()",
            2026.0,
        ),
    ] {
        close(source, expected);
    }
    for source in [
        "slider(1,0,1)",
        "slider(0,1,0)",
        "slider(0,1,-1)",
        "slider(0,1,0.1,2)",
        "slider(0,1,0.1,start=0)",
        "slider(0,1,0.1,foo=2)",
        "slider(0,1)",
        "dropdown([])",
        "dropdown([1,1])",
        "dropdown([1,\"one\"])",
        "dropdown([1,2],3)",
        "dropdown([None])",
        "date_input(\"2026-12-31\")",
        "date_input(date(2026,2,30))",
        "bound = 10\nslider(0,bound,1)",
    ] {
        assert!(evaluate(source).error.is_some(), "accepted {source}");
    }
}

#[test]
fn editing_controls_preserves_named_identity_history_and_persistence() {
    let mut store = workbook(
        "growth = slider(0,1,0.1)\nregion = dropdown([\"North\",\"South\"])\ncutoff = date_input(value=date(2026,12,31))\nanswer = growth * 100",
    );
    let initial = store.view();
    assert_eq!(initial.parameter_inputs.len(), 3);
    let growth = initial.parameter_inputs[0].id.clone();
    let block = initial.computed_blocks.values().next().unwrap();
    assert!(
        block.lines.iter().all(|line| line.cell.error.is_none()),
        "{block:?}"
    );
    store
        .apply(Operation::SetParameterValue {
            object_id: growth.clone(),
            value: ScalarValue::Number(0.25),
        })
        .unwrap();
    let changed = store.view();
    let changed_lines = &changed.computed_blocks.values().next().unwrap().lines;
    assert_eq!(changed_lines[3].cell.value, Some(25.0));
    assert_eq!(changed_lines[0].id, growth);
    assert_eq!(changed_lines[1].text, block.lines[1].text);
    assert!(
        changed_lines[0].text.contains("value=0.25"),
        "{}",
        changed_lines[0].text
    );
    assert_eq!(
        store.undo().computed_blocks.values().next().unwrap().lines[3]
            .cell
            .value,
        Some(0.0)
    );
    assert_eq!(
        store.redo().computed_blocks.values().next().unwrap().lines[3]
            .cell
            .value,
        Some(25.0)
    );
    store
        .apply(Operation::SetParameterValue {
            object_id: initial.parameter_inputs[1].id.clone(),
            value: ScalarValue::String("South".into()),
        })
        .unwrap();
    store
        .apply(Operation::SetParameterValue {
            object_id: initial.parameter_inputs[2].id.clone(),
            value: ScalarValue::Date(chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
        })
        .unwrap();
    let directory = std::env::temp_dir().join(format!("framework-controls-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("controls.fw");
    store.save(&path).unwrap();
    let loaded = Store::load(&path).unwrap().view();
    assert_eq!(
        loaded.parameter_inputs[0].control,
        changed.parameter_inputs[0].control
    );
    assert!(
        matches!(&loaded.parameter_inputs[1].control, ParameterControl::Dropdown { value: ScalarValue::String(v), .. } if v == "South")
    );
    assert!(
        matches!(&loaded.parameter_inputs[2].control, ParameterControl::DateInput { value } if value == "2027-01-01")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn rejected_control_edits_do_not_replace_formulas_or_change_revision() {
    let mut store = workbook("growth = slider(0,1,0.1)\nanswer = growth * 100");
    let view = store.view();
    let lines = &view.computed_blocks.values().next().unwrap().lines;
    for (object_id, value) in [
        (lines[0].id.clone(), ScalarValue::Number(2.0)),
        (lines[0].id.clone(), ScalarValue::String("bad".into())),
        (lines[1].id.clone(), ScalarValue::Number(2.0)),
    ] {
        assert!(
            store
                .apply(Operation::SetParameterValue { object_id, value })
                .is_err()
        );
    }
    assert_eq!(store.document().revision, view.document.revision);
}

#[test]
fn compact_variables_drive_semantic_filters_on_live_derived_frames() {
    let mut store = Store::new(Document::blank("Filter controls"));
    store
        .apply(Operation::AddVariable {
            name: "minimum".into(),
            formula: "slider(0,100,10,value=10)".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let variable = store.document().objects[0].id().to_string();
    store
        .apply(Operation::AddFrame {
            name: "Sales".into(),
            grid: vec![
                vec!["Amount".into()],
                vec!["10".into()],
                vec!["30".into()],
                vec!["50".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = crate::common::frame_named(store.document(), "Sales").clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame.id.clone(),
            name: "Filtered".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let derived = crate::common::frame_named(store.document(), "Filtered")
        .id
        .clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: derived,
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`Amount` >= `minimum`".into()],
                match_all: true,
            }],
        })
        .unwrap();
    store
        .apply(Operation::AddVariable {
            name: "total".into(),
            formula: "`Filtered`.`Amount`.sum()".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let result = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "total")
        .unwrap()
        .id()
        .to_string();
    assert_eq!(
        store.view().computed_results[&result].cell.value,
        Some(90.0)
    );
    store
        .apply(Operation::SetParameterValue {
            object_id: variable.clone(),
            value: ScalarValue::Number(40.0),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_results[&result].cell.value,
        Some(50.0)
    );
    assert_eq!(
        store.undo().computed_results[&result].cell.value,
        Some(90.0)
    );
    store.redo();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: frame.rows[1].id.clone(),
            column_id: frame.columns[0].id.clone(),
            raw: "60".into(),
        })
        .unwrap();
    assert_eq!(
        store.view().computed_results[&result].cell.value,
        Some(110.0)
    );
    store
        .apply(Operation::SetParameterValue {
            object_id: variable,
            value: ScalarValue::Number(100.0),
        })
        .unwrap();
    assert_eq!(store.view().computed_results[&result].cell.value, Some(0.0));
}
