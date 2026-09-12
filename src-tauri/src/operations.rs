use crate::{AppState, apply_session_operation_inner, scratchwork_window, sync_history_menu};
use framework_core::{DocumentView, Operation};
use tauri::State;

/// Model fitting is deliberate work and may take longer than an ordinary cell
/// edit. Keep the command's computation off the window event loop so the editor
/// can render its pending state. The same session lock and journal path still
/// serialize the entire operation; a failed prepare records no partial fit.
/// Release that lock before asking the window thread about focus or sending
/// notifications: a synchronous command on that thread may itself be waiting
/// for the document. Holding the lock across `is_focused` would deadlock both.
#[tauri::command]
pub(crate) async fn apply_operation(
    window: tauri::WebviewWindow,
    operation: Operation,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let writer_id = state.writer_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let view = {
            let mut session = session.lock().map_err(|error| error.to_string())?;
            apply_session_operation_inner(&mut session, &writer_id, operation)?
        };
        sync_history_menu(&window, view.can_undo, view.can_redo);
        scratchwork_window::emit_document_to_peers(&window, &view);
        Ok(view)
    })
    .await
    .map_err(|error| error.to_string())?
}
