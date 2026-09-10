//! An external file write and a workbook history entry have different undo
//! semantics. Keep the durable file receipt outside workbook history so Undo
//! can restore a working draft without mistaking our last save for an outside
//! edit. The next explicit write checks that receipt again.
use super::delimited::{DelimitedText, digest, encode, separator};
use crate::engine::plan::ROW_INDEX;
use crate::*;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub struct PreparedDelimitedWrite {
    pub path: PathBuf,
    original: Vec<u8>,
    output: Vec<u8>,
    receipt: DelimitedFileOrigin,
    receipt_path: PathBuf,
    previous_receipt: Option<Vec<u8>>,
}

fn file_error(e: impl std::fmt::Display) -> CoreError {
    CoreError::Export(e.to_string())
}

/// Renders the frame's current data over the file it was read from.
///
/// The row a result came from is named by its ordinal in the base read —
/// `ROW_INDEX`, which the page plan carries out for exactly this reason — so
/// source line `ordinal + 1` is the line this row was parsed from, and there
/// is nothing to store to say so. An ordinal past the end of the file is a
/// row added since, with no line of its own to keep.
///
/// A token is reused when it still spells this cell's value, which is what
/// makes an untouched file come back byte for byte: `12.00` is not the
/// canonical spelling of 12, and neither is `"a,b"` of `a,b`, but both are
/// still the value the plan holds. So the test is the value, not the text —
/// either the source spelling is what we were about to write anyway, or it
/// parses, by the same rules the import read it under, to the number or
/// string we hold. A cell whose value has changed loses its token and is
/// encoded fresh, which is the only place spelling is ever lost.
///
/// A chain that reshapes rows leaves no ordinal to carry, and then no row has
/// a source line: the result is still written, with every cell encoded fresh.
fn render_file(
    document: &Document,
    frame: &FrameObject,
    origin: &DelimitedFileOrigin,
    bytes: &[u8],
) -> Result<(Vec<u8>, Vec<Id>), CoreError> {
    let delimiter = separator(Path::new(&origin.path))?;
    let source = DelimitedText::read(bytes, delimiter)?;
    let cols: HashMap<_, _> = origin
        .column_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let data = document
        .frame_page_plan(frame, Layer::Data)?
        .collect()
        .map_err(|e| CoreError::Export(e.to_string()))?;
    // Only the columns the chain actually produced, the way a page reads
    // them: a Select drops columns from the plan while leaving them in the
    // frame, and asking for one of those is a schema error rather than a
    // column of blanks.
    let written: Vec<&Column> = frame
        .columns
        .iter()
        .filter(|column| data.column(&column.id).is_ok())
        .collect();
    let series: Vec<_> = written
        .iter()
        .map(|column| {
            data.column(&column.id)
                .map(|c| c.as_materialized_series().clone())
                .map_err(|e| CoreError::Export(e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let ordinals: Option<Vec<usize>> = data.column(ROW_INDEX).ok().and_then(|column| {
        column
            .as_materialized_series()
            .u32()
            .ok()
            .map(|values| values.into_no_null_iter().map(|v| v as usize).collect())
    });
    let newline = source
        .endings
        .iter()
        .find(|e| !e.is_empty())
        .map(String::as_str)
        .unwrap_or("\n");
    let final_newline = source.endings.last().is_some_and(|s| !s.is_empty());
    let mut output = if source.bom {
        "\u{feff}".to_string()
    } else {
        String::new()
    };
    for row_index in 0..=data.height() {
        // Row zero is the header, which is why the ordinals are offset by
        // one: the frame's own first row is the file's second line.
        let row = row_index.checked_sub(1);
        let line = match row {
            None => Some(0),
            Some(i) => ordinals
                .as_ref()
                .and_then(|ordinals| ordinals.get(i))
                .map(|ordinal| ordinal + 1),
        };
        for (field, column) in written.iter().enumerate() {
            if field > 0 {
                output.push(delimiter as char);
            }
            let value = match row {
                None => ScalarValue::String(column.name.clone()),
                Some(i) => polars_value_at(&series[field], i).map_err(CoreError::Export)?,
            };
            let raw = scalar_value_to_raw(value.clone());
            let token =
                line.zip(cols.get(column.id.as_str()).copied())
                    .and_then(|(line, field)| {
                        let spelled = source.values.get(line)?.get(field)?;
                        let keeps = *spelled == raw
                            || parse_scalar_value(spelled, column.data_type)
                                .is_ok_and(|parsed| parsed == value);
                        keeps.then(|| source.tokens[line][field].as_str())
                    });
            output.push_str(
                &token
                    .map(str::to_owned)
                    .unwrap_or_else(|| encode(&raw, delimiter)),
            );
        }
        if row_index < data.height() || final_newline {
            output.push_str(
                line.and_then(|line| source.endings.get(line))
                    .filter(|e| !e.is_empty())
                    .map(String::as_str)
                    .unwrap_or(newline),
            );
        }
    }
    let column_ids = written.iter().map(|column| column.id.clone()).collect();
    Ok((output.into_bytes(), column_ids))
}

impl Store {
    /// Prepare and validate the exact history event before touching the file.
    /// The desktop presents the destination and overwrite consequence; this
    /// helper itself never prompts and never writes during preparation.
    pub fn prepare_delimited_write(
        &self,
        frame_id: &str,
        state_directory: &Path,
    ) -> Result<PreparedDelimitedWrite, CoreError> {
        let frame = self.document().frame(frame_id)?;
        let source = frame.file_origin.as_ref().ok_or_else(|| {
            file_error("This table has no editable CSV/TSV source. Open it as stored data first.")
        })?;
        let path = PathBuf::from(&source.path);
        // A rename over a symlink replaces the link, not the file it points
        // at, which would leave the real file untouched and the link dead.
        if fs::symlink_metadata(&path)
            .map_err(file_error)?
            .file_type()
            .is_symlink()
        {
            return Err(file_error(
                "This path is a symbolic link. Open the file it points to and write back there.",
            ));
        }
        let original = fs::read(&path).map_err(file_error)?;
        let hash = digest(&original);
        let receipt_path = state_directory.join(format!("{}.json", digest(source.path.as_bytes())));
        let previous_receipt = match fs::read(&receipt_path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(file_error(e)),
        };
        let receipt = previous_receipt
            .as_deref()
            .map(serde_json::from_slice::<DelimitedFileOrigin>)
            .transpose()
            .map_err(file_error)?;
        let baseline = if source.sha256 == hash {
            source
        } else {
            receipt.as_ref().unwrap_or(source)
        };
        if baseline.path != source.path || baseline.sha256 != hash {
            return Err(file_error(
                "The source file changed outside FrameWork. Reopen it before writing back; your working edits have been kept.",
            ));
        }
        let (output, column_ids) = render_file(self.document(), frame, baseline, &original)?;
        let receipt = DelimitedFileOrigin {
            path: source.path.clone(),
            sha256: digest(&output),
            // The columns as just written, not as they were read: the next
            // write reads its tokens out of the file this one leaves behind.
            column_ids,
        };
        Ok(PreparedDelimitedWrite {
            path,
            original,
            output,
            receipt,
            receipt_path,
            previous_receipt,
        })
    }
}

/// Same-directory temporary plus rename: a failed write never truncates the
/// destination. Preserve permissions, including for the restored original.
fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    atomic_replace_checked(path, bytes, None)
}

fn atomic_replace_checked(
    path: &Path,
    bytes: &[u8],
    expected: Option<&[u8]>,
) -> Result<(), CoreError> {
    let temporary = path.with_file_name(format!(".framework-write-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(file_error)?;
        if let Ok(metadata) = fs::metadata(path) {
            file.set_permissions(metadata.permissions())
                .map_err(file_error)?;
        }
        file.write_all(bytes).map_err(file_error)?;
        file.sync_all().map_err(file_error)?;
        if let Some(expected) = expected
            && fs::read(path).map_err(file_error)? != expected
        {
            return Err(file_error(
                "The source file changed before replacement. Nothing was written.",
            ));
        }
        fs::rename(&temporary, path).map_err(file_error)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

impl PreparedDelimitedWrite {
    /// The caller publishes the validated event only after this succeeds.
    /// A second fingerprint check covers edits made while a preview was open.
    pub fn write(&self) -> Result<(), CoreError> {
        if fs::read(&self.path).map_err(file_error)? != self.original {
            return Err(file_error(
                "The source file changed while preparing write-back. Nothing was written.",
            ));
        }
        let parent = self
            .receipt_path
            .parent()
            .ok_or_else(|| file_error("Missing write-state directory"))?;
        fs::create_dir_all(parent).map_err(file_error)?;
        atomic_replace_checked(&self.path, &self.output, Some(&self.original))?;
        let receipt = serde_json::to_vec(&self.receipt).map_err(file_error)?;
        if let Err(error) = atomic_replace(&self.receipt_path, &receipt) {
            self.rollback()?;
            return Err(error);
        }
        Ok(())
    }

    /// Used if journaling fails after the external write. The previous bytes
    /// stay in memory just long enough for this rollback; Update original does
    /// not leave a recovery copy behind. Do not replace a third party's
    /// intervening write while rolling back.
    pub fn rollback(&self) -> Result<(), CoreError> {
        if fs::read(&self.path).map_err(file_error)? != self.output {
            return Err(file_error(
                "The file changed again before FrameWork could restore the previous contents.",
            ));
        }
        atomic_replace_checked(&self.path, &self.original, Some(&self.output))?;
        if let Some(bytes) = &self.previous_receipt {
            atomic_replace(&self.receipt_path, bytes)?;
        } else if self.receipt_path.exists() {
            fs::remove_file(&self.receipt_path).map_err(file_error)?;
        }
        Ok(())
    }
}
