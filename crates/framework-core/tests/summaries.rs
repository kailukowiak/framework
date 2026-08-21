use framework_core::*;

fn profile_fixture() -> (Store, FrameObject) {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Profile".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Values".into(),
            grid: vec![
                vec!["Name".into(), "Value".into()],
                vec!["A".into(), "1".into()],
                vec!["B".into(), "2".into()],
                vec!["A".into(), "3".into()],
                vec!["".into(), "4".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.clone()),
            _ => None,
        })
        .unwrap();
    (store, frame)
}

#[test]
fn a_profile_is_one_statistic_per_row_and_one_compatible_value_per_column() {
    let (mut store, frame) = profile_fixture();
    let operations = vec![
        SummaryOperation::Count,
        SummaryOperation::Missing,
        SummaryOperation::CountDistinct,
        SummaryOperation::Quartile25,
        SummaryOperation::Mean,
        SummaryOperation::Median,
        SummaryOperation::Quartile75,
        SummaryOperation::Sum,
        SummaryOperation::Mode,
    ];
    store
        .apply(Operation::SetFrameSummaryRows {
            frame_id: frame.id.clone(),
            summary_rows: operations.clone(),
        })
        .unwrap();
    let profile = store.get_frame_summary(&frame.id).unwrap();
    assert_eq!(
        profile
            .rows
            .iter()
            .map(|row| row.operation)
            .collect::<Vec<_>>(),
        operations
    );

    let name = &frame.columns[0].id;
    let value = &frame.columns[1].id;
    let row = |operation| {
        profile
            .rows
            .iter()
            .find(|row| row.operation == operation)
            .unwrap()
    };
    assert_eq!(row(SummaryOperation::Count).cells[name].value, Some(3.0));
    assert_eq!(row(SummaryOperation::Count).cells[value].value, Some(4.0));
    assert_eq!(row(SummaryOperation::Missing).cells[name].value, Some(1.0));
    assert_eq!(
        row(SummaryOperation::CountDistinct).cells[name].value,
        Some(2.0)
    );
    assert_eq!(
        row(SummaryOperation::Quartile25).cells[value].value,
        Some(1.75)
    );
    assert_eq!(row(SummaryOperation::Mean).cells[value].value, Some(2.5));
    assert_eq!(row(SummaryOperation::Median).cells[value].value, Some(2.5));
    assert_eq!(
        row(SummaryOperation::Quartile75).cells[value].value,
        Some(3.25)
    );
    assert_eq!(row(SummaryOperation::Sum).cells[value].value, Some(10.0));
    assert_eq!(
        row(SummaryOperation::Mode).cells[name].typed_value,
        ScalarValue::String("A".into())
    );
    assert!(!row(SummaryOperation::Mode).cells.contains_key(value));
    assert!(!row(SummaryOperation::Mean).cells.contains_key(name));
}

#[test]
fn a_profile_describes_the_filtered_rows_the_frame_is_showing() {
    let (mut store, frame) = profile_fixture();
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Value` > 2".into()],
            filter_match_all: true,
        })
        .unwrap();
    store
        .apply(Operation::SetFrameSummaryRows {
            frame_id: frame.id.clone(),
            summary_rows: vec![SummaryOperation::Count, SummaryOperation::Mean],
        })
        .unwrap();
    let profile = store.get_frame_summary(&frame.id).unwrap();
    let value = &frame.columns[1].id;
    assert_eq!(profile.rows[0].cells[value].value, Some(2.0));
    assert_eq!(profile.rows[1].cells[value].value, Some(3.5));
}

#[test]
fn profile_configuration_is_deduplicated_and_undoable() {
    let (mut store, frame) = profile_fixture();
    store
        .apply(Operation::SetFrameSummaryRows {
            frame_id: frame.id.clone(),
            summary_rows: vec![
                SummaryOperation::Mean,
                SummaryOperation::Mean,
                SummaryOperation::Mode,
            ],
        })
        .unwrap();
    assert_eq!(
        store
            .document()
            .frame(&frame.id)
            .unwrap()
            .display
            .summary_rows,
        Some(vec![SummaryOperation::Mean, SummaryOperation::Mode])
    );
    store.undo();
    assert!(
        store
            .document()
            .frame(&frame.id)
            .unwrap()
            .display
            .summary_rows
            .is_none()
    );
}

#[test]
fn clearing_a_profile_stays_empty_even_when_an_old_column_summary_exists() {
    let (mut store, frame) = profile_fixture();
    store
        .apply(Operation::AddSummary {
            frame_id: frame.id.clone(),
            column_id: frame.columns[1].id.clone(),
            operation: SummaryOperation::Sum,
        })
        .unwrap();
    assert_eq!(store.get_frame_summary(&frame.id).unwrap().rows.len(), 1);

    store
        .apply(Operation::SetFrameSummaryRows {
            frame_id: frame.id.clone(),
            summary_rows: Vec::new(),
        })
        .unwrap();
    assert!(store.get_frame_summary(&frame.id).unwrap().rows.is_empty());

    store.undo();
    assert_eq!(store.get_frame_summary(&frame.id).unwrap().rows.len(), 1);
}

#[test]
fn the_profile_drawer_opens_resizes_and_undoes_independently_of_its_rows() {
    let (mut store, frame) = profile_fixture();
    store
        .apply(Operation::SetFrameSummaryDrawer {
            frame_id: frame.id.clone(),
            open: true,
            height: Some(240.0),
        })
        .unwrap();
    let display = &store.document().frame(&frame.id).unwrap().display;
    assert!(display.summary_drawer_open);
    assert_eq!(display.summary_drawer_height, Some(240.0));
    assert!(display.summary_rows.is_none());

    store.undo();
    let display = &store.document().frame(&frame.id).unwrap().display;
    assert!(!display.summary_drawer_open);
    assert_eq!(display.summary_drawer_height, None);
}

#[test]
fn a_quartile_profile_cell_has_an_equivalent_formula() {
    let (mut store, frame) = profile_fixture();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame.id.clone(),
            name: "First quartile".into(),
            formula: "`Value`.quantile(0.25)".into(),
            after_column_id: None,
        })
        .unwrap();
    let view = store.view();
    let updated = view.document.frame(&frame.id).unwrap();
    let quartile = updated
        .columns
        .iter()
        .find(|column| column.name == "First quartile")
        .unwrap();
    for row in &updated.rows {
        assert_eq!(
            view.computed_frames[&frame.id].rows[&row.id][&quartile.id].value,
            Some(1.75)
        );
    }
}
