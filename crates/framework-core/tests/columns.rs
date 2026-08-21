use chrono::NaiveDate;
use framework_core::*;

#[test]
fn computed_columns_can_be_inserted_after_a_specific_column() {
    let mut store = Store::new(Document::demo());
    let customers = crate::common::frame_named(store.document(), "Customers");
    let frame_id = customers.id.clone();
    let first_column_id = customers.columns[0].id.clone();
    let first_column_name = customers.columns[0].name.clone();
    let second_column_id = customers.columns[1].id.clone();

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Inserted".into(),
            formula: format!("`{first_column_name}`"),
            after_column_id: Some(first_column_id.clone()),
        })
        .unwrap();

    let columns = &store.document().frame(&frame_id).unwrap().columns;
    assert_eq!(columns[0].id, first_column_id);
    assert_eq!(columns[1].name, "Inserted");
    assert_eq!(columns[2].id, second_column_id);

    let error = store
        .apply(Operation::AddComputedColumn {
            frame_id,
            name: "Nowhere".into(),
            formula: format!("`{first_column_name}`"),
            after_column_id: Some("missing-column".into()),
        })
        .unwrap_err();
    assert!(matches!(error, CoreError::ColumnNotFound));
}

#[test]
fn date_columns_and_nulls_are_typed_and_formula_aware() {
    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Dates and nulls".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::AddFrame {
            name: "Schedule".into(),
            grid: vec![
                vec!["Start".into(), "Fallback".into(), "Active".into()],
                vec!["2024-01-15".into(), "2024-02-01".into(), "true".into()],
                vec!["".into(), "2024-03-20".into(), "false".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = store
        .document()
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .unwrap();
    let start_id = store
        .document()
        .frame(&frame_id)
        .unwrap()
        .columns
        .iter()
        .find(|column| column.name == "Start")
        .unwrap()
        .id
        .clone();

    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Effective".into(),
            formula: "coalesce(`Start`, `Fallback`)".into(),
            after_column_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Next month".into(),
            formula: "`Effective`.dt.offset_by(\"1mo\")".into(),
            after_column_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Missing start".into(),
            formula: "`Start`.is_null()".into(),
            after_column_id: None,
        })
        .unwrap();
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame_id.clone(),
            name: "Effective year".into(),
            formula: "`Effective`.dt.year()".into(),
            after_column_id: None,
        })
        .unwrap();
    let view = store
        .apply(Operation::AddSummary {
            frame_id: frame_id.clone(),
            column_id: start_id.clone(),
            operation: SummaryOperation::Count,
        })
        .unwrap();

    let frame = view.document.frame(&frame_id).unwrap();
    assert_eq!(frame.columns[0].data_type, DataType::Date);
    assert_eq!(frame.columns[2].data_type, DataType::Boolean);
    assert_eq!(frame.columns[3].data_type, DataType::Date);
    assert_eq!(frame.columns[4].data_type, DataType::Date);
    assert_eq!(frame.columns[5].data_type, DataType::Boolean);
    assert_eq!(frame.columns[6].data_type, DataType::Integer);

    let computed = &view.computed_frames[&frame_id];
    let first = &frame.rows[0];
    let second = &frame.rows[1];
    assert_eq!(
        computed.rows[&first.id][&frame.columns[3].id].typed_value,
        ScalarValue::Date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
    );
    assert_eq!(
        computed.rows[&second.id][&start_id].typed_value,
        ScalarValue::Null
    );
    assert!(computed.rows[&second.id][&start_id].error.is_none());
    assert_eq!(
        computed.rows[&second.id][&frame.columns[3].id].typed_value,
        ScalarValue::Date(NaiveDate::from_ymd_opt(2024, 3, 20).unwrap())
    );
    assert_eq!(
        computed.rows[&second.id][&frame.columns[5].id].typed_value,
        ScalarValue::Boolean(true)
    );
    assert_eq!(
        computed.summaries[&frame.summaries[0].id].typed_value,
        ScalarValue::Number(1.0)
    );
}

#[test]
fn operation_api_accepts_frontend_camel_case_fields() {
    let move_view: Operation = serde_json::from_value(serde_json::json!({
        "type": "moveView",
        "viewId": "view-1",
        "x": 120.0,
        "y": 80.0
    }))
    .unwrap();
    assert!(matches!(
        move_view,
        Operation::MoveView { view_id, .. } if view_id == "view-1"
    ));

    let resize_view: Operation = serde_json::from_value(serde_json::json!({
        "type": "resizeView",
        "viewId": "view-1",
        "width": 720.0,
        "height": 480.0
    }))
    .unwrap();
    assert!(matches!(
        resize_view,
        Operation::ResizeView {
            view_id,
            width: 720.0,
            height: 480.0,
        } if view_id == "view-1"
    ));

    let orient: Operation = serde_json::from_value(serde_json::json!({
        "type": "setFrameDisplayOrientation",
        "frameId": "frame-1",
        "orientation": "fieldsAsRows"
    }))
    .unwrap();
    assert!(matches!(
        orient,
        Operation::SetFrameDisplayOrientation {
            frame_id,
            orientation: FrameViewOrientation::FieldsAsRows,
        } if frame_id == "frame-1"
    ));

    let filter: Operation = serde_json::from_value(serde_json::json!({
        "type": "setFrameDisplayFilter",
        "frameId": "frame-1",
        "filters": ["`Amount` > 10"],
        "filterMatchAll": true
    }))
    .unwrap();
    assert!(matches!(
        filter,
        Operation::SetFrameDisplayFilter { frame_id, filters, .. }
            if frame_id == "frame-1" && filters == vec!["`Amount` > 10"]
    ));

    let style: Operation = serde_json::from_value(serde_json::json!({
        "type": "setFrameStyle",
        "frameId": "frame-1",
        "target": { "kind": "cell", "rowId": "row-1", "columnId": "column-1" },
        "style": {
            "bold": true,
            "italic": null,
            "underline": false,
            "textColor": "#112233",
            "fillColor": "#ddeeff",
            "alignment": "right",
            "lineStyle": "double"
        }
    }))
    .unwrap();
    assert!(matches!(
        style,
        Operation::SetFrameStyle {
            target: FrameStyleTarget::Cell { row_id, column_id },
            style: FrameCellStyle { bold: Some(true), alignment: Some(FrameCellAlignment::Right), .. },
            ..
        } if row_id == "row-1" && column_id == "column-1"
    ));

    let set_cell: Operation = serde_json::from_value(serde_json::json!({
        "type": "setCell",
        "frameId": "frame-1",
        "rowId": "row-1",
        "columnId": "column-1",
        "raw": "42"
    }))
    .unwrap();
    assert!(matches!(
        set_cell,
        Operation::SetCell {
            frame_id,
            row_id,
            column_id,
            ..
        } if frame_id == "frame-1" && row_id == "row-1" && column_id == "column-1"
    ));

    let add_formula: Operation = serde_json::from_value(serde_json::json!({
        "type": "addComputedColumn",
        "frameId": "frame-1",
        "name": "Total",
        "formula": "Quantity * Price"
    }))
    .unwrap();
    assert!(matches!(
        add_formula,
        Operation::AddComputedColumn { frame_id, after_column_id, .. }
            if frame_id == "frame-1" && after_column_id.is_none()
    ));

    let insert_formula: Operation = serde_json::from_value(serde_json::json!({
        "type": "addComputedColumn",
        "frameId": "frame-1",
        "name": "Total",
        "formula": "Quantity * Price",
        "afterColumnId": "column-1"
    }))
    .unwrap();
    assert!(matches!(
        insert_formula,
        Operation::AddComputedColumn { frame_id, after_column_id, .. }
            if frame_id == "frame-1" && after_column_id.as_deref() == Some("column-1")
    ));

    let add_column: Operation = serde_json::from_value(serde_json::json!({
        "type": "addColumn",
        "frameId": "frame-1",
        "name": "Notes",
        "dataType": "string",
        "afterColumnId": "column-1"
    }))
    .unwrap();
    assert!(matches!(
        add_column,
        Operation::AddColumn { frame_id, after_column_id, .. }
            if frame_id == "frame-1" && after_column_id.as_deref() == Some("column-1")
    ));
}
