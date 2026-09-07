//! A failed final save must leave its writer alive. Reporting an error to a
//! window that is already closing is not recovery; keep both close and quit
//! cancellable until every pending document has actually reached disk.
use super::*;

fn report_failure(app: &AppHandle, labels: &[String], error: &str) {
    let message = format!(
        "Could not save: {error}. Your document is still open. Retry closing to save again, or choose File → Save As to save elsewhere."
    );
    for label in labels {
        let _ = app.emit_to(label, COLLABORATION_FAILED_EVENT, &message);
    }
}

/// Flushes the window's document and reports a failure to that window.
/// An explicit close always hears about it, because it is what the person
/// is trying to do; a blur hears about a given failure once, until a write
/// succeeds again. Returns whether the document is safely on disk.
fn flush_window_document(window: &tauri::Window, state: &AppState, every_time: bool) -> bool {
    // Auxiliary windows without a document have no pending data to guard.
    let Ok(session) = state.document_for(window.label()) else {
        return true;
    };
    let mut session = match session.lock() {
        Ok(session) => session,
        Err(error) => {
            report_failure(
                window.app_handle(),
                &[window.label().to_string()],
                &error.to_string(),
            );
            return false;
        }
    };
    match flush_session(&mut session) {
        Ok(()) => true,
        Err(error) => {
            if every_time || session.pending_write.report_failure_once() {
                report_failure(window.app_handle(), &[window.label().to_string()], &error);
            }
            false
        }
    }
}

pub(super) fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let Some(state) = window.try_state::<AppState>() else {
        return;
    };
    match event {
        // `CloseRequested` fires while the window (and this document's
        // path) is still known; flushing here, ahead of `Destroyed`, is what
        // keeps a debounced write from being silently dropped when a window
        // closes before its two-second idle window elapses on its own.
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if !flush_window_document(window, &state, true) {
                api.prevent_close();
            }
        }
        tauri::WindowEvent::Destroyed => {
            scratchwork_window::handle_destroyed(window, &state);
        }
        // Losing focus is exactly the moment a person is likely to switch to
        // a file browser, a sync client's UI, or another app entirely --
        // any of which may read the file next. A pending write should not
        // still be sitting in memory when that happens.
        tauri::WindowEvent::Focused(false) => {
            flush_window_document(window, &state, false);
        }
        tauri::WindowEvent::Focused(true) => {
            if let Ok(session) = state.document_for(window.label())
                && let Ok(session) = session.lock()
                && let Some(history) = window.try_state::<menu::HistoryMenuItems<tauri::Wry>>()
            {
                let view = session.store.view();
                history.set(view.can_undo, view.can_redo);
            }
        }
        _ => {}
    }
}

/// Quit may arrive without a CloseRequested event. Flush each shared session
/// once, report failures to its surviving views, and let the caller refuse
/// exit. Save As remains usable because it saves the in-memory store directly.
pub(super) fn flush_all_sessions(app: &AppHandle) -> bool {
    let mut saved = true;
    for (session, labels) in scratchwork_window::grouped_sessions(&app.state::<AppState>()) {
        let result = session
            .lock()
            .map_err(|error| error.to_string())
            .and_then(|mut session| flush_session(&mut session));
        if let Err(error) = result {
            saved = false;
            report_failure(app, &labels, &error);
        }
    }
    saved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_flush_keeps_pending_data_for_retry_or_save_as() {
        let root = env::temp_dir().join(format!("framework-save-failure-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let mut session = blank_session(root.join("work.fw"), false).unwrap();
        schedule_persist(&mut session);
        // A directory in place of the snapshot reliably refuses replacement,
        // without relying on permissions that differ under elevated tests.
        session.path = root.clone();
        assert!(flush_session(&mut session).is_err());
        assert!(session.pending_write.is_pending());
        session.path = root.join("recovered.fw");
        flush_session(&mut session).unwrap();
        assert!(!session.pending_write.is_pending());
        assert!(session.path.is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
