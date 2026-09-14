//! An exact amount has to survive the trip out of the document and back.
//!
//! The engine holds an accounting column as Decimal at a declared scale, and
//! every handoff format has its own idea of what a number is: Parquet has the
//! decimal type itself, CSV has text, and XLSX has one numeric type, a double.
//! So the rule per destination is written here rather than assumed: Parquet
//! keeps the logical type, delimited text keeps the exact digits, and the
//! workbook receives a real number cell whose value is the amount.

use crate::common::*;
use framework_core::*;
use polars::prelude as pl;
use polars::prelude::{NamedFrom, SerReader};
use std::fs;
use std::path::Path;

/// A parquet holding `Amount` as `Decimal(38, scale)` — the schema a warehouse
/// extract of a ledger actually arrives with.
fn decimal_parquet(path: &Path, amounts: &[f64], scale: usize) {
    let amount = pl::Series::new("Amount".into(), amounts)
        .cast(&pl::DataType::Decimal(ACCOUNTING_PRECISION, scale))
        .unwrap();
    let memo = pl::Series::new(
        "Memo".into(),
        &(0..amounts.len())
            .map(|index| format!("Line {}", index + 1))
            .collect::<Vec<_>>(),
    );
    let mut frame = pl::DataFrame::new(
        amounts.len(),
        vec![memo.into(), amount.with_name("Amount".into()).into()],
    )
    .unwrap();
    pl::ParquetWriter::new(fs::File::create(path).unwrap())
        .finish(&mut frame)
        .unwrap();
}

fn open_artifact(store: &mut Store, name: &str, source: &Path) -> FrameObject {
    let artifact = create_data_artifact(
        source,
        &source
            .parent()
            .expect("fixture has a directory")
            .join("data"),
    )
    .unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: name.into(),
            artifact,
            // A refreshable import: the scenario under test is a ledger
            // extract that gets re-pulled, not a one-off paste.
            connector: Some(ConnectorRecipe::File {
                source_path: source.display().to_string(),
            }),
            file_origin: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    frame_named(store.document(), name).clone()
}

fn column<'a>(frame: &'a FrameObject, name: &str) -> &'a Column {
    frame
        .columns
        .iter()
        .find(|column| column.name == name)
        .unwrap()
}

/// The footer total, asked for the way the frame asks for it: a paged frame
/// has no rows in the document, so the aggregate is a pass over the source
/// rather than a fold over literal cells.
fn footer_sum(store: &mut Store, frame: &FrameObject, column_name: &str) -> ComputedCell {
    let column_id = column(frame, column_name).id.clone();
    if !frame
        .summaries
        .iter()
        .any(|summary| summary.column_id == column_id)
    {
        store
            .apply(Operation::AddSummary {
                frame_id: frame.id.clone(),
                column_id: column_id.clone(),
                operation: SummaryOperation::Sum,
            })
            .unwrap();
    }
    store
        .get_frame_summary(&frame.id)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.operation == SummaryOperation::Sum)
        .and_then(|row| row.cells.get(&column_id).cloned())
        .expect("the footer carries a sum for the amount column")
}

#[test]
fn a_parquet_decimal_column_arrives_as_an_amount_and_stays_exact() {
    let directory = temporary_test_directory("accounting-parquet");
    let source = directory.join("ledger.parquet");
    // Three thousandths of a unit each: a scale the default of two would
    // round away, so the file's own scale is what is being observed.
    decimal_parquet(&source, &[0.100, 0.200, 0.305], 3);

    let mut store = Store::new(Document::blank("Ledger"));
    let frame = open_artifact(&mut store, "Ledger", &source);
    assert_eq!(column(&frame, "Amount").data_type, DataType::Accounting);

    // The plan keeps the file's scale rather than rounding to the default of
    // two, which is what makes the total below foot to the third place.
    let total = footer_sum(&mut store, &frame, "Amount");
    assert!(total.error.is_none(), "{:?}", total.error);
    assert_eq!(total.typed_value, ScalarValue::Number(0.605));

    // A refresh against a new extract of the same schema keeps all of it.
    let refreshed_source = directory.join("ledger-2.parquet");
    decimal_parquet(&refreshed_source, &[0.100, 0.200, 0.305, 1000.0], 3);
    let artifact = create_data_artifact(&refreshed_source, &directory.join("data")).unwrap();
    store
        .apply(Operation::RefreshFrameArtifact {
            frame_id: frame.id.clone(),
            artifact,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    assert_eq!(column(&frame, "Amount").data_type, DataType::Accounting);
    assert_eq!(
        store.get_frame_page(&frame.id, 0, 50).unwrap().total_rows,
        4
    );
    let total = footer_sum(&mut store, &frame, "Amount");
    assert_eq!(total.typed_value, ScalarValue::Number(1000.605));

    // Out the other side it is still a decimal at the file's own scale, so a
    // snapshot of an imported ledger is not a lossier file than the one it
    // was read from.
    let exported = directory.join("ledger-out.parquet");
    store.export_frame_file(&frame.id, &exported).unwrap();
    let read = pl::ParquetReader::new(fs::File::open(&exported).unwrap())
        .finish()
        .unwrap();
    assert_eq!(
        read.column("Amount").unwrap().dtype(),
        &pl::DataType::Decimal(ACCOUNTING_PRECISION, 3)
    );

    fs::remove_dir_all(directory).unwrap();
}

/// A paged frame renders its cells through the float scalar layer, so an
/// amount on an artifact-backed page loses the spelling the column promises:
/// `0.100` comes back as `0.1`. The document-owned path does not have this
/// problem — `engine/values.rs::frame_rows_from_polars` asks
/// `decimal_text_at` first — and the page loop at
/// `engine/plan.rs::get_frame_page` is the one place that does not. Ignored
/// rather than deleted: it is the statement of what the page should say, and
/// it lives in `engine/`, which this slice does not own.
#[test]
fn an_amount_on_a_paged_frame_should_show_its_declared_places() {
    let directory = temporary_test_directory("accounting-paged-text");
    let source = directory.join("ledger.parquet");
    decimal_parquet(&source, &[0.100, 0.200, 0.305], 3);
    let mut store = Store::new(Document::blank("Ledger"));
    let frame = open_artifact(&mut store, "Ledger", &source);
    let page = store.get_frame_page(&frame.id, 0, 50).unwrap();
    assert_eq!(
        page.rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        vec!["0.100", "0.200", "0.305"]
    );
    fs::remove_dir_all(directory).unwrap();
}

/// A file's decimal scale is the column's declared scale on the way in, so
/// the grid shows the places the file kept and a later replacement at the
/// same field keeps them.
#[test]
fn an_imported_amount_carries_the_files_scale() {
    let directory = temporary_test_directory("accounting-parquet-scale");
    let source = directory.join("ledger.parquet");
    decimal_parquet(&source, &[0.100, 0.200, 0.305], 3);
    let mut store = Store::new(Document::blank("Ledger"));
    let frame = open_artifact(&mut store, "Ledger", &source);
    assert_eq!(column(&frame, "Amount").data_type, DataType::Accounting);
    assert_eq!(column(&frame, "Amount").scale, Some(3));
    fs::remove_dir_all(directory).unwrap();
}

/// A document-owned ledger whose `Amount` is accounting at two places, with
/// the three amounts a float gets wrong.
fn typed_ledger(amounts: &[&str]) -> (Store, FrameObject) {
    let mut store = Store::new(Document::blank("Books"));
    let mut grid = vec![vec!["Memo".to_string(), "Amount".to_string()]];
    for (index, amount) in amounts.iter().enumerate() {
        grid.push(vec![format!("Line {}", index + 1), amount.to_string()]);
    }
    store
        .apply(Operation::AddFrame {
            name: "Ledger".into(),
            grid,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: column(&frame, "Amount").id.clone(),
            data_type: DataType::Accounting,
            scale: None,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Ledger").clone();
    (store, frame)
}

#[test]
fn exporting_an_amount_keeps_the_cents_in_every_format() {
    let directory = temporary_test_directory("accounting-export");
    let (store, frame) = typed_ledger(&["0.10", "0.20", "0.30"]);

    // Delimited text is the strict case: the digits themselves are the value,
    // so a trailing cent must be there and a float's noise must not.
    let csv = directory.join("ledger.csv");
    store.export_frame_file(&frame.id, &csv).unwrap();
    assert_eq!(
        fs::read_to_string(&csv).unwrap(),
        "Memo,Amount\nLine 1,0.10\nLine 2,0.20\nLine 3,0.30\n"
    );

    // Parquet keeps the logical type, so the reader on the other side gets a
    // decimal column and not a double it has to trust.
    let parquet = directory.join("ledger.parquet");
    store.export_frame_file(&frame.id, &parquet).unwrap();
    let read = pl::ParquetReader::new(fs::File::open(&parquet).unwrap())
        .finish()
        .unwrap();
    assert_eq!(
        read.column("Amount").unwrap().dtype(),
        &pl::DataType::Decimal(ACCOUNTING_PRECISION, 2)
    );

    // A workbook has one numeric type. The amount goes in as a number at its
    // exact value — never as text, which is what makes Excel's own SUM foot.
    let workbook = directory.join("ledger.xlsx");
    store
        .export_excel(std::slice::from_ref(&frame.id), &workbook, false)
        .unwrap();
    let preview = preview_excel_range(&workbook, "Ledger", "A1:B4", true, 10).unwrap();
    assert_eq!(preview.columns, vec!["Memo", "Amount"]);
    assert_eq!(
        preview.rows,
        vec![
            vec!["Line 1", "0.1"],
            vec!["Line 2", "0.2"],
            vec!["Line 3", "0.3"],
        ],
        "a number cell, read back as a number — not the string \"0.10\""
    );

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn writing_an_amount_back_to_its_csv_keeps_the_exact_digits() {
    let directory = temporary_test_directory("accounting-writeback");
    let path = directory.join("journal.csv");
    fs::write(&path, "Memo,Amount\nOpening,10.00\nFees,-0.05\n").unwrap();

    let mut store = Store::new(Document::blank("Books"));
    let artifact = create_data_artifact(&path, &directory.join("data")).unwrap();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Journal".into(),
            artifact,
            connector: None,
            file_origin: Some(path.display().to_string()),
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Journal").clone();
    store
        .apply(Operation::SetColumnType {
            frame_id: frame.id.clone(),
            column_id: column(&frame, "Amount").id.clone(),
            data_type: DataType::Accounting,
            scale: Some(2),
        })
        .unwrap();
    let frame = frame_named(store.document(), "Journal").clone();

    // A calculated column over the amounts, so the write-back has a value with
    // no source token to fall back on and must spell it itself.
    store
        .apply(Operation::AddComputedColumn {
            frame_id: frame.id.clone(),
            name: "Doubled".into(),
            formula: "`Amount` * 2".into(),
            after_column_id: None,
        })
        .unwrap();
    let frame = frame_named(store.document(), "Journal").clone();
    assert_eq!(column(&frame, "Doubled").data_type, DataType::Accounting);

    let prepared = store
        .prepare_delimited_write(&frame.id, &directory.join("recovery"))
        .unwrap();
    prepared.write().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "Memo,Amount,Doubled\nOpening,10.00,20.00\nFees,-0.05,-0.10\n",
        "the amounts keep their own spelling and the derived column is \
         written at its scale rather than as the nearest float"
    );

    fs::remove_dir_all(directory).unwrap();
}
