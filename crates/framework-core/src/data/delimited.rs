//! CSV text is the source of truth. Types are a conservative interpretation;
//! writing an unrelated cell must not turn an identifier into a number.
use crate::*;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::Path};

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn separator(path: &Path) -> Result<u8, CoreError> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("csv") => Ok(b','),
        Some("tsv") => Ok(b'\t'),
        _ => Err(CoreError::Export(
            "Write-back currently supports CSV and TSV files".into(),
        )),
    }
}

/// Keep each encoded token as well as its decoded value. A small scanner
/// finds boundaries; the CSV reader validates records and performs unescaping.
/// Unsupported encodings and irregular records fail explicitly, never through
/// a lossy fallback. Newlines inside quoted cells are part of their token.
pub(super) struct DelimitedText {
    pub values: Vec<Vec<String>>,
    pub tokens: Vec<Vec<String>>,
    pub endings: Vec<String>,
    pub bom: bool,
}

impl DelimitedText {
    pub fn read(bytes: &[u8], separator: u8) -> Result<Self, CoreError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| CoreError::Import("This file is not UTF-8. Convert its encoding before editing; no bytes were changed.".into()))?;
        let bom = text.starts_with('\u{feff}');
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(separator)
            .from_reader(text.as_bytes());
        let values = reader
            .records()
            .map(|r| r.map(|r| r.iter().map(str::to_owned).collect()))
            .collect::<Result<Vec<Vec<String>>, _>>()
            .map_err(|e| CoreError::Import(e.to_string()))?;
        if values.is_empty() || values[0].is_empty() {
            return Err(CoreError::Import("The file needs a header row".into()));
        }
        let mut tokens = Vec::new();
        let mut endings = Vec::new();
        let mut row = Vec::new();
        let (mut start, mut i, mut quoted) = (0, 0, false);
        let bytes = text.as_bytes();
        while i < bytes.len() {
            match bytes[i] {
                b'"' if quoted && bytes.get(i + 1) == Some(&b'"') => i += 1,
                b'"' => quoted = !quoted,
                b if !quoted && b == separator => {
                    row.push(text[start..i].to_string());
                    start = i + 1;
                }
                b'\r' | b'\n' if !quoted => {
                    row.push(text[start..i].to_string());
                    let end = i;
                    if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    tokens.push(std::mem::take(&mut row));
                    endings.push(text[end..=i].to_string());
                    start = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        if start < text.len() || !row.is_empty() {
            row.push(text[start..].to_string());
            tokens.push(row);
            endings.push(String::new());
        }
        if quoted
            || tokens.len() != values.len()
            || tokens.iter().zip(&values).any(|(a, b)| a.len() != b.len())
        {
            return Err(CoreError::Import("This CSV contains irregular quoting or blank records; normalize those records before editing.".into()));
        }
        Ok(Self {
            values,
            tokens,
            endings,
            bom,
        })
    }
}

fn conservative_type(values: &[Vec<String>], index: usize) -> DataType {
    let nonempty: Vec<&str> = values
        .iter()
        .skip(1)
        .map(|r| r[index].as_str())
        .filter(|v| !v.is_empty())
        .collect();
    if !nonempty.is_empty()
        && nonempty
            .iter()
            .all(|v| super::conservative_csv::is_plain_number(v))
    {
        DataType::Number
    } else {
        DataType::String
    }
}

impl Document {
    pub(crate) fn prepare_open_delimited(
        &self,
        name: String,
        path: String,
        x: f64,
        y: f64,
    ) -> Result<ReplicatedOperation, CoreError> {
        let path = fs::canonicalize(path).map_err(|e| CoreError::Import(e.to_string()))?;
        let bytes = fs::read(&path).map_err(|e| CoreError::Import(e.to_string()))?;
        let source = DelimitedText::read(&bytes, separator(&path)?)?;
        if source.values.len() > MAX_EDITABLE_ROWS + 1 {
            return Err(CoreError::Import(format!(
                "Editable copies hold at most {MAX_EDITABLE_ROWS} rows; this file has {}",
                source.values.len() - 1
            )));
        }
        let mut names = HashSet::new();
        if source.values[0]
            .iter()
            .any(|name| name.is_empty() || !names.insert(name))
        {
            return Err(CoreError::Import(
                "Editable CSVs need nonempty, unique column headers".into(),
            ));
        }
        let columns: Vec<Column> = source.values[0]
            .iter()
            .enumerate()
            .map(|(i, name)| Column {
                id: column_id(name),
                name: name.clone(),
                source_name: Some(name.clone()),
                data_type: conservative_type(&source.values, i),
                categories: vec![],
                format: None,
                formula: None,
            })
            .collect();
        let rows: Vec<Row> = source
            .values
            .iter()
            .skip(1)
            .map(|record| Row {
                id: id(),
                cells: columns
                    .iter()
                    .zip(record)
                    .map(|(c, raw)| {
                        (
                            c.id.clone(),
                            Cell {
                                raw: raw.clone(),
                                ..Cell::default()
                            },
                        )
                    })
                    .collect(),
            })
            .collect();
        let origin = DelimitedFileOrigin {
            path: path.display().to_string(),
            sha256: digest(&bytes),
            row_ids: rows.iter().map(|r| r.id.clone()).collect(),
            column_ids: columns.iter().map(|c| c.id.clone()).collect(),
        };
        let (mut frame, view) =
            Self::frame_with_view(self.unique_frame_name(&name, None), columns, rows, x, y);
        frame.file_origin = Some(origin);
        Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(frame),
            view,
            container_id: None,
        })
    }

    /// Use the same identity-carrying plan as the editable grid, but the data
    /// layer rather than the display layer. Literal spellings survive even a
    /// sorted/filtered chain; only computed cells acquire computed spellings.
    ///
    /// Only an editable file copy can be baked. The generic operation is
    /// reachable from any client, and without this gate it would pull a
    /// linked or derived frame of any size into the document as literal
    /// cells; that is what *Adopt data* is for, and it writes parquet.
    pub(crate) fn bake_frame(&self, frame_id: &str) -> Result<FrameObject, CoreError> {
        let original = self.frame(frame_id)?;
        if original.file_origin.is_none() {
            return Err(CoreError::InvalidOperation(
                "Only a table opened from a CSV/TSV file can be baked. Use Adopt data to take ownership of other tables.".into(),
            ));
        }
        let data = self
            .frame_page_plan(original, Layer::Data)?
            .collect()
            .map_err(|e| CoreError::Export(e.to_string()))?;
        if data.height() > MAX_EDITABLE_ROWS {
            return Err(CoreError::InvalidOperation(format!(
                "The result has {} rows, more than the {MAX_EDITABLE_ROWS} an editable copy can hold",
                data.height()
            )));
        }
        let row_ids = self.page_row_ids(original, &data, 0);
        let stored: std::collections::HashMap<_, _> =
            original.rows.iter().map(|r| (&r.id, r)).collect();
        let mut frame = original.clone();
        frame.rows = (0..data.height())
            .map(|i| {
                let cells = frame
                    .columns
                    .iter()
                    .map(|column| {
                        let literal = original
                            .column_is_editable_input(&column.id)
                            .then(|| {
                                stored
                                    .get(&row_ids[i])
                                    .and_then(|r| r.cells.get(&column.id))
                            })
                            .flatten()
                            .filter(|c| c.override_formula.is_none());
                        let raw = if let Some(cell) = literal {
                            cell.raw.clone()
                        } else {
                            scalar_value_to_raw(
                                polars_value_at(
                                    data.column(&column.id)
                                        .map_err(|e| CoreError::Export(e.to_string()))?
                                        .as_materialized_series(),
                                    i,
                                )
                                .map_err(CoreError::Export)?,
                            )
                        };
                        Ok((
                            column.id.clone(),
                            Cell {
                                raw,
                                ..Cell::default()
                            },
                        ))
                    })
                    .collect::<Result<_, CoreError>>()?;
                Ok(Row {
                    id: if original.preserves_own_row_identity() {
                        row_ids[i].clone()
                    } else {
                        id()
                    },
                    cells,
                })
            })
            .collect::<Result<_, CoreError>>()?;
        for column in &mut frame.columns {
            column.source_name = None;
            column.formula = None;
        }
        frame.steps.clear();
        frame.base_columns.clear();
        frame.derivation = None;
        frame.artifact = None;
        frame.source_file = None;
        frame.connector = None;
        frame.generator = None;
        frame.materialization = None;
        frame.entry_columns.clear();
        Ok(frame)
    }
}

pub(super) fn encode(value: &str, delimiter: u8) -> String {
    if value
        .bytes()
        .any(|b| b == delimiter || matches!(b, b'"' | b'\r' | b'\n'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

impl Document {
    /// A small replacement file remains an editable working copy. The source
    /// reconciliation has already assigned the surviving identities; remap the
    /// newly read cells onto those identities rather than importing a second
    /// frame or losing the write-back destination after the first replacement.
    pub(crate) fn attach_editable_source(&self, frame: &mut FrameObject) -> Result<(), CoreError> {
        let Some(ConnectorRecipe::File { source_path }) = &frame.connector else {
            return Ok(());
        };
        let Ok(ReplicatedOperation::AddObject {
            object: DataObject::Frame(input),
            ..
        }) = self.prepare_open_delimited(frame.name.clone(), source_path.clone(), 0.0, 0.0)
        else {
            return Ok(());
        };
        let bindings: Vec<_> = input
            .columns
            .iter()
            .map(|source| {
                frame
                    .input_columns()
                    .iter()
                    .find(|column| column.source_name == source.source_name)
                    .map(|column| (source.id.clone(), column.id.clone()))
                    .ok_or_else(|| {
                        CoreError::Import(
                            "Source headers changed while replacing the input; try again".into(),
                        )
                    })
            })
            .collect::<Result<_, _>>()?;
        frame.rows = input
            .rows
            .into_iter()
            .map(|mut row| {
                row.cells = bindings
                    .iter()
                    .filter_map(|(old, new)| {
                        row.cells.get(old).cloned().map(|cell| (new.clone(), cell))
                    })
                    .collect();
                row
            })
            .collect();
        let mut origin = input.file_origin.expect("delimited input origin");
        origin.column_ids = bindings.into_iter().map(|(_, new)| new).collect();
        frame.file_origin = Some(origin);
        frame.artifact = None;
        frame.connector = None;
        Ok(())
    }
}
