use super::*;

fn apply_model_operation(server: &FrameworkMcp, operation: serde_json::Value) {
    let revision = server.inspect_document().unwrap().0.revision;
    server
        .apply_operation(Parameters(ApplyOperationArgs {
            operation,
            expected_revision: Some(revision),
        }))
        .unwrap();
}

fn snapshot(server: &FrameworkMcp, frame: &str) -> FrameSnapshot {
    server
        .get_frame(Parameters(GetFrameArgs {
            frame: frame.into(),
            limit: None,
        }))
        .unwrap()
        .0
}

fn prepare_linear_model(server: &FrameworkMcp) -> (String, FrameSnapshot) {
    server
        .create_frame(Parameters(CreateFrameArgs {
            name: "Model data".into(),
            grid: vec![
                vec!["x".into(), "y".into()],
                vec!["1".into(), "3".into()],
                vec!["2".into(), "5.1".into()],
                vec!["3".into(), "6.9".into()],
                vec!["4".into(), "9".into()],
                vec!["5".into(), "11.2".into()],
                vec!["6".into(), "12.8".into()],
            ],
            x: None,
            y: None,
            expected_revision: None,
        }))
        .unwrap();
    let source = snapshot(server, "Model data");
    apply_model_operation(
        server,
        serde_json::json!({
            "type":"addModel", "name":"Response model", "x":400, "y":0,
            "spec": {"sourceFrameId":source.id,"targetColumnId":source.columns[1].id,
                "featureColumnIds":[source.columns[0].id],"method":"ols","covariance":"classical",
                "confidenceLevel":0.95,"holdoutFraction":0.0,"seed":42}
        }),
    );
    let model_id = server
        .inspect_document()
        .unwrap()
        .0
        .objects
        .into_iter()
        .find(|object| object.kind == "model" && object.name == "Response model")
        .unwrap()
        .id;
    apply_model_operation(
        server,
        serde_json::json!({"type":"fitModel","modelId":model_id}),
    );
    (model_id, source)
}

#[test]
fn model_tools_fit_predict_update_and_reopen_through_the_public_path() {
    let (server, path) = test_server();
    let (model_id, source) = prepare_linear_model(&server);
    let summary = server
        .get_model(Parameters(GetModelArgs {
            model: model_id.clone(),
        }))
        .unwrap()
        .0;
    assert_eq!(
        summary["summary"]["coefficients"].as_array().unwrap().len(),
        2
    );
    assert!(summary.get("payload").is_none());
    apply_model_operation(
        &server,
        serde_json::json!({
            "type":"addModelPredictions", "modelId":model_id,
            "sourceFrameId":source.id,"featureColumnIds":[source.columns[0].id],
            "name":"Predictions", "x":800,"y":0
        }),
    );
    let before = snapshot(&server, "Predictions");
    let first_before = before.rows[0].cells.last().unwrap().numeric_value.unwrap();
    server
        .set_cell(Parameters(SetCellArgs {
            frame: source.id,
            row: "1".into(),
            column: source.columns[0].id.clone(),
            raw: "10".into(),
            expected_revision: None,
        }))
        .unwrap();
    let after = snapshot(&server, "Predictions");
    let first_after = after.rows[0].cells.last().unwrap().numeric_value.unwrap();
    assert!(first_after > first_before + 10.0);
    let restored = FrameworkMcp::open(path.clone()).unwrap();
    assert_eq!(
        snapshot(&restored, "Predictions").rows[0]
            .cells
            .last()
            .unwrap()
            .numeric_value,
        Some(first_after)
    );
    server
        .undo(Parameters(HistoryArgs {
            expected_revision: None,
        }))
        .unwrap();
    assert_eq!(
        snapshot(&server, "Predictions").rows[0]
            .cells
            .last()
            .unwrap()
            .numeric_value,
        Some(first_before)
    );
    std::fs::remove_file(path).ok();
}
