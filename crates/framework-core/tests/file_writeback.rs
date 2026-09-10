use crate::common::*;
use framework_core::*;
use std::{fs, path::Path};

/// Opening a file the way the desktop does: stage it into a parquet the
/// document reads, and record where it came from so a result can go back.
///
/// The rows are deliberately not in the document. That is the whole of this
/// change: a frame reading a file costs what the corrections cost, and the
/// file it names is what somebody typed into.
fn open_into(store: &mut Store, name: &str, path: &Path) -> FrameObject {
    let data = path
        .parent()
        .expect("test file has a directory")
        .join("data");
    let artifact = create_data_artifact(path, &data).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: name.into(),
            artifact,
            connector: None,
            file_origin: Some(path.display().to_string()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    frame_named(store.document(), name).clone()
}

fn open(path: &Path, text: &[u8]) -> (Store, FrameObject) {
    fs::write(path, text).unwrap();
    let mut store = demo_store();
    let frame = open_into(&mut store, "Editing", path);
    (store, frame)
}

/// The identity of each row on the page, which for a frame reading a file is
/// the ordinal it was read at. There are no stored rows to take ids from.
fn row_ids(store: &Store, frame_id: &str) -> Vec<Id> {
    store.get_frame_page(frame_id, 0, 1000).unwrap().row_ids
}

fn write(store: &mut Store, frame: &FrameObject, recovery: &Path) {
    let prepared = store.prepare_delimited_write(&frame.id, recovery).unwrap();
    prepared.write().unwrap();
}

#[test]
fn editable_csv_preserves_identifiers_dates_quotes_and_noop_bytes() {
    let dir = temporary_test_directory("file-fidelity");
    let path = dir.join("customers.csv");
    let text = b"\xef\xbb\xbfID,Long,Date,Amount,Note\r\n00123,123456789012345678,1/2/2023,12.00,\"a,b\"\r\n007,987654321098765432,2/3/2023,2.50,\"two\nlines\"";
    let (mut store, frame) = open(&path, text);
    assert!(
        !frame.owns_its_rows() && frame.rows.is_empty(),
        "the values are in the file, and the document holds none of them"
    );
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|c| c.data_type)
            .collect::<Vec<_>>(),
        vec![
            DataType::String,
            DataType::String,
            DataType::String,
            DataType::Number,
            DataType::String
        ]
    );
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(fs::read(&path).unwrap(), text);
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(fs::read(&path).unwrap(), text);
}

#[test]
fn renamed_headers_and_manual_edits_write_back_and_undo_can_be_written_again() {
    let dir = temporary_test_directory("file-edit");
    let path = dir.join("customers.csv");
    let text = b"ID,Name\n001,\"Alice\"\n002,Bob\n";
    let (mut store, frame) = open(&path, text);
    store
        .apply(Operation::RenameColumn {
            frame_id: frame.id.clone(),
            column_id: frame.columns[1].id.clone(),
            name: "Customer".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            column_id: frame.columns[1].id.clone(),
            row_id: row_ids(&store, &frame.id)[1].clone(),
            raw: "Bobby, Jr.".into(),
        })
        .unwrap();
    let recovery = dir.join("recovery");
    let prepared = store.prepare_delimited_write(&frame.id, &recovery).unwrap();
    prepared.write().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "ID,Customer\n001,\"Alice\"\n002,\"Bobby, Jr.\"\n"
    );
    assert_eq!(
        fs::read_dir(&recovery)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "original"))
            .count(),
        0,
        "updating the original must not leave a recovery copy"
    );
    store.undo(); // Bobby -> Bob
    write(&mut store, &frame, &recovery);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "ID,Customer\n001,\"Alice\"\n002,Bob\n"
    );
}

#[test]
fn calculated_column_is_baked_once_and_header_filter_is_included() {
    let dir = temporary_test_directory("file-calculated");
    let path = dir.join("sales.csv");
    let (mut store, frame) = open(&path, b"Item,Price\nA,12.00\nB,2.50\n");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "total~test".into(),
                    name: "Total".into(),
                    formula: "`Price` * 2".into(),
                }],
            }],
        })
        .unwrap();
    store
        .apply(Operation::SetFrameDisplayFilter {
            frame_id: frame.id.clone(),
            filters: vec!["`Price` > 10".into()],
            filter_match_all: true,
        })
        .unwrap();
    let authored = frame_named(store.document(), "Editing").clone();
    write(&mut store, &frame, &dir.join("recovery"));
    let retained = frame_named(store.document(), "Editing");
    assert_eq!(retained.steps, authored.steps);
    assert_eq!(retained.rows, authored.rows);
    assert_eq!(retained.columns[0].id, frame.columns[0].id);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "Item,Price,Total\nA,12.00,24\n"
    );
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "Item,Price,Total\nA,12.00,24\n"
    );
}

#[test]
fn sorting_and_filtering_keep_tokens_attached_to_stable_rows() {
    let dir = temporary_test_directory("file-sort");
    let path = dir.join("inventory.tsv");
    let (mut store, frame) = open(
        &path,
        b"ID\tQty\r\n\"002\"\t2.00\r\n001\t1.00\r\n003\t0.00\r\n",
    );
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Filter {
                    predicates: vec!["`Qty` > 0".into()],
                    match_all: true,
                },
                FrameStepInput::Sort {
                    keys: vec![SortInput {
                        column_id: frame.columns[1].id.clone(),
                        descending: false,
                    }],
                },
            ],
        })
        .unwrap();
    let sorted = row_ids(&store, &frame.id);
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "ID\tQty\r\n001\t1.00\r\n\"002\"\t2.00\r\n",
        "each token followed its own row through the filter and the sort"
    );
    assert_eq!(
        row_ids(&store, &frame.id),
        sorted,
        "and a row is still named by where it was read, not by where it landed"
    );
}

#[test]
fn outside_change_even_with_same_row_count_refuses_write_and_keeps_draft() {
    let dir = temporary_test_directory("file-conflict");
    let path = dir.join("inventory.csv");
    let (store, frame) = open(&path, b"ID,Qty\nA,2\nB,1\n");
    let prepared = store
        .prepare_delimited_write(&frame.id, &dir.join("recovery"))
        .unwrap();
    fs::write(&path, "ID,Qty\nB,1\nA,2\n").unwrap();
    assert!(prepared.write().is_err());
    assert!(
        store
            .prepare_delimited_write(&frame.id, &dir.join("recovery"))
            .is_err()
    );
    assert_eq!(frame_named(store.document(), "Editing"), &frame);
    assert_eq!(fs::read_to_string(&path).unwrap(), "ID,Qty\nB,1\nA,2\n");
}

#[test]
fn failed_write_state_keeps_file_and_pipeline_and_rollback_restores_receipt() {
    let dir = temporary_test_directory("file-write-failure");
    let path = dir.join("sales.csv");
    let (mut store, frame) = open(&path, b"Amount\n12.00\n");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: frame.columns[0].id.clone(),
                    name: "Amount".into(),
                    formula: "`Amount` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let recovery = dir.join("not-directory");
    fs::write(&recovery, "occupied").unwrap();
    assert!(store.prepare_delimited_write(&frame.id, &recovery).is_err());
    assert!(!frame_named(store.document(), "Editing").steps.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), "Amount\n12.00\n");
    let recovery = dir.join("recovery");
    let prepared = store.prepare_delimited_write(&frame.id, &recovery).unwrap();
    prepared.write().unwrap();
    prepared.rollback().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "Amount\n12.00\n");
    write(&mut store, &frame, &recovery);
    assert_eq!(fs::read_to_string(&path).unwrap(), "Amount\n24\n");
}

/// A file far past the old twenty-thousand-row ceiling opens, is typed into,
/// and is written back — while the document holds not one of its cells.
///
/// The ceiling existed because an opened file used to be copied into the
/// document as literal rows, so every autosave serialized the whole table and
/// every operation rebuilt a view over it. Reading it where it lies removes
/// the reason for the limit rather than raising it.
#[test]
fn a_file_past_the_old_editable_ceiling_opens_and_is_still_editable() {
    let dir = temporary_test_directory("file-large");
    let path = dir.join("big.csv");
    let mut text = String::from("ID,Note\n");
    for i in 0..25_000 {
        text.push_str(&format!("{i},row {i}\n"));
    }
    fs::write(&path, &text).unwrap();
    let (mut store, frame) = open(&path, text.as_bytes());
    assert!(
        frame.rows.is_empty(),
        "the file's rows stay in the file, not in the document"
    );
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 5).unwrap().total_rows,
        25_000
    );

    let last = store.get_frame_page(&frame.id, 24_999, 1).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: last.row_ids[0].clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "corrected".into(),
        })
        .unwrap();
    let settled = frame_named(store.document(), "Editing");
    assert_eq!(
        settled.cell_overlay.len(),
        1,
        "one correction is one note, whatever the size of the table"
    );
    assert!(settled.rows.is_empty());

    write(&mut store, &frame, &dir.join("recovery"));
    let written = fs::read_to_string(&path).unwrap();
    assert!(
        written.ends_with("24999,corrected\n"),
        "the last line moved"
    );
    assert!(
        written.starts_with("ID,Note\n0,row 0\n1,row 1\n"),
        "and nothing else did"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_source_is_refused_before_anything_is_written() {
    let dir = temporary_test_directory("file-symlink");
    let real = dir.join("real.csv");
    let (store, frame) = open(&real, b"ID\n001\n");
    let link = dir.join("link.csv");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    fs::rename(&real, dir.join("moved.csv")).unwrap();
    fs::rename(&link, &real).unwrap();
    let error = store
        .prepare_delimited_write(&frame.id, &dir.join("recovery"))
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(error.contains("symbolic link"), "{error}");
}

#[test]
fn update_keeps_active_recipe_and_replacement_reuses_it_once() {
    let dir = temporary_test_directory("read-recipe");
    let path = dir.join("sales.csv");
    let (mut store, frame) = open(&path, b"ID,Price\n0002,10\n");
    store
        .apply(Operation::RenameColumns {
            frame_id: frame.id.clone(),
            names: vec![(frame.columns[0].id.clone(), "Code".into())],
        })
        .unwrap();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: frame.columns[1].id.clone(),
                    name: "Price".into(),
                    formula: "`Price` * 2".into(),
                }],
            }],
        })
        .unwrap();
    write(&mut store, &frame, &dir.join("recovery"));
    let saved = frame_named(store.document(), "Editing");
    assert_eq!(saved.steps.len(), 1);
    assert!(saved.disconnected_read.is_none());
    let saved = saved.clone();
    // Ten in the file, doubled by the chain, is twenty on the page — which is
    // also the check that writing the result back did not fold the doubling
    // into the base. If it had, this would read forty by now.
    assert_eq!(
        store.get_frame_page(&saved.id, 0, 10).unwrap().rows[0][1],
        "20"
    );
    // Serialization is the notebook boundary, independent of the CSV.
    let document: Document =
        serde_json::from_str(&serde_json::to_string(store.document()).unwrap()).unwrap();
    assert_eq!(frame_named(&document, "Editing").steps, saved.steps);
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Code,Price\n0002,20\n");
    let next = dir.join("next.csv");
    fs::write(&next, "ID,Price\n0003,30\n").unwrap();
    let artifact = create_data_artifact(&next, &dir.join("data")).unwrap();
    store
        .apply(Operation::SetFrameSource {
            frame_id: frame.id.clone(),
            artifact,
            connector: ConnectorRecipe::Database {
                connection_id: "test".into(),
                source_name: "Sales".into(),
                query: "select * from sales".into(),
            },
        })
        .unwrap();
    let reconnected = frame_named(store.document(), "Editing");
    assert!(reconnected.disconnected_read.is_none());
    assert_eq!(reconnected.steps.len(), 1);
    assert_eq!(reconnected.columns[0].id, frame.columns[0].id);
    assert_eq!(reconnected.columns[0].name, "Code");
    let output = dir.join("output.csv");
    store.export_frame_file(&frame.id, &output).unwrap();
    assert_eq!(fs::read_to_string(output).unwrap(), "Code,Price\n0003,60\n");
    store.undo();
    let restored = frame_named(store.document(), "Editing");
    assert!(restored.disconnected_read.is_none());
    assert_eq!(restored.steps.len(), 1);
}

#[test]
fn updated_file_returns_as_a_clean_frame_without_changing_the_recipe() {
    let dir = temporary_test_directory("update-split");
    let path = dir.join("sales.csv");
    let (mut store, frame) = open(&path, b"Item,Price\nDesk,10\n");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "total~test".into(),
                    name: "Total".into(),
                    formula: "`Price` * 2".into(),
                }],
            }],
        })
        .unwrap();

    write(&mut store, &frame, &dir.join("write-state"));
    let result = open_into(&mut store, "Saved result", &path);

    let recipe = frame_named(store.document(), "Editing").clone();
    assert_eq!(recipe.steps.len(), 1);
    assert_eq!(
        store.get_frame_page(&recipe.id, 0, 10).unwrap().rows[0][1],
        "10"
    );
    assert!(result.steps.is_empty());
    assert_eq!(
        store.get_frame_page(&result.id, 0, 10).unwrap().rows[0][2],
        "20"
    );
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "Item,Price,Total\nDesk,10,20\n"
    );
}

#[test]
fn batch_rename_swaps_names_atomically_and_rejects_duplicates() {
    let dir = temporary_test_directory("batch-rename");
    let (mut store, frame) = open(&dir.join("names.csv"), b"A,B\n1,2\n");
    store
        .apply(Operation::RenameColumns {
            frame_id: frame.id.clone(),
            names: vec![
                (frame.columns[0].id.clone(), "B".into()),
                (frame.columns[1].id.clone(), "A".into()),
            ],
        })
        .unwrap();
    assert_eq!(
        frame_named(store.document(), "Editing").columns[0].name,
        "B"
    );
    assert!(
        store
            .apply(Operation::RenameColumns {
                frame_id: frame.id.clone(),
                names: vec![(frame.columns[0].id.clone(), "A".into()),]
            })
            .is_err()
    );
    store.undo();
    assert_eq!(
        frame_named(store.document(), "Editing").columns,
        frame.columns
    );
}

#[test]
fn paged_csv_inference_preserves_late_identifiers_and_ambiguous_text() {
    let dir = temporary_test_directory("safe-paged-read");
    let path = dir.join("source.csv");
    let mut text = "ID,Number,Date,Missing\n".to_string();
    for _ in 0..150 {
        text.push_str("2,12.5,2026-01-02,NA\n");
    }
    text.push_str("0002,3.5,2026-01-03,NA\n");
    fs::write(&path, text).unwrap();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    let mut store = demo_store();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Safe".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Safe");
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|c| c.data_type)
            .collect::<Vec<_>>(),
        vec![
            DataType::String,
            DataType::Number,
            DataType::String,
            DataType::String
        ]
    );
}

#[test]
fn replacing_an_editable_csv_keeps_the_new_writeback_destination() {
    let dir = temporary_test_directory("read-new-destination");
    let (mut store, frame) = open(&dir.join("first.csv"), b"ID,Amount\n0001,1\n");
    write(&mut store, &frame, &dir.join("recovery"));
    let next = dir.join("next.csv");
    fs::write(&next, "ID,Amount\n0002,2\n").unwrap();
    let artifact = create_data_artifact(&next, &dir.join("data")).unwrap();
    store
        .apply(Operation::SetFrameSource {
            frame_id: frame.id.clone(),
            artifact,
            connector: ConnectorRecipe::File {
                source_path: next.display().to_string(),
            },
        })
        .unwrap();
    let replaced = frame_named(store.document(), "Editing").clone();
    assert_eq!(
        replaced.file_origin.as_ref().unwrap().path,
        fs::canonicalize(&next).unwrap().display().to_string(),
        "the destination follows the frame to the file it now reads"
    );
    assert!(
        replaced.connector.is_none(),
        "and a frame somebody can write back to is not one a refresh may overwrite"
    );
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: row_ids(&store, &frame.id)[0].clone(),
            column_id: replaced.columns[1].id.clone(),
            raw: "3".into(),
        })
        .unwrap();
    write(&mut store, &replaced, &dir.join("recovery"));
    assert_eq!(fs::read_to_string(next).unwrap(), "ID,Amount\n0002,3\n");
}

#[test]
fn ordinary_export_cannot_overwrite_its_input_and_exporting_a_copy_keeps_the_recipe() {
    let dir = temporary_test_directory("read-export-copy");
    let path = dir.join("input.csv");
    let (store, frame) = open(&path, b"ID,Amount\n0001,1\n");
    assert!(store.export_frame_file(&frame.id, &path).is_err());
    store
        .export_frame_file(&frame.id, &dir.join("copy.csv"))
        .unwrap();
    assert!(
        frame_named(store.document(), "Editing")
            .disconnected_read
            .is_none()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "ID,Amount\n0001,1\n");
}

#[test]
fn conservative_paged_types_do_not_collapse_duplicate_headers() {
    let dir = temporary_test_directory("read-duplicate-headers");
    let path = dir.join("duplicates.csv");
    fs::write(&path, "Code,Code\n0002,3\n").unwrap();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    let mut store = demo_store();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Duplicates".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Duplicates");
    assert_eq!(frame.columns.len(), 2);
    assert_eq!(frame.columns[0].data_type, DataType::String);
    assert_eq!(frame.columns[1].data_type, DataType::Integer);
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().rows,
        vec![vec!["0002", "3"]]
    );
}

#[test]
fn export_writes_the_format_its_name_promises() {
    let dir = temporary_test_directory("export-formats");
    let (store, frame) = open(&dir.join("sales.csv"), b"Item,Price\nDesk,10\n");
    let tsv = dir.join("sales.tsv");
    store.export_frame_file(&frame.id, &tsv).unwrap();
    assert_eq!(fs::read_to_string(&tsv).unwrap(), "Item\tPrice\nDesk\t10\n");
    assert!(
        store
            .export_frame_file(&frame.id, &dir.join("sales.xlsx"))
            .is_err(),
        "a frame export writes only the formats the chooser offers"
    );
}

#[test]
fn exporting_parquet_carries_types_a_csv_would_have_to_respell() {
    let dir = temporary_test_directory("export-parquet");
    let (mut store, frame) = open(&dir.join("stock.csv"), b"SKU,Count\n0002,3\n0010,4\n");
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "doubled~export".into(),
                    name: "Doubled".into(),
                    formula: "`Count` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let parquet = dir.join("stock.parquet");
    store.export_frame_file(&frame.id, &parquet).unwrap();

    // Read the file back through the ordinary import rather than trusting the
    // writer's own account of what it wrote.
    let artifact = create_data_artifact(&parquet, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Round trip".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let reopened = frame_named(store.document(), "Round trip").clone();
    assert_eq!(
        reopened
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["SKU", "Count", "Doubled"],
        "display names travel as the file's own header"
    );
    // The point of a typed destination: the identifier is still text on the
    // far side of the file, with no inference rule needed to recover it — and
    // a whole number is an integer on both sides now that a file has one read
    // path rather than a small one and a large one. The same file used to
    // export a double schema when it fitted in the document and an integer
    // schema when it did not.
    assert_eq!(reopened.columns[0].data_type, DataType::String);
    assert_eq!(reopened.columns[1].data_type, DataType::Integer);
    let page = store.get_frame_page(&reopened.id, 0, 50).unwrap();
    assert_eq!(page.rows[0], vec!["0002", "3", "6"]);
    assert_eq!(page.rows[1], vec!["0010", "4", "8"]);
}

#[test]
fn ndjson_exports_one_object_per_line_and_opens_again() {
    let dir = temporary_test_directory("export-ndjson");
    let (mut store, frame) = open(&dir.join("orders.csv"), b"SKU,Quantity\n0007,2\n0008,5\n");
    let ndjson = dir.join("orders.ndjson");
    store.export_frame_file(&frame.id, &ndjson).unwrap();
    let written = fs::read_to_string(&ndjson).unwrap();
    assert_eq!(
        written, "{\"SKU\":\"0007\",\"Quantity\":2}\n{\"SKU\":\"0008\",\"Quantity\":5}\n",
        "one self-describing object per line, with the identifier still quoted"
    );

    // The same file comes back in, which is what makes it a handoff rather
    // than a dead end: JSON states each value's type, so the identifier needs
    // none of the delimited path's inference rules to stay text.
    let artifact = create_data_artifact(&ndjson, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Reopened".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let reopened = frame_named(store.document(), "Reopened").clone();
    assert_eq!(
        reopened
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        [("SKU", DataType::String), ("Quantity", DataType::Integer)]
    );
    let page = store.get_frame_page(&reopened.id, 0, 50).unwrap();
    assert_eq!(page.rows[0], vec!["0007", "2"]);
    assert_eq!(page.rows[1], vec!["0008", "5"]);
}

#[test]
fn json_lines_typing_waits_for_the_whole_file() {
    let dir = temporary_test_directory("ndjson-late-types");
    let path = dir.join("events.ndjson");
    // A field that is null for a long run and a string only at the end is the
    // shape that defeats sampling. Write more rows than any sane inference
    // sample would read before deciding.
    let mut text = String::new();
    for index in 0..1_200 {
        text.push_str(&format!("{{\"id\":{index},\"note\":null}}\n"));
    }
    text.push_str("{\"id\":1200,\"note\":\"late arrival\"}\n");
    fs::write(&path, &text).unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Events".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Events").clone();
    assert_eq!(frame.columns[1].name, "note");
    assert_eq!(
        frame.columns[1].data_type,
        DataType::String,
        "a value seen only on the last line still decides the column's type"
    );
    let page = store.get_frame_page(&frame.id, 1_200, 1).unwrap();
    assert_eq!(page.rows[0][1], "late arrival");
}

#[test]
fn nested_json_records_are_refused_with_the_field_named() {
    let dir = temporary_test_directory("ndjson-nested");
    let path = dir.join("nested.ndjson");
    fs::write(
        &path,
        b"{\"id\":1,\"customer\":{\"name\":\"Ada\",\"city\":\"Calgary\"}}\n",
    )
    .unwrap();
    // Polars infers the struct happily and the frame would import; the grid
    // cannot render such a cell, so the table used to land looking fine and
    // fail on its first page read. Refuse before anything reaches the canvas.
    let refused = create_data_artifact(&path, &dir.join("data")).unwrap_err();
    let message = refused.to_string();
    assert!(message.contains("customer"), "{message}");
    assert!(message.contains("flatten"), "{message}");
}

/// A filter or a sort between you and the file no longer stops you typing.
///
/// This is the assertion the whole index move exists for: the page reports the
/// ordinal of the row a value was *read* from, so a write finds the row the
/// chain moved rather than the position it moved it to. Get this wrong and the
/// edit silently lands on a different row, which is why it is checked against
/// the file's own order with the chain taken back off.
#[test]
fn a_chained_parquet_frame_edits_the_row_the_value_came_from() {
    let dir = temporary_test_directory("adopted-chain-edit");
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\nC,3\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    let count = frame.columns[1].id.clone();

    // Drop the first row and reverse the rest, so every visible position
    // disagrees with the file's.
    let chain = vec![
        FrameStepInput::Filter {
            predicates: vec!["`Count` > 1".into()],
            match_all: true,
        },
        FrameStepInput::Sort {
            keys: vec![framework_core::SortInput {
                column_id: count.clone(),
                descending: true,
            }],
        },
    ];
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: chain,
        })
        .unwrap();
    assert!(
        store.view().computed_frames[&frame.id].editing.cells,
        "an identity-preserving chain over an owned parquet stays editable"
    );

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(page.rows[0][0], "C", "C sorts first on a descending Count");
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: count.clone(),
            raw: "99".into(),
        })
        .unwrap();

    // Take the chain off: the file's own order is the only witness that the
    // right row was written.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![],
        })
        .unwrap();
    let bare = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        bare.rows
            .iter()
            .map(|row| (row[0].as_str(), row[1].as_str()))
            .collect::<Vec<_>>(),
        [("A", "1"), ("B", "2"), ("C", "99")],
        "the third row of the file changed, and only it"
    );
}

/// A calculated column on such a frame is still not typeable: the chain made
/// it, so the chain is where it changes.
#[test]
fn a_calculated_column_over_a_parquet_frame_refuses_a_typed_value() {
    let dir = temporary_test_directory("adopted-chain-calc");
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::WithColumns {
                columns: vec![ExistingFormulaInput {
                    output_column_id: "doubled~chain".into(),
                    name: "Doubled".into(),
                    formula: "`Count` * 2".into(),
                }],
            }],
        })
        .unwrap();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    let doubled = frame_named(store.document(), "Stock")
        .columns
        .iter()
        .find(|column| column.name == "Doubled")
        .expect("the calculated column is declared")
        .id
        .clone();
    let refused = store.apply(Operation::SetCell {
        frame_id: frame.id.clone(),
        row_id: page.row_ids[0].clone(),
        column_id: doubled,
        raw: "7".into(),
    });
    assert!(refused.is_err(), "a calculation is edited as a formula");
    // The stored field beside it is still typeable, so the refusal is about
    // the column rather than the frame.
    assert!(
        store.view().computed_frames[&frame.id].editing.cells,
        "the frame itself still accepts typed values"
    );
}

/// The point of the whole exercise: correcting a cell costs the correction.
///
/// The write this replaced read the entire parquet in, spliced one value, and
/// wrote a new content-addressed file, so three corrections to a large table
/// meant three full rewrites and two orphans. Here the file is not touched at
/// all — byte for byte the one the import wrote — and the document grows by one
/// entry.
#[test]
fn typing_over_a_parquet_cell_leaves_the_file_untouched() {
    let dir = temporary_test_directory("overlay-efficiency");
    let path = dir.join("ledger.csv");
    let mut csv = String::from("Ref,Amount\n");
    for index in 0..5_000 {
        csv.push_str(&format!("R{index:05},{index}\n"));
    }
    fs::write(&path, &csv).unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    let amount = frame.columns[1].id.clone();
    let before = frame
        .artifact
        .clone()
        .expect("an imported frame has a file");
    let bytes_before = fs::read(&before.path).unwrap();

    let page = store.get_frame_page(&frame.id, 0, 3).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[1].clone(),
            column_id: amount.clone(),
            raw: "777".into(),
        })
        .unwrap();

    let after = frame_named(store.document(), "Ledger");
    let artifact_after = after.artifact.as_ref().expect("still file-backed");
    assert_eq!(
        artifact_after.id, before.id,
        "the data file is content-addressed and its content did not change"
    );
    assert_eq!(
        fs::read(&artifact_after.path).unwrap(),
        bytes_before,
        "byte for byte the file the import wrote"
    );
    assert_eq!(
        after.cell_overlay.len(),
        1,
        "one column carries a patch, holding only the rows that were touched"
    );
    assert_eq!(after.cell_overlay[0].entries.len(), 1);

    let edited = store.get_frame_page(&frame.id, 0, 3).unwrap();
    assert_eq!(edited.rows[1][1], "777");
    assert_eq!(edited.rows[0][1], "0", "and only that cell");
    assert_eq!(edited.rows[2][1], "2");

    // The patch is also what undo removes, rather than a second full rewrite.
    store.undo();
    let undone = frame_named(store.document(), "Ledger");
    assert!(
        undone.cell_overlay.is_empty(),
        "undoing the only correction takes the patch list with it"
    );
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 3).unwrap().rows[1][1],
        "1"
    );
}

/// Replacing the base takes the patches with it.
///
/// They are keyed by ordinal in the base they were entered against, so the
/// alternative is applying somebody's correction to whatever row now sits at
/// that position — a wrong value that looks exactly like a right one.
#[test]
fn replacing_the_source_drops_patches_rather_than_moving_them() {
    let dir = temporary_test_directory("overlay-source-swap");
    let first = dir.join("first.csv");
    fs::write(&first, b"Ref,Amount\nA,1\nB,2\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&first, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "900".into(),
        })
        .unwrap();
    assert_eq!(
        frame_named(store.document(), "Ledger").cell_overlay.len(),
        1
    );

    let second = dir.join("second.csv");
    fs::write(&second, b"Ref,Amount\nA,10\nB,20\n").unwrap();
    let replacement = create_data_artifact(&second, &dir.join("data")).unwrap();
    store
        .apply(Operation::SetFrameSource {
            frame_id: frame.id.clone(),
            artifact: replacement,
            connector: ConnectorRecipe::File {
                source_path: second.display().to_string(),
            },
        })
        .unwrap();

    assert!(
        frame_named(store.document(), "Ledger")
            .cell_overlay
            .is_empty(),
        "a patch against the old file does not follow the frame to a new one"
    );
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[1].as_str())
            .collect::<Vec<_>>(),
        ["10", "20"],
        "the replacement reads as itself"
    );
}

/// Rows struck out and rows added, against a file nobody rewrote.
///
/// Ordinals continue past the base's end, so an added row is one whose every
/// value is a patch — one identity space, one patch mechanism, and the file
/// stays exactly as the import wrote it.
#[test]
fn rows_can_be_struck_out_and_added_without_touching_the_file() {
    let dir = temporary_test_directory("overlay-row-patches");
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\nC,3\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &dir.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    let (sku, count) = (frame.columns[0].id.clone(), frame.columns[1].id.clone());
    let bytes_before = fs::read(frame.artifact.as_ref().unwrap().path.clone()).unwrap();

    // Strike out the middle row.
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    store
        .apply(Operation::DeleteRow {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[1].clone(),
        })
        .unwrap();
    assert_eq!(
        store
            .get_frame_page(&frame.id, 0, 10)
            .unwrap()
            .rows
            .iter()
            .map(|row| row[0].clone())
            .collect::<Vec<_>>(),
        ["A", "C"],
        "B is not part of the result any more"
    );

    // Add one past the end and fill it in.
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: Default::default(),
        })
        .unwrap();
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(page.rows.len(), 3, "A, C, and the new one");
    let added = page.row_ids[2].clone();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: added.clone(),
            column_id: sku.clone(),
            raw: "D".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: added,
            column_id: count.clone(),
            raw: "4".into(),
        })
        .unwrap();

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| (row[0].as_str(), row[1].as_str()))
            .collect::<Vec<_>>(),
        [("A", "1"), ("C", "3"), ("D", "4")]
    );
    assert_eq!(
        fs::read(
            frame_named(store.document(), "Stock")
                .artifact
                .as_ref()
                .unwrap()
                .path
                .clone()
        )
        .unwrap(),
        bytes_before,
        "and the file is still byte for byte the one the import wrote"
    );

    // Undo reaches back through both kinds of patch.
    store.undo();
    store.undo();
    store.undo();
    assert_eq!(
        store
            .get_frame_page(&frame.id, 0, 10)
            .unwrap()
            .rows
            .iter()
            .map(|row| row[0].clone())
            .collect::<Vec<_>>(),
        ["A", "C"],
        "the added row is gone again"
    );
}

/// A frame reading a parquet, with one of each kind of patch on it and a
/// chain above them.
///
/// Shared by the two tests below because building it is most of what either
/// one costs, and both need exactly the same starting point: a correction
/// typed over a cell, a row struck out, a row added past the end, and a
/// filter and a calculated column reading the result.
fn patched_stock(dir: &Path) -> (Store, FrameObject, Vec<Vec<String>>) {
    let data = dir.join("data");
    let path = dir.join("stock.csv");
    fs::write(&path, b"SKU,Count\nA,1\nB,2\nC,3\nD,0\n").unwrap();
    let mut store = demo_store();
    let artifact = create_data_artifact(&path, &data).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Stock".into(),
            artifact,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Stock").clone();
    let (sku, count) = (frame.columns[0].id.clone(), frame.columns[1].id.clone());

    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: count.clone(),
            raw: "9".into(),
        })
        .unwrap();
    store
        .apply(Operation::DeleteRow {
            frame_id: frame.id.clone(),
            row_id: page.row_ids[1].clone(),
        })
        .unwrap();
    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: Default::default(),
        })
        .unwrap();
    // A, C, D and the new blank one: B was struck out, and the added row is
    // last because its ordinal continues past the end of the base.
    let page = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(page.rows.len(), 4);
    let added = page.row_ids[3].clone();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: added.clone(),
            column_id: sku,
            raw: "E".into(),
        })
        .unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: added,
            column_id: count,
            raw: "5".into(),
        })
        .unwrap();

    // The chain goes on last only so the added row is fillable while it is
    // still visible: a row added past the end arrives empty, and this filter
    // would hide it until it had a number in it.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![
                FrameStepInput::Filter {
                    predicates: vec!["`Count` > 0".into()],
                    match_all: true,
                },
                FrameStepInput::WithColumns {
                    columns: vec![ExistingFormulaInput {
                        output_column_id: "doubled~test".into(),
                        name: "Doubled".into(),
                        formula: "`Count` * 2".into(),
                    }],
                },
            ],
        })
        .unwrap();
    let before = store.get_frame_page(&frame.id, 0, 10).unwrap();
    assert_eq!(
        before
            .rows
            .iter()
            .map(|row| (row[0].as_str(), row[1].as_str(), row[2].as_str()))
            .collect::<Vec<_>>(),
        [("A", "9", "18"), ("C", "3", "6"), ("E", "5", "10")]
    );
    assert!(frame_named(store.document(), "Stock").has_row_patches());
    (store, frame, before.rows)
}

/// Folding settles the corrections and changes nothing anybody can see.
///
/// The patches are what the fold is for, and they are all it takes: the
/// wrangle chain still runs over the new file exactly as it ran over the old,
/// which is why the calculated column still recomputes rather than arriving
/// baked, and why undo can still put the notes back.
#[test]
fn folding_patches_settles_them_and_leaves_the_view_alone() {
    let dir = temporary_test_directory("overlay-fold");
    let (mut store, frame, before) = patched_stock(&dir);
    let previous = frame_named(store.document(), "Stock")
        .artifact
        .as_ref()
        .unwrap()
        .id
        .clone();

    let folded = store
        .write_folded_frame_data(&frame.id, &dir.join("data"))
        .unwrap();
    assert_ne!(folded.id, previous, "the fold is a different file");
    store
        .apply(Operation::FoldRowPatches {
            folded: vec![(frame.id.clone(), folded)],
        })
        .unwrap();

    let settled = frame_named(store.document(), "Stock");
    assert!(
        !settled.has_row_patches(),
        "the corrections are the file now, not notes beside it"
    );
    assert_eq!(settled.steps.len(), 2, "the chain is not part of the fold");
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().rows,
        before,
        "and nothing anybody was looking at moved"
    );

    // A second fold has nothing to settle, from either direction.
    assert!(
        store
            .write_folded_frame_data(&frame.id, &dir.join("data"))
            .is_err(),
        "a frame with no corrections has nothing to write"
    );
    let settled = frame_named(store.document(), "Stock")
        .artifact
        .clone()
        .unwrap();
    assert!(
        store
            .apply(Operation::FoldRowPatches {
                folded: vec![(frame.id.clone(), settled)],
            })
            .is_err(),
        "and the operation refuses it too, so a replay cannot repoint a frame \
         at a file computed from a state it is no longer in"
    );

    store.undo();
    let restored = frame_named(store.document(), "Stock");
    assert!(restored.has_row_patches());
    assert_eq!(restored.artifact.as_ref().unwrap().id, previous);
    assert_eq!(store.get_frame_page(&frame.id, 0, 10).unwrap().rows, before);
}

/// What the folded file itself holds, read with nothing on top of it.
///
/// The corrections are in it, and so is the row the filter was only hiding:
/// a step is a plan over the base, and folding one in would delete rows
/// nobody asked to lose.
#[test]
fn a_folded_file_holds_the_corrections_and_the_rows_a_filter_was_hiding() {
    let dir = temporary_test_directory("overlay-fold-file");
    let (mut store, frame, _) = patched_stock(&dir);
    let folded = store
        .write_folded_frame_data(&frame.id, &dir.join("data"))
        .unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Folded".into(),
            artifact: folded,
            connector: None,
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let written = frame_named(store.document(), "Folded").id.clone();
    assert_eq!(
        store.get_frame_page(&written, 0, 10).unwrap().rows,
        [["A", "9"], ["C", "3"], ["D", "0"], ["E", "5"],],
        "the typed-over value and the added row are in the file; the struck-out \
         row is gone and the filtered-out one is not"
    );
}

/// Settling corrections and then putting the result back still writes the
/// right file.
///
/// The fold moves rows: a struck-out row leaves the base, so every ordinal
/// after it shifts and the source line an ordinal names is no longer the line
/// that row was read from. That costs spelling, never correctness — a token is
/// reused only when it still spells the value the plan holds, so a misaligned
/// line simply fails that test and the cell is encoded fresh. The values are
/// the plan's either way. This is the crossing of the two halves of this work,
/// and the place a silently wrong CSV could otherwise hide.
#[test]
fn a_folded_table_still_writes_the_right_file_back() {
    let dir = temporary_test_directory("fold-then-write");
    let path = dir.join("stock.csv");
    let (mut store, frame) = open(&path, b"SKU,Count\nA,1.00\nB,2.00\nC,3.00\n");
    let page = row_ids(&store, &frame.id);
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page[2].clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "9".into(),
        })
        .unwrap();
    store
        .apply(Operation::DeleteRow {
            frame_id: frame.id.clone(),
            row_id: page[0].clone(),
        })
        .unwrap();
    let before = store.get_frame_page(&frame.id, 0, 10).unwrap().rows;

    let folded = store
        .write_folded_frame_data(&frame.id, &dir.join("data"))
        .unwrap();
    store
        .apply(Operation::FoldRowPatches {
            folded: vec![(frame.id.clone(), folded)],
        })
        .unwrap();
    assert_eq!(store.get_frame_page(&frame.id, 0, 10).unwrap().rows, before);

    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "SKU,Count\nB,2\nC,9\n",
        "every value is the one on screen; the trailing zeros are gone because \
         after a fold no row is looking at the line it was read from any more"
    );
}

/// A correction moves the frame's lineage fingerprint.
///
/// The fingerprint is what tells a cached page it is out of date, so a
/// correction that does not move it is a correction nobody sees: the value
/// is in the document, every read is answered from a page fetched before it,
/// and the grid goes on showing the file. That is exactly what happened —
/// the digest listed the fields that decide a frame's rows and the patches
/// were not among them.
#[test]
fn a_correction_moves_the_fingerprint_that_invalidates_a_cached_page() {
    let dir = temporary_test_directory("overlay-fingerprint");
    let path = dir.join("stock.csv");
    let (mut store, frame) = open(&path, b"SKU,Count\nA,1\nB,2\n");
    let fingerprint = |store: &Store| store.view().computed_frames[&frame.id].fingerprint.clone();

    let opened = fingerprint(&store);
    let page = row_ids(&store, &frame.id);
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: page[0].clone(),
            column_id: frame.columns[1].id.clone(),
            raw: "9".into(),
        })
        .unwrap();
    let typed = fingerprint(&store);
    assert_ne!(typed, opened, "a typed-over value is a different table");

    store
        .apply(Operation::DeleteRow {
            frame_id: frame.id.clone(),
            row_id: page[1].clone(),
        })
        .unwrap();
    let struck = fingerprint(&store);
    assert_ne!(struck, typed, "so is a struck-out row");

    store
        .apply(Operation::AddRow {
            frame_id: frame.id.clone(),
            values: Default::default(),
        })
        .unwrap();
    assert_ne!(fingerprint(&store), struck, "and so is an added one");

    store.undo();
    store.undo();
    store.undo();
    assert_eq!(fingerprint(&store), opened, "and undo puts the table back");
}

/// Adding and removing rows is offered wherever it actually works.
///
/// The grid reads this flag to decide whether to show the new-row line and
/// the delete gesture, and `prepare` decides whether to accept them. They
/// were separate answers, and the file-backed frame is where they disagreed:
/// a row added past the end of a base read has been supported since patches
/// landed, while the view went on reporting that this table could not grow.
#[test]
fn a_file_backed_table_offers_the_row_gestures_it_accepts() {
    let dir = temporary_test_directory("overlay-row-affordance");
    let (mut store, frame) = open(&dir.join("stock.csv"), b"SKU,Count\nA,1\n");
    let editing = store.view().computed_frames[&frame.id].editing.clone();
    assert!(editing.cells);
    assert!(editing.rows, "a file-backed table can grow by a patch");
    assert!(
        store
            .apply(Operation::AddRow {
                frame_id: frame.id.clone(),
                values: Default::default(),
            })
            .is_ok(),
        "and the operation agrees"
    );

    // A reshaping chain leaves no row to name, and then neither answers yes.
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame.id.clone(),
            steps: vec![FrameStepInput::Summarize {
                group_keys: vec![ExistingFormulaInput {
                    output_column_id: frame.columns[0].id.clone(),
                    name: "SKU".into(),
                    formula: "`SKU`".into(),
                }],
                aggregates: vec![ExistingFormulaInput {
                    output_column_id: "total~affordance".into(),
                    name: "Total".into(),
                    formula: "`Count`.sum()".into(),
                }],
                maintain_order: true,
            }],
        })
        .unwrap();
    assert!(!store.view().computed_frames[&frame.id].editing.rows);
    assert!(
        store
            .apply(Operation::AddRow {
                frame_id: frame.id.clone(),
                values: Default::default(),
            })
            .is_err()
    );
}
