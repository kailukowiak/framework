//! Export scope and its exact preview use one engine query contract.
use super::*;

#[tauri::command]
pub(super) fn export_document_excel(
    window: tauri::WebviewWindow,
    frame_ids: Vec<String>,
    path: Option<String>,
    include_lineage: bool,
    current_view: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let suggested = {
                let session = state.document_for(window.label())?;
                let session = session.lock().map_err(|error| error.to_string())?;
                format!("{}.xlsx", session.store.document().name)
            };
            let Some(mut path) = rfd::FileDialog::new()
                .add_filter("Excel workbook", &["xlsx"])
                .set_file_name(suggested)
                .save_file()
            else {
                return Ok(None);
            };
            if path.extension().is_none() {
                path.set_extension("xlsx");
            }
            path
        }
    };
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .export_excel_scoped(
            &frame_ids,
            &path,
            include_lineage,
            current_view.unwrap_or(false),
        )
        .map_err(|error| error.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub(super) fn export_row_counts(
    window: tauri::WebviewWindow,
    frame_ids: Vec<String>,
    current_view: bool,
    state: State<'_, AppState>,
) -> Result<HashMap<String, usize>, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    frame_ids
        .into_iter()
        .map(|id| {
            let count = session
                .store
                .export_row_count(&id, current_view)
                .map_err(|error| error.to_string())?;
            Ok((id, count))
        })
        .collect()
}
