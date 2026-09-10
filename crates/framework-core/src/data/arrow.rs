use crate::error::CoreError;
use crate::model::data_artifact::{ArtifactFormat, DataArtifact};
use polars::prelude as pl;
use polars::prelude::{LazyFileListReader, SerReader};
use sha2::{Digest, Sha256};
use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub fn create_data_artifact(
    source_path: &Path,
    data_directory: &Path,
) -> Result<DataArtifact, CoreError> {
    if !source_path.is_file() {
        return Err(CoreError::Import(format!(
            "Import file does not exist: {}",
            source_path.display()
        )));
    }
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !matches!(
        extension.as_str(),
        "csv" | "tsv" | "parquet" | "ndjson" | "jsonl"
    ) {
        return Err(CoreError::Import(
            "Only .csv, .tsv, .parquet, and .ndjson files can be imported".into(),
        ));
    }
    fs::create_dir_all(data_directory).map_err(|error| CoreError::Import(error.to_string()))?;
    let temporary_path = data_directory.join(format!(".artifact-{}.tmp", Uuid::new_v4()));
    match extension.as_str() {
        "parquet" => {
            fs::copy(source_path, &temporary_path)
                .map_err(|error| CoreError::Import(error.to_string()))?;
        }
        "ndjson" | "jsonl" => normalize_json_lines_artifact(source_path, &temporary_path)?,
        _ => normalize_delimited_artifact(source_path, &temporary_path, extension == "tsv")?,
    }

    let artifact_id = sha256_file(&temporary_path)?;
    let artifact_path = data_directory.join(format!("{artifact_id}.parquet"));
    if artifact_path.exists() {
        fs::remove_file(&temporary_path).map_err(|error| CoreError::Import(error.to_string()))?;
    } else {
        fs::rename(&temporary_path, &artifact_path)
            .map_err(|error| CoreError::Import(error.to_string()))?;
    }
    let file =
        fs::File::open(&artifact_path).map_err(|error| CoreError::Import(error.to_string()))?;
    let row_count = pl::ParquetReader::new(file)
        .num_rows()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    Ok(DataArtifact {
        id: artifact_id,
        path: artifact_path.display().to_string(),
        row_count,
        format: ArtifactFormat::Parquet,
        source_name: source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported data")
            .to_string(),
    })
}

/// Read every line before choosing the schema, the same promise the delimited
/// path makes: a field that is null for the first thousand records and a string
/// after that must not decide the column's type from the first page alone. JSON
/// states each value's own type, so there are no inference *rules* here beyond
/// that — only the requirement to look at all of it.
fn normalize_json_lines_artifact(source_path: &Path, destination: &Path) -> Result<(), CoreError> {
    let source = source_path
        .to_str()
        .ok_or_else(|| CoreError::Import("Import path is not valid UTF-8".into()))?;
    let mut scan = pl::LazyJsonLineReader::new(pl::PlRefPath::new(source))
        .with_infer_schema_length(None)
        .finish()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    // Refuse a nested record here, before anything reaches the canvas. Polars
    // will happily infer a struct or list column and write it to the artifact,
    // and the frame will even import — but the grid cannot render such a cell,
    // so the table would land looking fine and fail on its first page read.
    // A grid holds one value per cell; saying so now beats a broken table.
    let schema = scan
        .collect_schema()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    for (name, data_type) in schema.iter() {
        if crate::framework_type_from_polars(data_type).is_err() {
            return Err(CoreError::Import(format!(
                "The field `{name}` holds nested JSON ({data_type}). FrameWork reads one flat \
                 record per line; flatten the nested fields before importing this file."
            )));
        }
    }
    sink_artifact(scan, destination)
}

fn normalize_delimited_artifact(
    source_path: &Path,
    destination: &Path,
    tab_separated: bool,
) -> Result<(), CoreError> {
    let source = source_path
        .to_str()
        .ok_or_else(|| CoreError::Import("Import path is not valid UTF-8".into()))?;
    let scan = pl::LazyCsvReader::new(pl::PlRefPath::new(source))
        .with_has_header(true)
        .with_separator(if tab_separated { b'\t' } else { b',' })
        .with_dtype_overwrite_by_position(Some(Arc::new(super::conservative_csv::types(
            source_path,
            if tab_separated { b'\t' } else { b',' },
        )?)))
        .finish()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    sink_artifact(scan, destination)
}

/// Stream a scan into the document's parquet artifact in bounded memory. Every
/// import format arrives through here, so a large file costs the same whichever
/// one it was.
fn sink_artifact(mut scan: pl::LazyFrame, destination: &Path) -> Result<(), CoreError> {
    let schema = scan
        .collect_schema()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    let output =
        fs::File::create(destination).map_err(|error| CoreError::Import(error.to_string()))?;
    let writer = pl::ParquetWriter::new(output)
        .with_row_group_size(Some(65_536))
        .batched(schema.as_ref())
        .map_err(|error| CoreError::Import(error.to_string()))?;
    let writer = Arc::new(Mutex::new(writer));
    let callback_writer = Arc::clone(&writer);
    let sink = scan
        .sink_batches(
            pl::PlanCallback::new(move |mut batch: pl::DataFrame| {
                batch.rechunk_mut_par();
                callback_writer
                    .lock()
                    .map_err(|_| {
                        pl::PolarsError::ComputeError("Artifact writer lock failed".into())
                    })?
                    .write_batch(&batch)?;
                Ok(false)
            }),
            true,
            NonZeroUsize::new(65_536),
        )
        .map_err(|error| CoreError::Import(error.to_string()))?;
    sink.collect_with_engine(pl::Engine::Streaming)
        .map_err(|error| CoreError::Import(error.to_string()))?;
    writer
        .lock()
        .map_err(|_| CoreError::Import("Artifact writer lock failed".into()))?
        .finish()
        .map_err(|error| CoreError::Import(error.to_string()))?;
    Ok(())
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, CoreError> {
    let mut file = fs::File::open(path).map_err(|error| CoreError::Import(error.to_string()))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| CoreError::Import(error.to_string()))?;
    Ok(format!("{:x}", hasher.finalize()))
}
