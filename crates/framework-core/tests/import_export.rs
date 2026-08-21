use crate::common::*;
use chrono::NaiveDate;
use framework_core::*;
use polars::prelude as pl;
use polars::prelude::NamedFrom;
use std::fs;
use std::path::Path;

#[test]
fn csv_imports_round_trip_through_typed_frames_and_export() {
    let directory = temporary_test_directory("csv-import");
    let source = directory.join("orders.csv");
    fs::write(
        &source,
        "Item,Amount,Sold on,Active\nWidget,3,2026-01-15,true\nGadget,4.5,2026-02-01,false\n",
    )
    .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Import".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    let view = store
        .apply(Operation::ImportFrameFromFile {
            name: "Orders".into(),
            path: source.display().to_string(),
            x: 80.0,
            y: 80.0,
        })
        .unwrap();
    assert!(view.can_undo);

    let frame = frame_named(store.document(), "Orders").clone();
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        vec![
            ("Item", DataType::String),
            ("Amount", DataType::Number),
            ("Sold on", DataType::Date),
            ("Active", DataType::Boolean),
        ]
    );
    let page = store.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0], vec!["Widget", "3", "2026-01-15", "true"]);

    let exported = directory.join("orders-export.csv");
    store.export_frame_csv(&frame.id, &exported).unwrap();
    assert_eq!(
        fs::read_to_string(&exported).unwrap(),
        "Item,Amount,Sold on,Active\nWidget,3.0,2026-01-15,true\nGadget,4.5,2026-02-01,false\n"
    );

    assert!(store.undo().document.objects.is_empty());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn excel_export_writes_selected_tables_and_named_answers() {
    let directory = temporary_test_directory("excel-export");
    let path = directory.join("handoff.xlsx");
    let mut store = Store::new(Document::blank("Handoff"));
    store
        .apply(Operation::AddFrame {
            name: "Orders".into(),
            grid: vec![
                vec!["Item".into(), "Amount".into()],
                vec!["Widget & gear".into(), "3.5".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddFrame {
            name: "Not selected".into(),
            grid: vec![vec!["Ignore".into()], vec!["me".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store
        .apply(Operation::AddBlock {
            name: "Checks".into(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let block_id = store
        .document()
        .objects
        .iter()
        .find(|object| object.name() == "Checks")
        .unwrap()
        .id()
        .to_string();
    store
        .apply(Operation::SetBlockSource {
            block_id,
            source: "tax_rate = 0.125\ntax_multiplier = 1 + tax_rate".into(),
            editing: None,
        })
        .unwrap();

    let orders_id = frame_named(store.document(), "Orders").id.clone();
    store.export_excel(&[orders_id], &path).unwrap();

    let workbook = inspect_excel_workbook(&path).unwrap();
    assert_eq!(
        workbook
            .sheets
            .iter()
            .map(|sheet| sheet.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Values", "Orders"]
    );
    let values = preview_excel_range(&path, "Values", "A1:B3", true, 10).unwrap();
    assert_eq!(values.columns, vec!["Name", "Value"]);
    assert_eq!(
        values.rows,
        vec![
            vec!["Checks.tax_rate", "0.125"],
            vec!["Checks.tax_multiplier", "1.125"],
        ]
    );
    let orders = preview_excel_range(&path, "Orders", "A1:B2", true, 10).unwrap();
    assert_eq!(orders.columns, vec!["Item", "Amount"]);
    assert_eq!(orders.rows, vec![vec!["Widget & gear", "3.5"]]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn excel_export_qualifies_names_and_disambiguates_duplicate_paths() {
    let directory = temporary_test_directory("excel-export-names");
    let path = directory.join("names.xlsx");
    let mut store = Store::new(Document::blank("Names"));
    for container_name in ["Finance", "Operations"] {
        store
            .apply(Operation::AddContainer {
                name: container_name.into(),
                x: 0.0,
                y: 0.0,
                container_id: None,
            })
            .unwrap();
        let container_id = store
            .document()
            .objects
            .iter()
            .find(|object| object.name() == container_name)
            .unwrap()
            .id()
            .to_string();
        store
            .apply(Operation::AddValue {
                name: "Rate".into(),
                raw: "0.1".into(),
                x: 0.0,
                y: 0.0,
                container_id: Some(container_id),
            })
            .unwrap();
    }
    for value in ["1", "2"] {
        store
            .apply(Operation::AddBlock {
                name: "Checks".into(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let block_id = store
            .document()
            .objects
            .iter()
            .rev()
            .find(|object| object.name() == "Checks")
            .unwrap()
            .id()
            .to_string();
        store
            .apply(Operation::SetBlockSource {
                block_id,
                source: format!("x = {value}"),
                editing: None,
            })
            .unwrap();
    }

    store.export_excel(&[], &path).unwrap();
    let values = preview_excel_range(&path, "Values", "A1:B5", true, 10).unwrap();
    let names = values
        .rows
        .iter()
        .map(|row| row[0].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names[0..2], ["Finance.Rate", "Operations.Rate"]);
    assert!(names[2].starts_with("Checks.x ["));
    assert!(names[3].starts_with("Checks.x ["));
    assert_ne!(names[2], names[3]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn relative_file_imports_remain_readable_after_the_document_reopens() {
    // MCP processes inherit their client's working directory. Persisting the
    // relative spelling accepted by one process made the same document fail
    // as soon as a verifier, the desktop app, or another client reopened it
    // from somewhere else. The stored source is the file we resolved during
    // import, independent of whichever process reads the document next.
    let current = std::env::current_dir().unwrap();
    let directory = current.join(format!(
        "framework-relative-import-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("orders.csv");
    fs::write(&source, "Item,Amount\nWidget,3\nGadget,4\n").unwrap();
    let relative_source = source.strip_prefix(&current).unwrap();
    let document_path = directory.join("analysis.fw");

    let mut store = Store::new(Document::blank("Analysis"));
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Orders".into(),
            path: relative_source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let recorded = frame_named(store.document(), "Orders")
        .source_file
        .as_deref()
        .unwrap();
    assert!(Path::new(recorded).is_absolute());
    store.save(&document_path).unwrap();

    let reopened = Store::load(&document_path).unwrap();
    let frame = frame_named(reopened.document(), "Orders");
    let page = reopened.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[1], vec!["Gadget", "4"]);

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn tsv_imports_split_on_tabs() {
    let directory = temporary_test_directory("tsv-import");
    let source = directory.join("regions.tsv");
    fs::write(&source, "Region\tTarget\nNorth\t120\nSouth\t90\n").unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Import".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Regions".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Regions");
    assert_eq!(frame.columns.len(), 2);
    assert_eq!(frame.columns[1].data_type, DataType::Integer);
    let page = store.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[1][0], "South");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn parquet_imports_keep_schema_types_instead_of_string_inference() {
    let directory = temporary_test_directory("parquet-import");
    let source = directory.join("inventory.parquet");
    let mut frame = pl::DataFrame::new(
        2,
        vec![
            pl::Series::new("Code".into(), &["0011", "0012"]).into(),
            pl::Series::new("Count".into(), &[3i64, 9]).into(),
            pl::Series::new(
                "Seen".into(),
                &[
                    NaiveDate::from_ymd_opt(2026, 3, 1)
                        .unwrap()
                        .and_hms_opt(9, 30, 0)
                        .unwrap(),
                    NaiveDate::from_ymd_opt(2026, 3, 2)
                        .unwrap()
                        .and_hms_opt(18, 0, 0)
                        .unwrap(),
                ],
            )
            .into(),
        ],
    )
    .unwrap();
    pl::ParquetWriter::new(fs::File::create(&source).unwrap())
        .finish(&mut frame)
        .unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Import".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Inventory".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Inventory");
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        vec![
            ("Code", DataType::String),
            ("Count", DataType::Integer),
            ("Seen", DataType::Date),
        ]
    );
    let page = store.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(page.rows[0][0], "0011");
    assert_eq!(page.rows[0][1], "3");
    assert_eq!(page.rows[0][2], "2026-03-01");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn file_imports_support_large_files() {
    let directory = temporary_test_directory("import-large-file");
    let source = directory.join("big.csv");
    let mut contents = String::with_capacity(1000 * 3 + 10);
    contents.push_str("Value\n");
    contents.push_str(&"1\n".repeat(1000));
    fs::write(&source, contents).unwrap();

    let mut store = Store::new(Document {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Import".into(),
        revision: 0,
        objects: Vec::new(),
        views: Vec::new(),
        frozen_values: Default::default(),
    });
    store
        .apply(Operation::ImportFrameFromFile {
            name: "Big".into(),
            path: source.display().to_string(),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Big");
    let page = store.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(page.total_rows, 1000);

    let unsupported = directory.join("big.xlsx");
    fs::write(&unsupported, "not a frame").unwrap();
    assert!(matches!(
        store.prepare_operation(Operation::ImportFrameFromFile {
            name: "Sheet".into(),
            path: unsupported.display().to_string(),
            x: 0.0,
            y: 0.0,
        }),
        Err(CoreError::Import(message)) if message.contains(".parquet")
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn artifacts_are_content_addressed_paged_and_copied_on_save_as() {
    let directory = temporary_test_directory("artifact-import");
    let source = directory.join("orders.csv");
    fs::write(
        &source,
        "name,amount\nAlpha,10\nBeta,20\nGamma,30\nDelta,40\n",
    )
    .unwrap();
    let first_document = directory.join("first/analysis.fw");
    let document = Document::blank("Analysis");
    let artifact_directory = CollaborationPaths::for_document(&first_document, &document.id)
        .unwrap()
        .root
        .join("data");
    let artifact = create_data_artifact(&source, &artifact_directory).unwrap();
    assert_eq!(artifact.row_count, 4);
    assert_eq!(artifact.id.len(), 64);
    assert!(artifact.path.ends_with(&format!("{}.parquet", artifact.id)));

    let mut store = Store::new(document);
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact,
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(&store.view().document, "Orders").clone();
    assert!(store.view().computed_frames[&frame.id].paged);
    let page = store.get_frame_page(&frame.id, 2, 2).unwrap();
    assert_eq!(page.total_rows, 4);
    assert_eq!(page.rows, vec![vec!["Gamma", "30"], vec!["Delta", "40"]]);

    store.save(&first_document).unwrap();
    let second_document = directory.join("second/copied.fw");
    store.save_as(&second_document).unwrap();
    let copied_artifact = frame_named(&store.view().document, "Orders")
        .artifact
        .as_ref()
        .unwrap()
        .path
        .clone();
    assert!(Path::new(&copied_artifact).starts_with(directory.join("second/.framework")));
    fs::remove_dir_all(directory.join("first")).unwrap();
    let reopened = Store::load(&second_document).unwrap();
    assert_eq!(
        frame_named(reopened.document(), "Orders").connector,
        Some(ConnectorRecipe::File {
            source_path: source.display().to_string(),
        })
    );
    assert_eq!(
        reopened.get_frame_page(&frame.id, 0, 1).unwrap().rows[0],
        vec!["Alpha", "10"]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_copied_document_reads_its_own_artifacts() {
    let directory = temporary_test_directory("copy-document");
    let source = directory.join("orders.csv");
    fs::write(&source, "name,amount\nAlpha,10\nBeta,20\n").unwrap();

    let original = directory.join("original/ledger.fw");
    fs::create_dir_all(original.parent().unwrap()).unwrap();
    let mut store = Store::new(Document::blank("Ledger"));
    let artifact = create_data_artifact(
        &source,
        &CollaborationPaths::for_document(&original, store.document_id())
            .unwrap()
            .root
            .join("data"),
    )
    .unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    store.save(&original).unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();

    let copy = directory.join("copy/ledger.fw");
    fs::create_dir_all(copy.parent().unwrap()).unwrap();
    fs::copy(&original, &copy).unwrap();
    copy_directory_recursive(
        &directory.join("original/.framework"),
        &directory.join("copy/.framework"),
    );

    fs::remove_dir_all(directory.join("original")).unwrap();

    let reopened = Store::load(&copy).unwrap();
    let artifact_path = frame_named(reopened.document(), "Orders")
        .artifact
        .as_ref()
        .unwrap()
        .path
        .clone();
    assert!(
        Path::new(&artifact_path).starts_with(directory.join("copy")),
        "the copy must read the artifact beside it, not the original's: {artifact_path}"
    );
    assert_eq!(
        reopened.get_frame_page(&frame_id, 0, 2).unwrap().rows,
        vec![vec!["Alpha", "10"], vec!["Beta", "20"]]
    );

    fs::remove_dir_all(directory).unwrap();
}

fn copy_directory_recursive(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory_recursive(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn artifact_refresh_preserves_identity_and_reconciles_schema_changes() {
    let directory = temporary_test_directory("artifact-refresh");
    let source = directory.join("orders.csv");
    fs::write(&source, "name,amount\nAlpha,10\nBeta,20\n").unwrap();
    let document = Document::blank("Refresh");
    let artifact_directory = directory.join("artifacts");
    let initial = create_data_artifact(&source, &artifact_directory).unwrap();
    let mut store = Store::new(document);
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: initial.clone(),
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();

    fs::write(&source, "name,amount\nGamma,30\nDelta,40\nEcho,50\n").unwrap();
    let refreshed = create_data_artifact(&source, &artifact_directory).unwrap();
    assert_ne!(initial.id, refreshed.id);
    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame_id.clone(),
            artifact: refreshed.clone(),
        })
        .unwrap();
    assert_eq!(frame_named(store.document(), "Orders").id, frame_id);
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Gamma", "30"], vec!["Delta", "40"], vec!["Echo", "50"]]
    );
    assert_eq!(
        frame_named(store.document(), "Orders")
            .artifact
            .as_ref()
            .unwrap()
            .id,
        refreshed.id
    );

    store.undo();
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Alpha", "10"], vec!["Beta", "20"]]
    );
    store.redo();

    let name_id = frame_named(store.document(), "Orders").columns[0]
        .id
        .clone();
    let changed_source = directory.join("orders-changed.csv");
    fs::write(&changed_source, "name,total\nFoxtrot,60\n").unwrap();
    let changed = create_data_artifact(&changed_source, &artifact_directory).unwrap();
    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame_id.clone(),
            artifact: changed,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Orders");
    assert_eq!(frame.columns[0].id, name_id);
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["name", "total"]
    );
    assert!(frame.columns[1].id.starts_with("total~"));
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Foxtrot", "60"]]
    );
    store.undo();
    assert_eq!(
        frame_named(store.document(), "Orders")
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["name", "amount"]
    );
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Gamma", "30"], vec!["Delta", "40"], vec!["Echo", "50"]]
    );
    fs::remove_dir_all(directory).unwrap();
}

/// Repointing keeps the frame and everything downstream of it: same frame
/// ID, same column IDs, so a derived frame built on the old file goes on
/// reading the new one without being rebuilt.
#[test]
fn a_frame_can_be_repointed_at_another_file_that_matches_its_columns() {
    let directory = temporary_test_directory("repoint-source");
    let artifact_directory = directory.join("artifacts");
    let january = directory.join("january.csv");
    let february = directory.join("february.csv");
    fs::write(&january, "name,amount\nAlpha,10\nBeta,20\n").unwrap();
    fs::write(&february, "name,amount\nGamma,30\n").unwrap();

    let mut store = Store::new(Document::blank("Repoint"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&january, &artifact_directory).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: january.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    let column_ids: Vec<Id> = frame_named(store.document(), "Orders")
        .columns
        .iter()
        .map(|column| column.id.clone())
        .collect();

    store
        .apply(Operation::SetFrameSource {
            frame_id: frame_id.clone(),
            artifact: create_data_artifact(&february, &artifact_directory).unwrap(),
            connector: ConnectorRecipe::File {
                source_path: february.display().to_string(),
            },
        })
        .unwrap();

    let frame = frame_named(store.document(), "Orders");
    assert_eq!(frame.id, frame_id);
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| column.id.clone())
            .collect::<Vec<_>>(),
        column_ids
    );
    // The connector moved too, so a later refresh reads February, not January.
    assert!(matches!(
        frame.connector.as_ref().unwrap(),
        ConnectorRecipe::File { source_path } if source_path == &february.display().to_string()
    ));
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Gamma", "30"]]
    );

    store.undo();
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Alpha", "10"], vec!["Beta", "20"]]
    );
    fs::remove_dir_all(directory).unwrap();
}

/// A moved source may have evolved too. Fields still present keep identity,
/// additions get readable identities, and unused deletions simply disappear.
#[test]
fn repointing_reconciles_a_file_whose_columns_differ() {
    let directory = temporary_test_directory("repoint-mismatch");
    let artifact_directory = directory.join("artifacts");
    let orders = directory.join("orders.csv");
    let other = directory.join("other.csv");
    fs::write(&orders, "name,amount\nAlpha,10\n").unwrap();
    fs::write(&other, "name,total\nBeta,20\n").unwrap();

    let mut store = Store::new(Document::blank("Repoint"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&orders, &artifact_directory).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: orders.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    let name_id = frame_named(store.document(), "Orders").columns[0]
        .id
        .clone();

    store
        .apply(Operation::SetFrameSource {
            frame_id: frame_id.clone(),
            artifact: create_data_artifact(&other, &artifact_directory).unwrap(),
            connector: ConnectorRecipe::File {
                source_path: other.display().to_string(),
            },
        })
        .unwrap();
    let frame = frame_named(store.document(), "Orders");
    assert_eq!(frame.columns[0].id, name_id);
    assert!(frame.columns[1].id.starts_with("total~"));
    assert!(matches!(
        frame.connector.as_ref().unwrap(),
        ConnectorRecipe::File { source_path } if source_path == &other.display().to_string()
    ));
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Beta", "20"]]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn renaming_an_imported_column_does_not_rename_its_source_field() {
    let directory = temporary_test_directory("rename-import-column");
    let artifact_directory = directory.join("artifacts");
    let source = directory.join("orders.csv");
    fs::write(&source, "name,amount\nAlpha,10\n").unwrap();
    let mut store = Store::new(Document::blank("Rename import"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&source, &artifact_directory).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Orders");
    let frame_id = frame.id.clone();
    let amount_id = frame.columns[1].id.clone();
    assert!(amount_id.starts_with("amount~"));

    store
        .apply(Operation::RenameColumn {
            frame_id: frame_id.clone(),
            column_id: amount_id.clone(),
            name: "Revenue".into(),
        })
        .unwrap();

    let amount = &frame_named(store.document(), "Orders").columns[1];
    assert_eq!(amount.id, amount_id);
    assert_eq!(amount.name, "Revenue");
    assert_eq!(amount.source_name.as_deref(), Some("amount"));
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Alpha", "10"]]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_missing_source_field_keeps_its_id_when_downstream_reads_it() {
    let directory = temporary_test_directory("missing-source-column");
    let artifact_directory = directory.join("artifacts");
    let original = directory.join("orders.csv");
    let replacement = directory.join("orders-moved.csv");
    fs::write(&original, "name,amount\nAlpha,10\n").unwrap();
    fs::write(&replacement, "name,total\nBeta,20\n").unwrap();
    let mut store = Store::new(Document::blank("Missing source field"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&original, &artifact_directory).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: original.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Orders");
    let frame_id = frame.id.clone();
    let amount_id = frame.columns[1].id.clone();
    store
        .apply(Operation::AddLinkedFrame {
            source_frame_id: frame_id.clone(),
            name: "Linked orders".into(),
            x: 500.0,
            y: 0.0,
        })
        .unwrap();

    store
        .apply(Operation::SetFrameSource {
            frame_id: frame_id.clone(),
            artifact: create_data_artifact(&replacement, &artifact_directory).unwrap(),
            connector: ConnectorRecipe::File {
                source_path: replacement.display().to_string(),
            },
        })
        .unwrap();

    let frame = frame_named(store.document(), "Orders");
    assert!(frame.columns.iter().any(|column| column.id == amount_id));
    assert!(
        frame
            .columns
            .iter()
            .any(|column| column.id.starts_with("total~"))
    );
    let error = store
        .get_frame_page(&frame_id, 0, 10)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Source field ‘amount’"));
    assert!(error.contains(&amount_id));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn source_additions_flow_through_a_chain_that_does_not_select_them_away() {
    let directory = temporary_test_directory("source-addition-through-chain");
    let artifact_directory = directory.join("artifacts");
    let original = directory.join("orders.csv");
    let replacement = directory.join("orders-expanded.csv");
    fs::write(&original, "name,amount\nAlpha,10\n").unwrap();
    fs::write(&replacement, "name,amount,currency\nBeta,20,CAD\n").unwrap();
    let mut store = Store::new(Document::blank("Expanded source"));
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Orders".into(),
            artifact: create_data_artifact(&original, &artifact_directory).unwrap(),
            connector: Some(ConnectorRecipe::File {
                source_path: original.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Orders").id.clone();
    store
        .apply(Operation::SetFramePipeline {
            frame_id: frame_id.clone(),
            steps: vec![FrameStepInput::Filter {
                predicates: vec!["`amount` > 0".into()],
                match_all: true,
            }],
        })
        .unwrap();

    store
        .apply(Operation::SetFrameSource {
            frame_id: frame_id.clone(),
            artifact: create_data_artifact(&replacement, &artifact_directory).unwrap(),
            connector: ConnectorRecipe::File {
                source_path: replacement.display().to_string(),
            },
        })
        .unwrap();

    let frame = frame_named(store.document(), "Orders");
    assert!(
        frame
            .columns
            .iter()
            .any(|column| { column.name == "currency" && column.id.starts_with("currency~") })
    );
    assert!(
        frame
            .base_columns
            .iter()
            .any(|column| column.name == "currency")
    );
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows,
        vec![vec!["Beta", "20", "CAD"]]
    );
    fs::remove_dir_all(directory).unwrap();
}

/// Pasting into an empty frame is how a frame gets its shape: the clipboard
/// is read by the same Polars reader a file import uses, so the columns and
/// their types come out the same either way.
#[test]
fn pasting_into_an_empty_frame_builds_typed_columns_from_the_clipboard() {
    let mut store = Store::new(Document::blank("Paste"));
    store
        .apply(Operation::AddFrame {
            name: "Scratch".into(),
            grid: vec![
                vec!["Column 1".into(), "Column 2".into()],
                vec![String::new(), String::new()],
                vec![String::new(), String::new()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Scratch").id.clone();

    store
        .apply(Operation::SetFrameFromPastedText {
            frame_id: frame_id.clone(),
            text: "Item\tAmount\tSold on\nWidget\t3\t2026-01-15\nGadget\t4.5\t2026-02-01\n".into(),
        })
        .unwrap();

    let frame = frame_named(store.document(), "Scratch");
    assert_eq!(
        frame
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.data_type))
            .collect::<Vec<_>>(),
        vec![
            ("Item", DataType::String),
            ("Amount", DataType::Number),
            ("Sold on", DataType::Date),
        ]
    );
    assert_eq!(frame.rows.len(), 2);
    assert_eq!(
        store.get_frame_page(&frame_id, 0, 10).unwrap().rows[0],
        vec!["Widget", "3", "2026-01-15"]
    );

    // A frame with values in it is not a blank slate, so the same paste is
    // refused rather than throwing the data away.
    assert!(matches!(
        store.apply(Operation::SetFrameFromPastedText {
            frame_id,
            text: "Other\nvalue\n".into(),
        }),
        Err(CoreError::InvalidOperation(_))
    ));
}

/// A paste taller than the frame grows it. Clipping instead is the failure
/// that looks like success: eight of ten rows silently missing.
#[test]
fn pasting_past_the_last_row_appends_rows_instead_of_dropping_them() {
    let mut store = Store::new(Document::blank("Paste"));
    store
        .apply(Operation::AddFrame {
            name: "Scratch".into(),
            grid: vec![
                vec!["Name".into(), "Amount".into()],
                vec![String::new(), String::new()],
                vec![String::new(), String::new()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Scratch").clone();

    store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[0].id.clone(),
            grid: (1..=5)
                .map(|index| vec![format!("Row {index}"), (index * 10).to_string()])
                .collect(),
        })
        .unwrap();

    let pasted = frame_named(store.document(), "Scratch");
    assert_eq!(pasted.rows.len(), 5, "the frame grew to fit the paste");
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().rows,
        (1..=5)
            .map(|index| vec![format!("Row {index}"), (index * 10).to_string()])
            .collect::<Vec<_>>()
    );
    // The two rows that already existed kept their identities.
    assert_eq!(pasted.rows[0].id, frame.rows[0].id);
    assert_eq!(pasted.rows[1].id, frame.rows[1].id);

    store.undo();
    assert_eq!(frame_named(store.document(), "Scratch").rows.len(), 2);
}

/// Pasting a single column into one column fills that column and nothing
/// else — a paste is not allowed to widen a frame, because a schema change
/// made by accident is not a paste.
#[test]
fn pasting_one_column_leaves_the_others_alone_and_never_widens() {
    let mut store = Store::new(Document::blank("Paste"));
    store
        .apply(Operation::AddFrame {
            name: "Scratch".into(),
            grid: vec![
                vec!["Name".into(), "Amount".into()],
                vec!["Alpha".into(), "1".into()],
            ],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Scratch").clone();

    store
        .apply(Operation::PasteCells {
            frame_id: frame.id.clone(),
            row_id: frame.rows[0].id.clone(),
            column_id: frame.columns[1].id.clone(),
            // Two columns wide, but only one column remains to its right.
            grid: vec![
                vec!["11".into(), "ignored".into()],
                vec!["22".into(), "ignored".into()],
            ],
        })
        .unwrap();

    let pasted = frame_named(store.document(), "Scratch");
    assert_eq!(pasted.columns.len(), 2, "a paste never adds columns");
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 10).unwrap().rows,
        vec![vec!["Alpha", "11"], vec!["", "22"]]
    );
}

/// Whether a frame can be typed into is a property of the frame, and the
/// answer has to travel with it rather than be worked out again by whoever
/// draws the grid.
#[test]
fn a_frame_reports_what_can_be_edited_by_hand_and_why_not() {
    let directory = temporary_test_directory("editing-metadata");
    let source = directory.join("ledger.csv");
    fs::write(&source, "Period,Debit\n2024-01,100\n2024-02,20\n").unwrap();
    let document = Document::blank("Ledger");
    let artifact_directory =
        CollaborationPaths::for_document(&directory.join("l.fw"), &document.id)
            .unwrap()
            .root
            .join("data");
    let mut store = Store::new(document);

    store
        .apply(Operation::AddFrame {
            name: "Assumptions".into(),
            grid: vec![vec!["Rate".into()], vec!["0.08".into()]],
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let typed_id = frame_named(store.document(), "Assumptions").id.clone();
    let typed = &store.view().computed_frames[&typed_id];
    assert!(typed.editing.cells, "a frame someone typed in is theirs");
    assert!(typed.editing.rows);
    assert_eq!(typed.editing.reason, None, "nothing to explain");
    assert_eq!(typed.source_name, None, "it reads from nothing");

    let artifact = create_data_artifact(&source, &artifact_directory).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    let imported = &store.view().computed_frames[&ledger_id];
    assert!(!imported.editing.cells);
    assert!(!imported.editing.rows);
    assert_eq!(
        imported.source_name.as_deref(),
        Some("ledger.csv"),
        "named the short way, for a list"
    );
    assert!(
        imported
            .editing
            .reason
            .as_ref()
            .unwrap()
            .contains("ledger.csv"),
        "the explanation names the file the rows actually come from"
    );

    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id.clone(),
            name: "By period".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Period".into(),
                formula: "`Period`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Debit total".into(),
                formula: "`Debit`.sum()".into(),
            }],
            maintain_order: true,
            x: 800.0,
            y: 0.0,
        })
        .unwrap();
    let grouped_id = frame_named(store.document(), "By period").id.clone();
    let derived = &store.view().computed_frames[&grouped_id];
    assert!(!derived.editing.cells);
    assert!(derived.editing.reason.as_ref().unwrap().contains("chain"));
    assert_eq!(derived.source_name, None, "it reads a frame, not a file");

    // And the report is the rule, not a description of one enforced
    // elsewhere: what it says cannot be done is refused.
    let row_id = store
        .view()
        .document
        .objects
        .iter()
        .find_map(|object| match object {
            DataObject::Frame(frame) if frame.id == ledger_id => frame.rows.first().cloned(),
            _ => None,
        })
        .map(|row| row.id);
    let error = store
        .apply(Operation::AddRow {
            frame_id: ledger_id.clone(),
            values: Default::default(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("come from its source"));
    if let Some(row_id) = row_id {
        let error = store
            .apply(Operation::DeleteRow {
                frame_id: ledger_id,
                row_id,
            })
            .unwrap_err();
        assert!(error.to_string().contains("come from its source"));
    }
    fs::remove_dir_all(directory).unwrap();
}

/// Whether a frame is live — whether its values can move without anyone
/// editing the document — is a fact about its whole lineage, not about its
/// own definition, and it decides what a hand edit would even mean.
#[test]
fn liveness_travels_down_the_lineage_and_shapes_what_an_edit_would_mean() {
    let directory = temporary_test_directory("liveness");
    let source = directory.join("ledger.csv");
    fs::write(&source, "Period,Debit\n2024-01,100\n2024-02,20\n").unwrap();
    let document = Document::blank("Ledger");
    let artifact_directory =
        CollaborationPaths::for_document(&directory.join("l.fw"), &document.id)
            .unwrap()
            .root
            .join("data");
    let mut store = Store::new(document);

    // The same bytes imported twice: once keeping the connector that can
    // re-read the original, once as a copy that answers to nobody.
    for (name, connector) in [
        (
            "Live ledger",
            Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
        ),
        ("Static ledger", None),
    ] {
        let artifact = create_data_artifact(&source, &artifact_directory).unwrap();
        store
            .apply(Operation::ImportFrameFromArtifact {
                name: name.into(),
                artifact,
                connector,
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
    }
    let live_id = frame_named(store.document(), "Live ledger").id.clone();
    let static_id = frame_named(store.document(), "Static ledger").id.clone();

    let view = store.view();
    assert!(view.computed_frames[&live_id].live);
    assert!(!view.computed_frames[&static_id].live);
    assert!(
        view.computed_frames[&live_id]
            .editing
            .reason
            .as_ref()
            .unwrap()
            .contains("refreshing replaces them"),
        "the reason a live frame refuses an edit is what happens next, not where it came from"
    );
    assert!(
        view.computed_frames[&static_id].editing.cells,
        "nothing will ever refresh over a static import, so its values are the \
         document's own to type into"
    );
    assert!(
        !view.computed_frames[&live_id].editing.cells,
        "and a connector is exactly the thing that would throw such an edit away"
    );

    // Derived frames inherit it. Reading live numbers makes you live.
    for (name, parent) in [("From live", &live_id), ("From static", &static_id)] {
        store
            .apply(Operation::AddLinkedFrame {
                source_frame_id: parent.clone(),
                name: name.into(),
                x: 400.0,
                y: 0.0,
            })
            .unwrap();
    }
    let view = store.view();
    let from_live = frame_named(&view.document, "From live").id.clone();
    let from_static = frame_named(&view.document, "From static").id.clone();
    assert!(view.computed_frames[&from_live].live);
    assert!(
        !view.computed_frames[&from_static].live,
        "nothing in its lineage re-reads anything, so nothing moves on its own"
    );

    // A derived frame is not editable whichever kind of import it reads:
    // its values are computed, and typing over a computed value is a
    // request to be overwritten on the next read.
    for frame_id in [&from_live, &from_static] {
        assert!(!view.computed_frames[frame_id].editing.cells);
    }
    // And none of the four takes a per-cell override: all are read a page
    // at a time from a parquet, so an override would have no row of the
    // document to be recorded against.
    for frame_id in [&from_live, &from_static, &live_id, &static_id] {
        assert!(!view.computed_frames[frame_id].editing.overrides);
    }
    let row_id = framework_core::id();
    let error = store
        .apply(Operation::SetCellOverride {
            frame_id: from_live,
            row_id,
            column_id: frame_named(&view.document, "From live").columns[0]
                .id
                .clone(),
            formula: Some("1".into()),
        })
        .unwrap_err();
    assert!(error.to_string().contains("page at a time"));
    fs::remove_dir_all(directory).unwrap();
}

/// Taking ownership, and then the thing ownership is for: typing into the
/// values and having them stay typed.
#[test]
fn an_adopted_frame_is_the_documents_own_data_and_takes_hand_edits() {
    let directory = temporary_test_directory("adopt-rows");
    let source = directory.join("ledger.csv");
    fs::write(
        &source,
        "Period,Debit\n2024-01,100\n2024-02,20\n2024-01,5\n",
    )
    .unwrap();
    let document = Document::blank("Ledger");
    let data = CollaborationPaths::for_document(&directory.join("l.fw"), &document.id)
        .unwrap()
        .root
        .join("data");
    let mut store = Store::new(document);
    let artifact = create_data_artifact(&source, &data).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    store
        .apply(Operation::AddDerivedFrame {
            source_frame_id: ledger_id,
            name: "By period".into(),
            group_keys: vec![NamedFormulaInput {
                name: "Period".into(),
                formula: "`Period`".into(),
            }],
            aggregates: vec![NamedFormulaInput {
                name: "Debit total".into(),
                formula: "`Debit`.sum()".into(),
            }],
            maintain_order: true,
            x: 400.0,
            y: 0.0,
        })
        .unwrap();
    let grouped_id = frame_named(store.document(), "By period").id.clone();
    assert!(!store.view().computed_frames[&grouped_id].editing.cells);

    let owned = store.write_owned_frame_data(&grouped_id, &data).unwrap();
    store
        .apply(Operation::AdoptFrameRows {
            frame_id: grouped_id.clone(),
            artifact: owned,
        })
        .unwrap();

    let view = store.view();
    let adopted = frame_named(&view.document, "By period");
    assert!(adopted.derivation.is_none(), "the cord is cut");
    assert!(adopted.connector.is_none());
    assert!(adopted.materialization.is_none());
    assert!(adopted.artifact.is_some(), "and the data is the document's");
    let computed = &view.computed_frames[&grouped_id];
    assert!(computed.editing.cells, "which is what makes it editable");
    assert!(!computed.live, "nothing is left to refresh over it");

    // The values survived the move.
    let page = store.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(page.total_rows, 2);
    assert_eq!(page.rows[0][0], "2024-01");
    assert_eq!(page.rows[0][1].parse::<f64>().unwrap(), 105.0);

    // Now type over one. A scanned row's identity is its ordinal, which is
    // what the page reports as its id.
    let total_column = adopted.columns[1].id.clone();
    store
        .apply(Operation::SetCell {
            frame_id: grouped_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: total_column.clone(),
            raw: "999".into(),
        })
        .unwrap();
    let edited = store.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(edited.rows[0][1].parse::<f64>().unwrap(), 999.0);
    assert_eq!(
        edited.rows[1][1].parse::<f64>().unwrap(),
        20.0,
        "and only that one"
    );

    // Undo puts the file back, and does it by rewriting rather than by
    // remembering which file it used to be.
    store.undo();
    let restored = store.get_frame_page(&grouped_id, 0, 10).unwrap();
    assert_eq!(restored.rows[0][1].parse::<f64>().unwrap(), 105.0);
    store.redo();
    assert_eq!(
        store.get_frame_page(&grouped_id, 0, 10).unwrap().rows[0][1]
            .parse::<f64>()
            .unwrap(),
        999.0
    );

    // Undoing the adoption itself gives back the frame that was there.
    store.undo();
    store.undo();
    let view = store.view();
    let restored = frame_named(&view.document, "By period");
    assert!(
        restored.derivation.is_some(),
        "the chain comes back with everything it knew"
    );
    assert!(!view.computed_frames[&grouped_id].editing.cells);
    fs::remove_dir_all(directory).unwrap();
}

/// A value that does not fit its column is refused before anything is
/// written, rather than landing as a null nobody asked for.
#[test]
fn an_edit_to_owned_data_is_checked_against_the_column_type() {
    let directory = temporary_test_directory("adopt-typed");
    let source = directory.join("ledger.csv");
    fs::write(&source, "Period,Debit\n2024-01,100\n2024-02,20\n").unwrap();
    let document = Document::blank("Ledger");
    let data = CollaborationPaths::for_document(&directory.join("l.fw"), &document.id)
        .unwrap()
        .root
        .join("data");
    let mut store = Store::new(document);
    let artifact = create_data_artifact(&source, &data).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    let debit = frame_named(store.document(), "Ledger").columns[1]
        .id
        .clone();
    let page = store.get_frame_page(&ledger_id, 0, 10).unwrap();

    let error = store
        .apply(Operation::SetCell {
            frame_id: ledger_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: debit.clone(),
            raw: "not a number".into(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("Invalid integer"));
    assert_eq!(
        store.get_frame_page(&ledger_id, 0, 10).unwrap().rows[0][1]
            .parse::<f64>()
            .unwrap(),
        100.0,
        "and the file is untouched"
    );

    // Emptying a cell is a null, which is a value like any other.
    store
        .apply(Operation::SetCell {
            frame_id: ledger_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: debit,
            raw: String::new(),
        })
        .unwrap();
    assert_eq!(
        store.get_frame_page(&ledger_id, 0, 10).unwrap().rows[0][1],
        "",
        "an emptied cell reads back empty"
    );
    fs::remove_dir_all(directory).unwrap();
}

/// Packaging cuts every outside dependency at once, and the sweep reclaims
/// the files that no longer have anything pointing at them.
#[test]
fn packaging_a_document_cuts_its_links_and_the_sweep_reclaims_what_is_left() {
    let directory = temporary_test_directory("package-and-sweep");
    let source = directory.join("ledger.csv");
    fs::write(&source, "Period,Debit\n2024-01,100\n2024-02,20\n").unwrap();
    let document_path = directory.join("ledger.fw");
    let document = Document::blank("Ledger");
    let paths = CollaborationPaths::for_document(&document_path, &document.id).unwrap();
    let data = paths.root.join("data");
    let mut store = Store::new(document);
    let artifact = create_data_artifact(&source, &data).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Ledger".into(),
            artifact,
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let ledger_id = frame_named(store.document(), "Ledger").id.clone();
    assert!(store.view().computed_frames[&ledger_id].live);

    store
        .apply(Operation::PackageDocument {
            adopted: Vec::new(),
        })
        .unwrap();
    let view = store.view();
    assert!(
        frame_named(&view.document, "Ledger").connector.is_none(),
        "nothing is left that reads the file it came from"
    );
    assert!(!view.computed_frames[&ledger_id].live);
    assert!(
        view.computed_frames[&ledger_id].editing.cells,
        "and what nothing will overwrite can be edited"
    );
    assert!(
        store
            .apply(Operation::PackageDocument {
                adopted: Vec::new()
            })
            .is_err(),
        "packaging a document that depends on nothing is not an edit"
    );

    // Undo gives the connection back, in one step rather than one per frame.
    store.undo();
    assert!(frame_named(store.document(), "Ledger").connector.is_some());
    store.redo();

    // Editing rewrites the artifact, leaving the version it replaced behind.
    let debit = frame_named(store.document(), "Ledger").columns[1]
        .id
        .clone();
    let page = store.get_frame_page(&ledger_id, 0, 10).unwrap();
    store
        .apply(Operation::SetCell {
            frame_id: ledger_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: debit,
            raw: "555".into(),
        })
        .unwrap();
    let files = |directory: &std::path::Path| {
        fs::read_dir(directory)
            .map(|entries| entries.flatten().count())
            .unwrap_or_default()
    };
    assert_eq!(
        files(&data),
        2,
        "the version before the edit is still there"
    );

    let journal = EventJournal::open(&document_path, &store.document().id).unwrap();
    let held = store
        .collect_unreferenced_artifacts(&journal, &data)
        .unwrap();
    assert_eq!(
        held.files, 0,
        "undo can still reach the old version, so it stays"
    );

    // Reopening is what lets go: the history goes with the session.
    store.save(&document_path).unwrap();
    let reopened = Store::load(&document_path).unwrap();
    let swept = reopened
        .collect_unreferenced_artifacts(&journal, &data)
        .unwrap();
    assert_eq!(swept.files, 1, "and now the old version is collectable");
    assert!(swept.bytes > 0);
    assert_eq!(files(&data), 1);
    assert_eq!(
        reopened.get_frame_page(&ledger_id, 0, 10).unwrap().rows[0][1]
            .parse::<f64>()
            .unwrap(),
        555.0,
        "the file it still reads was not the one swept"
    );
    fs::remove_dir_all(directory).unwrap();
}
