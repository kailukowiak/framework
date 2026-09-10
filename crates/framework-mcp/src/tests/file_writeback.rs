use super::*;

#[test]
fn editable_file_and_bake_are_available_through_the_generated_operation_surface() {
    let (server, path) = test_server();
    let csv = path.with_extension("csv");
    std::fs::write(&csv, "ID,Amount\n001,12.00\n").unwrap();
    let opened = server.apply_operation(Parameters(ApplyOperationArgs {
        operation: serde_json::json!({ "type": "openDelimitedFile", "name": "Editable", "path": csv, "x": 0, "y": 0 }),
        expected_revision: None,
    })).unwrap().0;
    let snapshot = server
        .get_frame(Parameters(GetFrameArgs {
            frame: "Editable".into(),
            limit: None,
        }))
        .unwrap()
        .0;
    assert_eq!(snapshot.rows[0].cells[0].display, "001");
    let frame_id = snapshot.id;
    server
        .apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({ "type": "bakeFrame", "frameId": frame_id }),
            expected_revision: Some(opened.revision),
        }))
        .unwrap();
    // The generic operation only bakes the workbook. It cannot silently
    // acquire the desktop action's authority to overwrite an external file.
    assert_eq!(
        std::fs::read_to_string(&csv).unwrap(),
        "ID,Amount\n001,12.00\n"
    );
    std::fs::remove_file(csv).unwrap();
}

#[test]
fn batch_rename_uses_the_canonical_operation_and_revision_guard() {
    let (server, path) = test_server();
    let csv = path.with_extension("csv");
    std::fs::write(&csv, "A,B\n1,2\n").unwrap();
    let opened = server.apply_operation(Parameters(ApplyOperationArgs {
        operation: serde_json::json!({"type":"openDelimitedFile","name":"Batch","path":csv,"x":0,"y":0}),
        expected_revision: None,
    })).unwrap().0;
    let snapshot = server
        .get_frame(Parameters(GetFrameArgs {
            frame: "Batch".into(),
            limit: None,
        }))
        .unwrap()
        .0;
    let operation = serde_json::json!({"type":"renameColumns", "frameId":snapshot.id,
        "names":[[snapshot.columns[0].id,"B"],[snapshot.columns[1].id,"A"]]});
    server
        .apply_operation(Parameters(ApplyOperationArgs {
            operation: operation.clone(),
            expected_revision: Some(opened.revision),
        }))
        .unwrap();
    assert!(
        server
            .apply_operation(Parameters(ApplyOperationArgs {
                operation,
                expected_revision: Some(opened.revision),
            }))
            .is_err()
    );
    let renamed = server
        .get_frame(Parameters(GetFrameArgs {
            frame: "Batch".into(),
            limit: None,
        }))
        .unwrap()
        .0;
    assert_eq!(renamed.columns[0].name, "B");
    assert_eq!(std::fs::read_to_string(&csv).unwrap(), "A,B\n1,2\n");
    std::fs::remove_file(csv).unwrap();
}
