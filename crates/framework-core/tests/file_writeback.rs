use crate::common::*;
use framework_core::*;
use std::{fs, path::Path};

fn open(path: &Path, text: &[u8]) -> (Store, FrameObject) {
    fs::write(path, text).unwrap();
    let mut store = demo_store();
    store
        .apply(Operation::OpenDelimitedFile {
            name: "Editing".into(),
            path: path.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Editing").clone();
    (store, frame)
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
    assert!(frame.owns_its_rows());
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
            row_id: frame.rows[1].id.clone(),
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
    let authored = frame_named(store.document(), "Editing").clone();
    write(&mut store, &frame, &dir.join("recovery"));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "ID\tQty\r\n001\t1.00\r\n\"002\"\t2.00\r\n"
    );
    assert_eq!(
        frame_named(store.document(), "Editing").rows[0].id,
        authored.rows[0].id
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

#[test]
fn only_a_file_copy_can_be_baked_and_only_up_to_the_editable_cap() {
    let dir = temporary_test_directory("file-gates");
    let mut store = demo_store();
    let orders = frame_named(store.document(), "Orders").id.clone();
    let refused = store.apply(Operation::BakeFrame { frame_id: orders });
    assert!(
        refused.is_err(),
        "a hand-entered frame has no file to bake toward"
    );

    let path = dir.join("big.csv");
    let mut text = String::from("ID\n");
    for i in 0..=MAX_EDITABLE_ROWS {
        text.push_str(&format!("{i}\n"));
    }
    fs::write(&path, &text).unwrap();
    let error = store
        .apply(Operation::OpenDelimitedFile {
            name: "Big".into(),
            path: path.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("at most"), "{error}");
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
    assert_eq!(saved.rows[0].cells[&frame.columns[1].id].raw, "10");
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
    store
        .apply(Operation::OpenDelimitedFile {
            name: "Saved result".into(),
            path: path.display().to_string(),
            x: 28.0,
            y: 28.0,
        })
        .unwrap();

    let recipe = frame_named(store.document(), "Editing");
    assert_eq!(recipe.steps.len(), 1);
    assert_eq!(recipe.rows[0].cells[&frame.columns[1].id].raw, "10");
    let result = frame_named(store.document(), "Saved result");
    assert!(result.steps.is_empty());
    assert_eq!(result.rows[0].cells[&result.columns[2].id].raw, "20");
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
    assert!(replaced.owns_its_rows());
    assert_eq!(
        replaced.file_origin.as_ref().unwrap().path,
        fs::canonicalize(&next).unwrap().display().to_string()
    );
    store
        .apply(Operation::SetCell {
            frame_id: frame.id.clone(),
            row_id: replaced.rows[0].id.clone(),
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
    assert_eq!(
        fs::read_to_string(&tsv).unwrap(),
        "Item\tPrice\nDesk\t10.0\n"
    );
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
    // far side of the file, with no inference rule needed to recover it.
    //
    // The counts arrive back as Number rather than Integer because an editable
    // copy types every numeric column as one (`conservative_type`), which the
    // paged path does not — it reads whole numbers as Int64. So the same file
    // exports a double schema when it is small and an integer schema when it
    // is large. Harmless in CSV, where `3.0` is just a spelling; recorded here
    // because a Parquet schema states it as fact.
    assert_eq!(reopened.columns[0].data_type, DataType::String);
    assert_eq!(reopened.columns[1].data_type, DataType::Number);
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
        written, "{\"SKU\":\"0007\",\"Quantity\":2.0}\n{\"SKU\":\"0008\",\"Quantity\":5.0}\n",
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
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let reopened = frame_named(store.document(), "Reopened").clone();
    assert_eq!(
        reopened
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type.clone()))
            .collect::<Vec<_>>(),
        [("SKU", DataType::String), ("Quantity", DataType::Number)]
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
