use crate::Id;
use crate::VersionVector;
use crate::error::CoreError;
use crate::operation::event::{OPERATION_EVENT_VERSION, OperationEvent};
use crate::persist::FRAMEWORK_FILE_EXTENSION;
use crate::store::Store;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Cross-platform paths that accompany a clickable `.fw` document file.
///
/// The `.fw` file remains an ordinary file that Windows, macOS, and Linux can
/// associate with FrameWork. Collaboration data lives beside it under a
/// document-ID directory so renaming the `.fw` file does not disconnect its
/// operation history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationPaths {
    pub root: PathBuf,
    pub events: PathBuf,
    pub checkpoints: PathBuf,
    pub sessions: PathBuf,
}

impl CollaborationPaths {
    pub fn for_document(document_path: &Path, document_id: &str) -> Result<Self, CoreError> {
        let document_id = Uuid::parse_str(document_id)
            .map_err(|_| CoreError::Persistence("Document ID is not a valid UUID".into()))?
            .to_string();
        let parent = document_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let root = parent.join(".framework").join(document_id);
        Ok(Self {
            events: root.join("events"),
            checkpoints: root.join("checkpoints"),
            sessions: root.join("sessions"),
            root,
        })
    }

    pub fn ensure_exists(&self) -> Result<(), CoreError> {
        for path in [&self.events, &self.checkpoints, &self.sessions] {
            fs::create_dir_all(path).map_err(|error| CoreError::Persistence(error.to_string()))?;
        }
        Ok(())
    }
}

/// Immutable, sync-friendly operation files grouped by writer identity.
///
/// A writer owns monotonically numbered files beneath
/// `events/<writer-id>/`. Files are never edited after publication, which
/// keeps cloud-drive synchronization away from a shared append target.
#[derive(Debug, Clone)]
pub struct EventJournal {
    document_id: Id,
    paths: CollaborationPaths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeResult {
    pub applied: usize,
    pub pending: usize,
}

impl EventJournal {
    pub fn open(document_path: &Path, document_id: &str) -> Result<Self, CoreError> {
        let paths = CollaborationPaths::for_document(document_path, document_id)?;
        paths.ensure_exists()?;
        Ok(Self {
            document_id: document_id.into(),
            paths,
        })
    }

    pub fn paths(&self) -> &CollaborationPaths {
        &self.paths
    }

    pub fn append(&self, event: &OperationEvent) -> Result<PathBuf, CoreError> {
        validate_event_envelope(event, &self.document_id)?;
        let writer_directory = self.paths.events.join(&event.event_id.writer_id);
        fs::create_dir_all(&writer_directory)
            .map_err(|error| CoreError::Persistence(error.to_string()))?;
        let destination = writer_directory.join(format!("{:020}.json", event.event_id.sequence));
        let mut contents = serde_json::to_vec_pretty(event)
            .map_err(|error| CoreError::Persistence(error.to_string()))?;
        contents.push(b'\n');

        if destination.exists() {
            return ensure_existing_event_matches(&destination, &contents);
        }

        let temporary = writer_directory.join(format!(
            ".{:020}.{}.tmp",
            event.event_id.sequence,
            Uuid::new_v4()
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| CoreError::Persistence(error.to_string()))?;
            file.write_all(&contents)
                .and_then(|_| file.sync_all())
                .map_err(|error| CoreError::Persistence(error.to_string()))?;

            match fs::hard_link(&temporary, &destination) {
                Ok(()) => Ok(destination.clone()),
                Err(_) if destination.exists() => {
                    ensure_existing_event_matches(&destination, &contents)
                }
                Err(link_error) => match fs::rename(&temporary, &destination) {
                    Ok(()) => Ok(destination.clone()),
                    Err(_) if destination.exists() => {
                        ensure_existing_event_matches(&destination, &contents)
                    }
                    Err(rename_error) => Err(CoreError::Persistence(format!(
                        "{link_error}; {rename_error}"
                    ))),
                },
            }
        })();
        let _ = fs::remove_file(&temporary);
        result
    }

    pub fn read_all(&self) -> Result<Vec<OperationEvent>, CoreError> {
        self.read_after(&VersionVector::new())
    }

    /// Every event a replica at `applied` has not seen yet.
    ///
    /// The writer and sequence live in the path, so an event already covered
    /// by `applied` is skipped without reading it. That is not just an
    /// optimization: it is what keeps a journal readable after an operation
    /// is renamed or removed. Those events are, by definition, already in the
    /// snapshot -- replaying them was never going to happen -- so failing to
    /// parse one would reject a log that has nothing left to contribute.
    /// An event that genuinely still needs applying is parsed, and still
    /// errors if this build cannot understand it.
    pub fn read_after(&self, applied: &VersionVector) -> Result<Vec<OperationEvent>, CoreError> {
        self.paths.ensure_exists()?;
        let mut events = Vec::new();
        let writers =
            fs::read_dir(&self.paths.events).map_err(|error| CoreError::Load(error.to_string()))?;
        for writer in writers {
            let writer = writer.map_err(|error| CoreError::Load(error.to_string()))?;
            let file_type = writer
                .file_type()
                .map_err(|error| CoreError::Load(error.to_string()))?;
            if !file_type.is_dir() {
                continue;
            }
            let writer_id = writer.file_name().to_string_lossy().into_owned();
            if Uuid::parse_str(&writer_id).is_err() {
                continue;
            }
            for entry in
                fs::read_dir(writer.path()).map_err(|error| CoreError::Load(error.to_string()))?
            {
                let entry = entry.map_err(|error| CoreError::Load(error.to_string()))?;
                if !entry
                    .file_type()
                    .map_err(|error| CoreError::Load(error.to_string()))?
                    .is_file()
                    || entry.path().extension().and_then(|value| value.to_str()) != Some("json")
                {
                    continue;
                }
                let sequence = entry
                    .path()
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .and_then(|value| value.parse::<u64>().ok())
                    .ok_or_else(|| {
                        CoreError::InvalidEvent(format!(
                            "invalid event filename {}",
                            entry.path().display()
                        ))
                    })?;
                if applied.get(&writer_id).copied().unwrap_or_default() >= sequence {
                    continue;
                }
                let contents =
                    fs::read(entry.path()).map_err(|error| CoreError::Load(error.to_string()))?;
                let event: OperationEvent = serde_json::from_slice(&contents)
                    .map_err(|error| CoreError::InvalidEvent(error.to_string()))?;
                validate_event_envelope(&event, &self.document_id)?;
                if event.event_id.writer_id != writer_id || event.event_id.sequence != sequence {
                    return Err(CoreError::InvalidEvent(format!(
                        "event identity does not match {}",
                        entry.path().display()
                    )));
                }
                events.push(event);
            }
        }
        events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
        Ok(events)
    }

    pub fn merge_into(&self, store: &mut Store) -> Result<MergeResult, CoreError> {
        let before_merge = store.clone();
        let mut remaining = self.read_after(store.version_vector())?;
        let mut applied = 0;
        loop {
            let before = remaining.len();
            let mut pending = Vec::new();
            for event in remaining {
                let current = store
                    .version_vector()
                    .get(&event.event_id.writer_id)
                    .copied()
                    .unwrap_or_default();
                if event.event_id.sequence <= current {
                    continue;
                }
                let dependencies_ready = event.dependencies.iter().all(|(writer_id, sequence)| {
                    store
                        .version_vector()
                        .get(writer_id)
                        .copied()
                        .unwrap_or_default()
                        >= *sequence
                });
                if event.event_id.sequence == current + 1 && dependencies_ready {
                    if let Err(error) = store.apply_imported_event(&event) {
                        *store = before_merge;
                        return Err(error);
                    }
                    applied += 1;
                } else {
                    pending.push(event);
                }
            }
            if pending.is_empty() || pending.len() == before {
                return Ok(MergeResult {
                    applied,
                    pending: pending.len(),
                });
            }
            remaining = pending;
        }
    }
}

fn validate_event_envelope(event: &OperationEvent, document_id: &str) -> Result<(), CoreError> {
    if event.format_version != OPERATION_EVENT_VERSION {
        return Err(CoreError::InvalidEvent(format!(
            "unsupported event version {}",
            event.format_version
        )));
    }
    if event.document_id != document_id {
        return Err(CoreError::InvalidEvent(
            "event belongs to a different document".into(),
        ));
    }
    Uuid::parse_str(&event.event_id.writer_id)
        .map_err(|_| CoreError::InvalidEvent("writer ID is not a valid UUID".into()))?;
    if event.event_id.sequence == 0 {
        return Err(CoreError::InvalidEvent(
            "writer sequence must start at one".into(),
        ));
    }
    if event
        .dependencies
        .get(&event.event_id.writer_id)
        .copied()
        .unwrap_or_default()
        != event.event_id.sequence - 1
    {
        return Err(CoreError::InvalidEvent(
            "event dependencies do not match its writer sequence".into(),
        ));
    }
    Ok(())
}

fn ensure_existing_event_matches(path: &Path, expected: &[u8]) -> Result<PathBuf, CoreError> {
    let existing = fs::read(path).map_err(|error| CoreError::Persistence(error.to_string()))?;
    if existing == expected {
        Ok(path.to_path_buf())
    } else {
        Err(CoreError::Persistence(format!(
            "refusing to overwrite immutable event {}",
            path.display()
        )))
    }
}

pub fn is_framework_document_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(FRAMEWORK_FILE_EXTENSION))
}
