//! Explicit file editing is separate from workbook autosave and live imports.
use super::*;

/// What an import produced, and anything the person should hear about how.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImportOutcome {
    pub document: DocumentView,
    pub notice: Option<String>,
}

#[tauri::command]
pub(super) fn import_dataset_file(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    path: Option<String>,
    linked: bool,
    state: State<'_, AppState>,
) -> Result<Option<ImportOutcome>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Data files", &["csv", "tsv", "parquet", "ndjson", "jsonl"])
                .pick_file()
            else {
                return Ok(None);
            };
            path
        }
    };
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let outcome = import_file_into_session(&mut session, &state.writer_id, &path, linked, x, y)?;
    sync_history_menu(
        &window,
        outcome.document.can_undo,
        outcome.document.can_redo,
    );
    scratchwork_window::emit_document_to_peers(&window, &outcome.document);
    Ok(Some(outcome))
}

/// Shared by the import gesture and operating-system file opening, so both
/// retain the same editable origin and paged fallback.
pub(super) fn import_file_into_session(
    session: &mut DocumentSession,
    writer_id: &str,
    path: &Path,
    linked: bool,
    x: f64,
    y: f64,
) -> Result<ImportOutcome, String> {
    ensure_live(session)?;
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Imported data")
        .to_string();
    let delimited = path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("csv") || s.eq_ignore_ascii_case("tsv"));
    // A stored CSV/TSV holds literal rows when it is small and regular enough
    // to do so. Otherwise it imports the paged way it always has. That is not
    // an import warning: explain the narrower direct-cell boundary only if
    // somebody actually tries that gesture.
    let notice = None;
    let mut paged_source = false;
    if !linked && delimited {
        match apply_session_operation_inner(
            session,
            writer_id,
            Operation::OpenDelimitedFile {
                name: name.clone(),
                path: path.display().to_string(),
                x,
                y,
            },
        ) {
            Ok(document) => {
                return Ok(ImportOutcome {
                    document,
                    notice: None,
                });
            }
            Err(reason) => {
                log::info!("using paged import for {}: {reason}", path.display());
                paged_source = true;
            }
        }
    }
    let artifact = stage_import_file(&session.path, session.store.document_id(), path)?;
    let operation = Operation::ImportFrameFromArtifact {
        name,
        artifact,
        connector: (linked || paged_source).then(|| ConnectorRecipe::File {
            source_path: path.display().to_string(),
        }),
        x,
        y,
    };
    let document = apply_session_operation_inner(session, writer_id, operation)?;
    Ok(ImportOutcome { document, notice })
}

/// The file a frame was read from, whatever records it. Used to place the
/// export dialog and to decide which format it should offer first.
fn frame_source_path(frame: &FrameObject) -> Option<PathBuf> {
    frame
        .file_origin
        .as_ref()
        .map(|origin| origin.path.clone())
        .or_else(|| frame.source_file.clone())
        .or_else(|| match &frame.connector {
            Some(ConnectorRecipe::File { source_path }) => Some(source_path.clone()),
            _ => None,
        })
        .map(PathBuf::from)
}

/// A table read from Parquet should be offered Parquet first, and the same for
/// delimited text: the format somebody already has is the likeliest one they
/// want more of. Anything FrameWork cannot write — a database query, entered
/// data — falls back to CSV.
fn export_extension(frame: &FrameObject) -> &'static str {
    frame_source_path(frame)
        .as_deref()
        .and_then(Path::extension)
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        // Both spellings of JSON lines read back as the one the chooser offers.
        .map(|extension| {
            if extension == "jsonl" {
                "ndjson".to_string()
            } else {
                extension
            }
        })
        .and_then(|extension| {
            EXPORT_FILE_EXTENSIONS
                .into_iter()
                .find(|candidate| *candidate == extension)
        })
        .unwrap_or("csv")
}

fn export_format_label(extension: &str) -> &'static str {
    match extension {
        "tsv" => "Tab-separated values",
        "parquet" => "Parquet",
        "ndjson" => "JSON lines",
        _ => "CSV",
    }
}

#[tauri::command]
pub(super) fn export_frame_as(
    window: tauri::WebviewWindow,
    frame_id: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<DocumentView>, String> {
    let (suggested, preferred, directory, x, y) = {
        let session = state.document_for(window.label())?;
        let session = session.lock().map_err(|e| e.to_string())?;
        ensure_live(&session)?;
        let frame = session
            .store
            .document()
            .frame(&frame_id)
            .map_err(|e| e.to_string())?;
        let placement = session
            .store
            .document()
            .views
            .iter()
            .find(|view| view.object_id == frame_id);
        let preferred = export_extension(frame);
        let directory = frame_source_path(frame)
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        (
            format!("{} output.{preferred}", frame.name),
            preferred,
            directory,
            placement.map_or(28.0, |view| view.x + 28.0),
            placement.map_or(28.0, |view| view.y + 28.0),
        )
    };
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            // The source's own format leads, because rfd takes the first
            // filter as the dialog's default. The rest follow in the order
            // core offers them, so the chooser can never present a format the
            // writer refuses.
            let mut dialog = rfd::FileDialog::new().add_filter(
                export_format_label(preferred),
                std::slice::from_ref(&preferred),
            );
            for extension in EXPORT_FILE_EXTENSIONS
                .iter()
                .filter(|extension| **extension != preferred)
            {
                dialog = dialog.add_filter(export_format_label(extension), &[extension]);
            }
            dialog = dialog.set_file_name(suggested);
            if let Some(directory) = directory {
                dialog = dialog.set_directory(directory);
            }
            let Some(mut path) = dialog.save_file() else {
                return Ok(None);
            };
            if path.extension().is_none() {
                path.set_extension(preferred);
            }
            path
        }
    };
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|e| e.to_string())?;
    ensure_live(&session)?;
    if path.exists() {
        return Err(
            "Export creates a new file. Choose a filename that is not already in use.".into(),
        );
    }
    // Exporting is a fork, not a mutation of the input. The table being
    // worked on keeps its transformations, so saving the workbook makes that
    // recipe reusable and discarding it does not. The values-only result
    // returns as a fresh imported table with an empty Wrangle chain.
    session
        .store
        .export_frame_file(&frame_id, &path)
        .map_err(|e| e.to_string())?;
    let outcome = import_file_into_session(&mut session, &state.writer_id, &path, false, x, y)?;
    sync_history_menu(
        &window,
        outcome.document.can_undo,
        outcome.document.can_redo,
    );
    scratchwork_window::emit_document_to_peers(&window, &outcome.document);
    Ok(Some(outcome.document))
}

#[tauri::command]
pub(super) fn update_original_delimited(
    window: tauri::WebviewWindow,
    frame_id: String,
    state: State<'_, AppState>,
) -> Result<Option<DocumentView>, String> {
    let (source_path, x, y) = {
        let session = state.document_for(window.label())?;
        let session = session.lock().map_err(|error| error.to_string())?;
        ensure_live(&session)?;
        let frame = session
            .store
            .document()
            .frame(&frame_id)
            .map_err(|error| error.to_string())?;
        let source_path = frame
            .file_origin
            .as_ref()
            .map(|origin| origin.path.clone())
            .ok_or_else(|| {
                "This table has no editable CSV/TSV source to update. Open it as stored data first."
                    .to_string()
            })?;
        let placement = session
            .store
            .document()
            .views
            .iter()
            .find(|view| view.object_id == frame_id);
        (
            source_path,
            placement.map_or(28.0, |view| view.x + 28.0),
            placement.map_or(28.0, |view| view.y + 28.0),
        )
    };
    // Name the format actually being replaced rather than guessing between
    // two. An update writes the source's own format — that is the whole point
    // of it — so a button that named the wrong one would be lying about which
    // file it is about to overwrite.
    let kind = Path::new(&source_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_uppercase)
        .unwrap_or_else(|| "file".to_string());
    let update_label = format!("Update original {kind}");
    let result = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(format!("{update_label}?"))
        .set_description(format!(
            "Replace the original {kind} with this table's current values?\n\nThis cannot be undone for the file. The replacement opens as a new direct read; this transformation chain stays in the workbook."
        ))
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            update_label.clone(),
            "Cancel".into(),
        ))
        .show();
    if result != rfd::MessageDialogResult::Custom(update_label) {
        return Ok(None);
    }

    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    ensure_live(&session)?;
    let write_state_directory = session.journal.paths().root.join("file-write-state");
    let prepared = session
        .store
        .prepare_delimited_write(&frame_id, &write_state_directory)
        .map_err(|error| error.to_string())?;
    prepared.write().map_err(|error| error.to_string())?;
    let outcome = match import_file_into_session(
        &mut session,
        &state.writer_id,
        Path::new(&source_path),
        false,
        x,
        y,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            return match prepared.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!(
                    "{error}; the original file could not be restored: {rollback}"
                )),
            };
        }
    };
    sync_history_menu(
        &window,
        outcome.document.can_undo,
        outcome.document.can_redo,
    );
    scratchwork_window::emit_document_to_peers(&window, &outcome.document);
    Ok(Some(outcome.document))
}
