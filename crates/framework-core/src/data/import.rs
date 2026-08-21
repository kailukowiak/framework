//! Reading source files into frames and writing the parquet artifacts
//! that back a paged frame.

use crate::*;
use polars::prelude as pl;
use polars::prelude::{NamedFrom, SerReader};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Read a CSV, TSV, or Parquet file into a Polars frame for import. Reading
/// stops one row past [`MAX_IMPORT_ROWS`] so oversized files are rejected
/// without loading completely into memory.
pub(crate) fn read_import_frame(path: &Path) -> Result<pl::DataFrame, CoreError> {
    let import_error = |error: pl::PolarsError| CoreError::Import(error.to_string());
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "csv" | "tsv" => pl::CsvReadOptions::default()
            .with_has_header(true)
            .with_n_rows(Some(MAX_IMPORT_ROWS + 1))
            .with_parse_options(
                pl::CsvParseOptions::default()
                    .with_separator(if extension == "tsv" { b'\t' } else { b',' })
                    .with_try_parse_dates(true),
            )
            .try_into_reader_with_file_path(Some(path.to_path_buf()))
            .map_err(import_error)?
            .finish()
            .map_err(import_error),
        "parquet" => {
            let file =
                fs::File::open(path).map_err(|error| CoreError::Import(error.to_string()))?;
            pl::ParquetReader::new(file)
                .with_slice(Some((0, MAX_IMPORT_ROWS + 1)))
                .finish()
                .map_err(import_error)
        }
        _ => Err(CoreError::Import(
            "Only .csv, .tsv, and .parquet files can be imported".into(),
        )),
    }
}

/// Read pasted text as a frame, the same way a file import is read.
///
/// This is the point of routing paste through the core at all: the browser
/// hands over a string, and splitting it in TypeScript means a second,
/// worse parser — one that has to be taught about quoted fields containing
/// separators, embedded newlines, and every date format. Polars already
/// knows, and a pasted column of dates should become a date column for
/// exactly the reason an imported one does.
///
/// The separator is sniffed from the header line. Anything copied out of a
/// spreadsheet arrives tab-separated; anything copied out of a text file is
/// usually a comma.
pub(crate) fn read_pasted_frame(text: &str) -> Result<pl::DataFrame, CoreError> {
    let trimmed = text.trim_end_matches(['\n', '\r']);
    if trimmed.trim().is_empty() {
        return Err(CoreError::Import("There is nothing to paste".into()));
    }
    let header = trimmed.lines().next().unwrap_or_default();
    let separator = if header.matches('\t').count() >= header.matches(',').count() {
        b'\t'
    } else {
        b','
    };
    pl::CsvReadOptions::default()
        .with_has_header(true)
        .with_n_rows(Some(MAX_IMPORT_ROWS + 1))
        .with_parse_options(
            pl::CsvParseOptions::default()
                .with_separator(separator)
                .with_try_parse_dates(true),
        )
        .into_reader_with_file_handle(std::io::Cursor::new(trimmed.as_bytes().to_vec()))
        .finish()
        .map_err(|error| CoreError::Import(error.to_string()))
}

pub(crate) fn read_import_frame_full(path: &Path) -> Result<pl::DataFrame, CoreError> {
    let import_error = |error: pl::PolarsError| CoreError::Import(error.to_string());
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "csv" | "tsv" => pl::CsvReadOptions::default()
            .with_has_header(true)
            .with_parse_options(
                pl::CsvParseOptions::default()
                    .with_separator(if extension == "tsv" { b'\t' } else { b',' })
                    .with_try_parse_dates(true),
            )
            .try_into_reader_with_file_path(Some(path.to_path_buf()))
            .map_err(import_error)?
            .finish()
            .map_err(import_error),
        "parquet" => {
            let file =
                fs::File::open(path).map_err(|error| CoreError::Import(error.to_string()))?;
            pl::ParquetReader::new(file).finish().map_err(import_error)
        }
        _ => Err(CoreError::Import(
            "Only .csv, .tsv, and .parquet files can be imported".into(),
        )),
    }
}

/// Rewrites one cell of a parquet artifact, returning the artifact that
/// replaces it.
///
/// Parquet has no in-place write, so this is a read and a rewrite of the
/// whole file — the honest cost of editing data that lives in a columnar
/// file rather than in the document. It is charged once per committed edit,
/// not per keystroke.
///
/// The result is content-addressed like any other artifact, which does two
/// useful things for free. The lineage fingerprint moves, so anything cached
/// downstream knows it is out of date; and undoing an edit rewrites the file
/// back to bytes that hash to the artifact that is still sitting there, so
/// going back and forth costs no extra copies.
pub(crate) fn write_artifact_cell(
    artifact: &DataArtifact,
    column_name: &str,
    data_type: DataType,
    row_ordinal: usize,
    raw: &str,
) -> Result<DataArtifact, CoreError> {
    let import_error = |error: pl::PolarsError| CoreError::Import(error.to_string());
    let path = Path::new(&artifact.path);
    let directory = path
        .parent()
        .ok_or_else(|| CoreError::Persistence("This frame's data has no directory".into()))?;
    let mut frame = pl::LazyFrame::scan_parquet(
        pl::PlRefPath::new(&artifact.path),
        pl::ScanArgsParquet::default(),
    )
    .and_then(|scan| scan.collect())
    .map_err(import_error)?;

    let series = frame
        .column(column_name)
        .map_err(import_error)?
        .as_materialized_series()
        .clone();
    if row_ordinal >= series.len() {
        return Err(CoreError::RowNotFound);
    }
    let value = parse_scalar_value(raw, data_type).map_err(CoreError::Import)?;
    let replacement = single_value_series(column_name, &value)
        .cast(series.dtype())
        .map_err(import_error)?;

    // Spliced rather than scattered: `slice` and `append` mean the same
    // thing for every dtype, where the in-place write APIs differ per type
    // and per Polars version.
    let mut updated = series.slice(0, row_ordinal);
    updated.append(&replacement).map_err(import_error)?;
    updated
        .append(&series.slice(row_ordinal as i64 + 1, series.len() - row_ordinal - 1))
        .map_err(import_error)?;
    frame
        .replace(column_name, updated.into())
        .map_err(import_error)?;

    write_frame_artifact(frame, directory, &artifact.source_name)
}

/// The one-element series an edited cell becomes, typed the way a literal
/// column of the same type is built.
fn single_value_series(name: &str, value: &ScalarValue) -> pl::Series {
    let name: pl::PlSmallStr = name.to_string().into();
    match value {
        ScalarValue::Null => pl::Series::new(name, [None::<&str>]),
        ScalarValue::String(value) => pl::Series::new(name, [Some(value.as_str())]),
        ScalarValue::Number(value) => pl::Series::new(name, [Some(*value)]),
        ScalarValue::Boolean(value) => pl::Series::new(name, [Some(*value)]),
        ScalarValue::Date(value) => pl::Series::new(name, [Some(*value)]),
    }
}

/// The value a parquet artifact holds at one cell, as the raw text an edit
/// would have replaced. What makes an edit undoable.
pub(crate) fn read_artifact_cell(
    artifact: &DataArtifact,
    column_name: &str,
    row_ordinal: usize,
) -> Result<String, CoreError> {
    let import_error = |error: pl::PolarsError| CoreError::Import(error.to_string());
    let frame = pl::LazyFrame::scan_parquet(
        pl::PlRefPath::new(&artifact.path),
        pl::ScanArgsParquet::default(),
    )
    .map_err(import_error)?
    .select([pl::col(column_name)])
    .slice(row_ordinal as i64, 1)
    .collect()
    .map_err(import_error)?;
    let series = frame
        .column(column_name)
        .map_err(import_error)?
        .as_materialized_series()
        .clone();
    if series.is_empty() {
        return Err(CoreError::RowNotFound);
    }
    Ok(scalar_value_to_raw(
        polars_value_at(&series, 0).map_err(CoreError::Import)?,
    ))
}

/// Writes `frame` to parquet in `data_directory` and describes the result.
///
/// The file is named by its own SHA-256, exactly like an imported artifact:
/// two snapshots with identical bytes are one file on disk, and rewriting a
/// snapshot that did not actually change costs nothing. Columns keep the
/// names the frame carries -- for a derived frame those are column *ids*, so
/// two columns sharing a display name still round-trip.
/// A one-column frame holding an answer, at whatever length it is.
///
/// Parquet for a single number is not the obvious storage, and it is the
/// right one: a frozen answer is then the same kind of thing as every other
/// recorded read in this document, so the machinery that relocates artifacts
/// on Save As, sweeps unreferenced ones, and carries them between machines
/// needs to learn nothing about it. It is also what let a frozen *column*
/// land here without a format change, which is what a scratchpad line
/// reading a frame with no snapshot needs to write down.
pub(crate) fn frame_of_series(name: &str, series: pl::Series) -> Result<pl::DataFrame, CoreError> {
    let height = series.len();
    let column: pl::Column = series.with_name(name.into()).into();
    pl::DataFrame::new(height, vec![column])
        .map_err(|error| CoreError::Persistence(error.to_string()))
}

pub(crate) fn write_frame_artifact(
    mut frame: pl::DataFrame,
    data_directory: &Path,
    source_name: &str,
) -> Result<DataArtifact, CoreError> {
    let persistence_error = |error: std::io::Error| CoreError::Persistence(error.to_string());
    fs::create_dir_all(data_directory).map_err(persistence_error)?;
    let row_count = frame.height();
    let temporary_path = data_directory.join(format!(".snapshot-{}.tmp", Uuid::new_v4()));
    {
        let file = fs::File::create(&temporary_path).map_err(persistence_error)?;
        pl::ParquetWriter::new(file)
            .with_row_group_size(Some(65_536))
            .finish(&mut frame)
            .map_err(|error| CoreError::Persistence(error.to_string()))?;
    }
    let artifact_id = sha256_file(&temporary_path)?;
    let artifact_path = data_directory.join(format!("{artifact_id}.parquet"));
    if artifact_path.exists() {
        fs::remove_file(&temporary_path).map_err(persistence_error)?;
    } else {
        fs::rename(&temporary_path, &artifact_path).map_err(persistence_error)?;
    }
    Ok(DataArtifact {
        id: artifact_id,
        path: artifact_path.display().to_string(),
        row_count,
        format: ArtifactFormat::Parquet,
        source_name: source_name.to_string(),
    })
}

pub(crate) fn artifact_schema(
    artifact: &DataArtifact,
) -> Result<Vec<(String, DataType)>, CoreError> {
    if artifact.format != ArtifactFormat::Parquet {
        return Err(CoreError::Import("Unsupported artifact format".into()));
    }
    let file =
        fs::File::open(&artifact.path).map_err(|error| CoreError::Import(error.to_string()))?;
    let frame = pl::ParquetReader::new(file)
        .with_slice(Some((0, 0)))
        .finish()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    frame
        .columns()
        .iter()
        .map(|column| {
            framework_type_from_polars(column.dtype())
                .map(|data_type| (column.name().to_string(), data_type))
                .map_err(CoreError::Import)
        })
        .collect()
}

pub(crate) fn write_replacing(path: &Path, contents: &[u8]) -> Result<(), CoreError> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.fw");
    let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

    fs::write(&temporary_path, contents)
        .map_err(|error| CoreError::Persistence(error.to_string()))?;
    match fs::rename(&temporary_path, path) {
        Ok(()) => Ok(()),
        Err(first_error) if path.exists() => {
            // Unix rename replaces an existing file atomically. Windows does
            // not, so fall back to copying over the destination without first
            // deleting the last good snapshot.
            fs::copy(&temporary_path, path).map_err(|error| {
                let _ = fs::remove_file(&temporary_path);
                CoreError::Persistence(format!("{first_error}; {error}"))
            })?;
            fs::remove_file(&temporary_path)
                .map_err(|error| CoreError::Persistence(error.to_string()))
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            Err(CoreError::Persistence(error.to_string()))
        }
    }
}
