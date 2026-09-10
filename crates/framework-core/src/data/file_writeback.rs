//! An external file write and a workbook history entry have different undo
//! semantics. Keep the durable file receipt outside workbook history so Undo
//! can restore a working draft without mistaking our last save for an outside
//! edit. The next explicit write checks that receipt again.
use super::delimited::{DelimitedText, digest, encode, separator};
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

/// Reuse a token only when its decoded value is still the same cell's value.
/// This retains quoting and numeric spelling without letting an old row
/// position decide where a correction belongs.
fn render_file(
    frame: &FrameObject,
    origin: &DelimitedFileOrigin,
    bytes: &[u8],
) -> Result<Vec<u8>, CoreError> {
    let delimiter = separator(Path::new(&origin.path))?;
    let source = DelimitedText::read(bytes, delimiter)?;
    let cols: HashMap<_, _> = origin
        .column_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let rows: HashMap<_, _> = origin
        .row_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i + 1))
        .collect();
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
    for row_index in 0..=frame.rows.len() {
        let row = row_index.checked_sub(1).map(|i| &frame.rows[i]);
        let old_row = row.map_or(Some(0), |r| rows.get(&r.id).copied());
        for (i, column) in frame.columns.iter().enumerate() {
            if i > 0 {
                output.push(delimiter as char);
            }
            let value = row.map_or(column.name.as_str(), |r| {
                r.cells
                    .get(&column.id)
                    .map(|c| c.raw.as_str())
                    .unwrap_or("")
            });
            let token = old_row
                .zip(cols.get(&column.id).copied())
                .and_then(|(r, c)| {
                    (source.values.get(r)?.get(c)? == value).then(|| source.tokens[r][c].as_str())
                });
            output.push_str(
                &token
                    .map(str::to_owned)
                    .unwrap_or_else(|| encode(value, delimiter)),
            );
        }
        if row_index < frame.rows.len() || final_newline {
            output.push_str(
                old_row
                    .and_then(|r| source.endings.get(r))
                    .filter(|e| !e.is_empty())
                    .map(String::as_str)
                    .unwrap_or(newline),
            );
        }
    }
    Ok(output.into_bytes())
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
        let source = self
            .document()
            .frame(frame_id)?
            .file_origin
            .as_ref()
            .ok_or_else(|| {
                file_error(
                    "This table has no editable CSV/TSV source. Open it as stored data first.",
                )
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
        let ReplicatedOperation::RestoreFrame { frame } =
            self.prepare_operation(Operation::BakeFrame {
                frame_id: frame_id.into(),
            })?
        else {
            unreachable!()
        };
        let output = render_file(&frame, baseline, &original)?;
        let receipt = DelimitedFileOrigin {
            path: source.path.clone(),
            sha256: digest(&output),
            row_ids: frame.rows.iter().map(|r| r.id.clone()).collect(),
            column_ids: frame.columns.iter().map(|c| c.id.clone()).collect(),
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
