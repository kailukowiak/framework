use super::*;

/// A file read through the generic operation surface, and what an agent can
/// and cannot do to it.
///
/// The MCP server has no data directory to stage a parquet into, so the
/// operation it can reach is the one that reads a path directly. What it
/// deliberately never gets is a write-back destination: a `file_origin` is
/// recorded by the desktop, which owns the staging, the confirmation and the
/// file receipt. An agent editing a workbook must not silently acquire the
/// authority to overwrite somebody's CSV.
#[test]
fn a_file_read_through_the_operation_surface_never_gains_authority_over_it() {
    let (server, path) = test_server();
    let csv = path.with_extension("csv");
    std::fs::write(&csv, "ID,Amount\n001,12.00\n").unwrap();
    server
        .apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({
                "type": "importFrameFromFile",
                "name": "Editable",
                "path": csv.display().to_string(),
                "x": 0.0,
                "y": 0.0,
            }),
            expected_revision: None,
        }))
        .unwrap();
    let snapshot = server
        .get_frame(Parameters(GetFrameArgs {
            frame: "Editable".into(),
            limit: None,
        }))
        .unwrap()
        .0;
    assert_eq!(snapshot.rows[0].cells[0].display, "001");
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
    let opened = server
        .apply_operation(Parameters(ApplyOperationArgs {
            operation: serde_json::json!({
                "type": "importFrameFromFile",
                "name": "Batch",
                "path": csv.display().to_string(),
                "x": 0.0,
                "y": 0.0,
            }),
            expected_revision: None,
        }))
        .unwrap()
        .0;
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
