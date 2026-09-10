//! Inference must not erase an identifier before the person can choose its
//! type. Read text first and inspect every loaded value, including values
//! beyond Polars' inference sample. Explicit conversions remain Wrangle work.
use crate::CoreError;
use polars::prelude as pl;
use polars::prelude::SerReader;
use std::path::Path;

pub(super) fn is_plain_number(value: &str) -> bool {
    let digits = value.trim_start_matches(['-', '+']);
    !value.is_empty()
        && value.trim() == value
        && !(digits.starts_with('0') && digits.as_bytes().get(1).is_some_and(u8::is_ascii_digit))
        && value.bytes().filter(u8::is_ascii_digit).count() <= 15
        && value.parse::<f64>().is_ok_and(f64::is_finite)
}

pub(super) fn read(path: &Path, limit: Option<usize>) -> Result<pl::DataFrame, CoreError> {
    let delimiter = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("tsv"))
    {
        b'\t'
    } else {
        b','
    };
    let mut frame = pl::CsvReadOptions::default()
        .with_has_header(true)
        .with_n_rows(limit)
        .with_infer_schema_length(Some(0))
        .with_parse_options(pl::CsvParseOptions::default().with_separator(delimiter))
        .try_into_reader_with_file_path(Some(path.to_path_buf()))
        .and_then(|reader| reader.finish())
        .map_err(|e| CoreError::Import(e.to_string()))?;
    let columns = frame.columns().to_vec();
    for column in columns {
        let values = column.str().map_err(|e| CoreError::Import(e.to_string()))?;
        let mut any = false;
        let numeric = values.iter().flatten().filter(|s| !s.is_empty()).all(|s| {
            any = true;
            is_plain_number(s)
        });
        if numeric && any {
            let integer = values.iter().flatten().all(|s| s.parse::<i64>().is_ok());
            let dtype = if integer {
                pl::DataType::Int64
            } else {
                pl::DataType::Float64
            };
            let converted = column
                .cast(&dtype)
                .map_err(|e| CoreError::Import(e.to_string()))?;
            frame
                .with_column(converted)
                .map_err(|e| CoreError::Import(e.to_string()))?;
        }
    }
    Ok(frame)
}

/// One bounded-memory pass decides the schema for a streamed import. Sampling
/// cannot promise identifier preservation: a leading zero may occur on the
/// final row. Types are supplied by position so Polars can still disambiguate
/// duplicate headers without collapsing two physical fields into one.
pub(super) fn types(path: &Path, delimiter: u8) -> Result<Vec<pl::DataType>, CoreError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_path(path)
        .map_err(|e| CoreError::Import(e.to_string()))?;
    let headers = reader
        .headers()
        .map_err(|e| CoreError::Import(e.to_string()))?
        .clone();
    let mut numeric = vec![true; headers.len()];
    let mut integer = numeric.clone();
    let mut any = vec![false; headers.len()];
    for record in reader.records() {
        let record = record.map_err(|e| CoreError::Import(e.to_string()))?;
        for (index, value) in record.iter().enumerate() {
            if !value.is_empty() {
                any[index] = true;
                numeric[index] &= is_plain_number(value);
                integer[index] &= value.parse::<i64>().is_ok();
            }
        }
    }
    Ok(headers
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if !any[index] || !numeric[index] {
                pl::DataType::String
            } else if integer[index] {
                pl::DataType::Int64
            } else {
                pl::DataType::Float64
            }
        })
        .collect())
}
