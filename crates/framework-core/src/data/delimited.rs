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

impl Document {
    /// The record an update needs: which file, as it stood, and which column
    /// of this frame each of its physical fields is.
    ///
    /// Resolved without reading the file's rows. The artifact beside it is
    /// already the whole content, read once and streamed to parquet; a second
    /// full pass here would only be to say early what write-back has to check
    /// again anyway, since the file may change between opening it and putting
    /// something back. So the checks here are the ones that are both cheap
    /// and decisive — that the format has a delimiter FrameWork can write,
    /// and that the header names each field once — and the structural
    /// fidelity check stays where the bytes are actually about to be
    /// replaced, in [`Store::prepare_delimited_write`], which refuses before
    /// touching the file.
    ///
    /// The hash is streamed rather than taken over a buffer, because an
    /// opened file is no longer bounded by what a document can hold.
    pub(crate) fn delimited_file_origin(
        path: &str,
        columns: &[Column],
    ) -> Result<DelimitedFileOrigin, CoreError> {
        let path = fs::canonicalize(path).map_err(|e| CoreError::Import(e.to_string()))?;
        let delimiter = separator(&path)?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .from_path(&path)
            .map_err(|e| CoreError::Import(e.to_string()))?;
        let headers = reader
            .headers()
            .map_err(|e| CoreError::Import(e.to_string()))?
            .clone();
        let mut seen = HashSet::new();
        if headers.is_empty()
            || headers
                .iter()
                .any(|name| name.trim().is_empty() || !seen.insert(name))
        {
            return Err(CoreError::Import(
                "A file FrameWork can write back to needs nonempty, unique column headers".into(),
            ));
        }
        // Positional, because that is the only thing a physical field and a
        // document column reliably share: the import mints one column per
        // field in the file's own order, and a header rename afterwards must
        // not lose the tokens sitting under it.
        if headers.len() != columns.len() {
            return Err(CoreError::Import(
                "This file's header does not match the data that was read from it".into(),
            ));
        }
        Ok(DelimitedFileOrigin {
            path: path.display().to_string(),
            sha256: sha256_file(&path)?,
            column_ids: columns.iter().map(|column| column.id.clone()).collect(),
        })
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
    /// Keeps the write-back destination pointing at whatever file the frame
    /// now reads.
    ///
    /// Replacing the source has already swapped in the new artifact and the
    /// new connector; all that is left is the record an update needs, which
    /// is about the *file* rather than about the data — which physical field
    /// is which column, and what the file hashed to. It is re-derived rather
    /// than carried over because the columns are the reconciled ones and the
    /// file is a different file.
    ///
    /// The connector goes with it. A frame that can be written back to is one
    /// somebody is fixing, and a connector is the standing permission to
    /// replace what they are fixing with a fresh read — including the
    /// corrections, which are keyed by ordinal and cannot survive a new base.
    /// A file that should refresh instead of being written back keeps its
    /// connector and gets no origin, which is what a linked import already is.
    ///
    /// A replacement that is not a CSV or TSV, or whose header no longer
    /// matches, simply leaves the frame without a destination: it reads the
    /// new file, and *Update original* is not offered for it.
    pub(crate) fn attach_editable_source(&self, frame: &mut FrameObject) -> Result<(), CoreError> {
        let Some(ConnectorRecipe::File { source_path }) = frame.connector.clone() else {
            return Ok(());
        };
        frame.file_origin = Self::delimited_file_origin(&source_path, frame.input_columns()).ok();
        if frame.file_origin.is_some() {
            frame.connector = None;
        }
        Ok(())
    }
}
