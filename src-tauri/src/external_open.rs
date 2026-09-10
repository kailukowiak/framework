//! Finder and command-line opens share one dispatch path. Data files become
//! unsaved workbooks: workbook autosave must never write JSON over the input.
use super::*;

fn is_data_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["csv", "tsv", "parquet", "ndjson", "jsonl"]
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
}

fn paths_from_arguments<I>(arguments: I, working_directory: &Path) -> Vec<PathBuf>
where
    I: IntoIterator<Item = OsString>,
{
    arguments
        .into_iter()
        .filter_map(|argument| {
            let path = PathBuf::from(argument);
            if !is_framework_document_path(&path) && !is_data_file(&path) {
                return None;
            }
            Some(if path.is_absolute() {
                path
            } else {
                working_directory.join(path)
            })
        })
        .collect()
}

fn data_session(path: &Path) -> Result<(DocumentSession, DocumentView, Option<String>), String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let mut session = scratch_session()?;
    let writer = Uuid::new_v4().to_string();
    // Rename before importing, while the document is still the empty one
    // `scratch_session` just made. Applying an operation clones the whole
    // document to compute its inverse and then evaluates a fresh view of
    // it; against an empty document both are nearly free. Importing first
    // and renaming after would do that same work against the megabytes of
    // rows the import just landed, for no benefit: nothing on the import
    // path reads the document's name back (the frame name comes from the
    // file stem and is passed separately), so the two operations have no
    // order dependency to respect.
    apply_session_operation_inner(
        &mut session,
        &writer,
        Operation::RenameDocument {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        },
    )?;
    let outcome =
        file_writeback::import_file_into_session(&mut session, &writer, &path, false, 80.0, 80.0)?;
    session.external_source = Some(path);
    flush_session(&mut session)?;
    // `outcome.document` is the `DocumentView` the import's own last applied
    // operation already computed; it reflects the rename above plus the
    // import, so it is already the answer a caller would get back from
    // `session.store.view()`. Hand it up instead of making callers recompute
    // it -- `flush_session` only serializes `store` to disk below, it never
    // mutates it, so this view does not go stale between here and there.
    Ok((session, outcome.document, outcome.notice))
}

fn open_data_window(app: &AppHandle, path: &Path) -> Result<(), String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    // Two Finder opens of one file must not create competing write-back
    // drafts. This identity belongs to the session, independent of renames,
    // transformations, or the notebook's eventual Save As destination.
    let sessions = app
        .state::<AppState>()
        .sessions
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|(label, window)| (label.clone(), Arc::clone(&window.document)))
        .collect::<Vec<_>>();
    for (label, session) in sessions {
        if scratchwork_window::is_scratchwork_label(&label) {
            continue;
        }
        if session
            .lock()
            .map_err(|error| error.to_string())?
            .external_source
            .as_ref()
            == Some(&path)
        {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Ok(());
        }
    }
    // This caller builds a plain window, which reads only the document's
    // name off `store` directly (see `build_document_window`); the
    // evaluated view `data_session` produced has no reader here.
    let (session, _view, notice) = data_session(&path)?;
    let _window = build_document_window(app, session, false)?;
    if let Some(notice) = notice {
        log::info!("{}: {notice}", path.display());
    }
    Ok(())
}

/// Finder delivers the first requested path after Tauri has already created
/// its configured `main` window. During that narrow startup interval the
/// blank session is only a placeholder, not a document the person asked for;
/// replace it instead of leaving an Untitled window beside the requested
/// file. Opens delivered later use `open_data_window` and remain separate, so
/// an actual working scratch canvas is never silently replaced.
fn open_data_in_startup_window(app: &AppHandle, path: &Path) -> Result<(), String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    // `data_session` already evaluated this view as the last step of the
    // import; re-deriving it with a second `store.view()` would repeat that
    // whole evaluation over the document this call just populated.
    let (session, document, notice) = data_session(&path)?;
    app.state::<AppState>().replace_document("main", session)?;
    scratchwork_window::set_workbook_titles(app, "main", &document.document.name);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        sync_history_menu(&window, document.can_undo, document.can_redo);
    }
    let payload = OpenedDocument {
        document,
        // A CSV opens into an unsaved workbook. An empty path gives the
        // event-driven frontend the same answer `get_document_path` returns.
        path: String::new(),
    };
    let _ = app.emit_to("main", DOCUMENT_OPENED_EVENT, &payload);
    if let Some(notice) = notice {
        log::info!("{}: {notice}", path.display());
    }
    Ok(())
}

fn main_started_blank(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .sessions
        .lock()
        .ok()
        .and_then(|sessions| sessions.get("main").map(|session| session.started_blank))
        .unwrap_or(false)
}

pub(super) fn try_open_from_arguments<I>(app: &AppHandle, arguments: I, working_directory: &Path)
where
    I: IntoIterator<Item = OsString>,
{
    for path in paths_from_arguments(arguments, working_directory) {
        let result = if is_data_file(&path) {
            open_data_window(app, &path)
        } else {
            open_document_window(app, path)
        };
        if let Err(error) = result {
            log::error!("external open failed: {error}");
            let _ = app.emit(DOCUMENT_OPEN_FAILED_EVENT, error);
        }
    }
}

/// Dispatches only the paths queued before `RunEvent::Ready`. The first one
/// owns the placeholder window created by Tauri; every additional path is a
/// genuine additional document and follows the ordinary new-window path.
pub(super) fn try_open_startup_arguments<I>(app: &AppHandle, arguments: I, working_directory: &Path)
where
    I: IntoIterator<Item = OsString>,
{
    for path in paths_from_arguments(arguments, working_directory) {
        let result = if main_started_blank(app) {
            if is_data_file(&path) {
                open_data_in_startup_window(app, &path)
            } else {
                open_document_at(app, "main", path, true, false).map(|_| ())
            }
        } else if is_data_file(&path) {
            open_data_window(app, &path)
        } else {
            open_document_window(app, path)
        };
        if let Err(error) = result {
            log::error!("external open failed: {error}");
            let _ = app.emit(DOCUMENT_OPEN_FAILED_EVENT, error);
        }
    }
}

/// An explicit path chooses the content. An unrequested launch stays blank:
/// reusing yesterday's scratch document or reopening the last saved workbook
/// would let a hot-reload restart put subsequent edits into the wrong file.
pub(super) fn initial_session() -> Result<(DocumentSession, bool, Option<String>), String> {
    let working_directory = env::current_dir().map_err(|error| error.to_string())?;
    if let Some(path) = paths_from_arguments(env::args_os(), &working_directory)
        .into_iter()
        .next()
    {
        if is_data_file(&path) {
            let (session, _view, notice) = data_session(&path)?;
            return Ok((session, false, notice));
        }
        if path.exists() {
            let (session, warning) = load_session(path)?;
            return Ok((session, false, warning));
        }
        return Ok((blank_session(path, false)?, false, None));
    }
    Ok((scratch_session()?, true, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_documents_and_data_including_multiple_paths() {
        let cwd = Path::new("/shared/projects");
        let paths = paths_from_arguments(
            [
                "framework",
                "--verbose",
                "Orders.FW",
                "August.CSV",
                "data.tsv",
                "data.parquet",
                "notes.txt",
            ]
            .map(OsString::from),
            cwd,
        );
        assert_eq!(
            paths,
            ["Orders.FW", "August.CSV", "data.tsv", "data.parquet"].map(|name| cwd.join(name))
        );
    }

    #[test]
    fn csv_open_keeps_source_bytes_and_creates_an_editable_scratch_workbook() {
        let directory = env::temp_dir().join(format!("framework-open-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("Orders.CSV");
        let bytes = "ID,Quantity\n0002,3\n";
        fs::write(&path, bytes).unwrap();
        let (session, view, notice) = data_session(&path).unwrap();
        assert!(notice.is_none());
        assert!(session.scratch);
        assert_ne!(session.path, path);
        assert_eq!(session.external_source, Some(path.canonicalize().unwrap()));
        assert_eq!(view.document.name, "Orders.CSV");
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| {
                if let DataObject::Frame(frame) = object {
                    Some(frame)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(frame.file_origin.is_some());
        assert!(frame.owns_its_rows());
        assert_eq!(fs::read_to_string(&path).unwrap(), bytes);
        assert!(Store::load(&session.path).is_ok());
        fs::remove_dir_all(session.path.parent().unwrap()).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn large_csv_open_uses_the_same_paged_fallback_as_import() {
        let directory = env::temp_dir().join(format!("framework-large-open-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("large.csv");
        let mut csv = String::from("ID,Quantity\n");
        for index in 0..20_001 {
            csv.push_str(&format!("{index:07},3\n"));
        }
        fs::write(&path, &csv).unwrap();
        let (mut session, view, notice) = data_session(&path).unwrap();
        assert!(notice.is_none());
        let frame = view
            .document
            .objects
            .iter()
            .find_map(|object| {
                if let DataObject::Frame(frame) = object {
                    Some(frame)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(frame.artifact.is_some());
        assert!(frame.file_origin.is_none());
        let editing = &view.computed_frames[&frame.id].editing;
        assert!(!editing.cells);
        let reason = editing.reason.as_deref().unwrap_or_default();
        assert!(reason.contains("paged"), "{reason}");
        assert!(reason.contains("calculated column"), "{reason}");
        assert!(reason.contains("Wrangle"), "{reason}");
        let frame_id = frame.id.clone();
        session
            .store
            .apply(Operation::SetFramePipeline {
                frame_id,
                steps: vec![framework_core::FrameStepInput::WithColumns {
                    columns: vec![framework_core::ExistingFormulaInput {
                        output_column_id: "double-quantity~test".into(),
                        name: "Double quantity".into(),
                        formula: "`Quantity` * 2".into(),
                    }],
                }],
            })
            .expect("paged frames still accept calculated columns");
        assert_eq!(fs::read_to_string(&path).unwrap(), csv);
        fs::remove_dir_all(session.path.parent().unwrap()).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
