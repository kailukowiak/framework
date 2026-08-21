use crate::common::*;
use framework_core::*;
use polars::prelude as pl;
use polars::prelude::NamedFrom;
use std::fs;
use std::time::Instant;

/// Enough disorder that parquet cannot compress the file down to nothing,
/// which is what makes the measurement about real bytes.
fn noise(seed: usize) -> usize {
    let mut value = seed as u64 ^ 0x9e37_79b9_7f4a_7c15;
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    (value % 1_000_000) as usize
}

/// What one hand edit to owned data costs, since the answer decides whether
/// edits need buffering behind a Save.
///
/// Measured 2026-08-12 on 1.18M rows in a 30.7MB parquet: ~600ms for the
/// edit and ~600ms for the undo. Linear in file size, so ~5ms at 10k rows
/// and ~50ms at 100k — invisible across every frame anybody hand-edits, and
/// slow only where hand-editing has stopped being a plausible thing to do.
/// That is the case against a Save button: it would buy nothing at the sizes
/// people edit, and cost the property that the file always *is* the data.
///
/// Ignored because it writes a million rows. Run it with
/// `cargo test -p framework-core --test edit_cost_probe --release -- --ignored --nocapture`.
#[test]
#[ignore]
fn probe_edit_cost() {
    let directory = temporary_test_directory("edit-cost");
    let rows = 1_180_000usize;
    let source = directory.join("big.parquet");
    let mut frame = pl::DataFrame::new(
        rows,
        vec![
            pl::Series::new(
                "Period".into(),
                (0..rows)
                    .map(|i| format!("2024-{:02}", 1 + (noise(i) % 12)))
                    .collect::<Vec<_>>(),
            )
            .into(),
            pl::Series::new(
                "Debit".into(),
                (0..rows)
                    .map(|i| noise(i) as f64 / 97.0)
                    .collect::<Vec<_>>(),
            )
            .into(),
            pl::Series::new(
                "Credit".into(),
                (0..rows)
                    .map(|i| noise(i * 7) as f64 / 13.0)
                    .collect::<Vec<_>>(),
            )
            .into(),
            pl::Series::new(
                "Account".into(),
                (0..rows)
                    .map(|i| format!("{}-{}-{}", noise(i), noise(i * 3), noise(i * 11)))
                    .collect::<Vec<_>>(),
            )
            .into(),
            pl::Series::new(
                "Memo".into(),
                (0..rows)
                    .map(|i| format!("entry {} for {}", noise(i * 5), noise(i * 13)))
                    .collect::<Vec<_>>(),
            )
            .into(),
        ],
    )
    .unwrap();
    pl::ParquetWriter::new(fs::File::create(&source).unwrap())
        .finish(&mut frame)
        .unwrap();

    let document = Document::blank("Big");
    let data = CollaborationPaths::for_document(&directory.join("b.fw"), &document.id)
        .unwrap()
        .root
        .join("data");
    let mut store = Store::new(document);
    let artifact = create_data_artifact(&source, &data).unwrap();
    let size = fs::metadata(&artifact.path).unwrap().len();
    store
        .apply(Operation::ImportFrameFromArtifact {
            name: "Big".into(),
            artifact,
            connector: None,
            x: 0.0,
            y: 0.0,
        })
        .unwrap();
    let frame_id = frame_named(store.document(), "Big").id.clone();
    let column = frame_named(store.document(), "Big").columns[1].id.clone();
    let page = store.get_frame_page(&frame_id, 0, 1).unwrap();

    let started = Instant::now();
    store
        .apply(Operation::SetCell {
            frame_id: frame_id.clone(),
            row_id: page.row_ids[0].clone(),
            column_id: column,
            raw: "999".into(),
        })
        .unwrap();
    let elapsed = started.elapsed();
    let undo_started = Instant::now();
    store.undo();
    println!(
        "PROBE rows={rows} parquet={:.1}MB edit={:?} undo={:?}",
        size as f64 / 1_048_576.0,
        elapsed,
        undo_started.elapsed()
    );
    fs::remove_dir_all(directory).unwrap();
}
