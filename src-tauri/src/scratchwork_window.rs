//! The workbook-owned Scratchwork window.
//!
//! This is a second view over the same `DocumentSession`, not a second
//! writer and not a copy of the block. Sharing the session Arc is what makes
//! history, persistence, renames, and live recalculation remain workbook
//! behavior while the editor is in an ordinary operating-system window.

use crate::{AppState, DOCUMENT_CHANGED_EVENT, DocumentSession, WindowSession, menu};
use framework_core::DocumentView;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

const SCRATCHWORK_PREFIX: &str = "scratchwork-";
pub const EDITOR_STATE_EVENT: &str = "framework-scratchwork-editor-state";
pub const EDIT_EVENT: &str = "framework-scratchwork-edit";
pub const CLEAR_EDITOR_EVENT: &str = "framework-scratchwork-clear-editor";
pub const WINDOW_CLOSED_EVENT: &str = "framework-scratchwork-window-closed";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScratchworkEditorState {
    revision: u64,
    active: bool,
    draft: String,
    selection_start: usize,
    selection_end: usize,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScratchworkEdit {
    text: String,
    replace_selection: bool,
    refocus: bool,
}

pub fn is_scratchwork_label(label: &str) -> bool {
    label.starts_with(SCRATCHWORK_PREFIX)
}

fn owner_label(label: &str) -> &str {
    label.strip_prefix(SCRATCHWORK_PREFIX).unwrap_or(label)
}

fn scratchwork_label(label: &str) -> String {
    format!("{SCRATCHWORK_PREFIX}{}", owner_label(label))
}

fn peer_labels(state: &AppState, label: &str) -> Vec<String> {
    let Ok(sessions) = state.sessions.lock() else {
        return Vec::new();
    };
    let Some(current) = sessions.get(label) else {
        return Vec::new();
    };
    sessions
        .iter()
        .filter(|(candidate, session)| {
            candidate.as_str() != label && Arc::ptr_eq(&current.document, &session.document)
        })
        .map(|(candidate, _)| candidate.clone())
        .collect()
}

/// A command return updates its own webview; sibling views need the same view
/// as an event. The payload is cloned only for the uncommon second view.
pub fn emit_document_to_peers(window: &tauri::WebviewWindow, document: &DocumentView) {
    let Some(state) = window.try_state::<AppState>() else {
        return;
    };
    for label in peer_labels(&state, window.label()) {
        let _ = window
            .app_handle()
            .emit_to(label, DOCUMENT_CHANGED_EVENT, document);
    }
}

pub fn emit_document_to_peers_from(
    app: &AppHandle,
    state: &AppState,
    label: &str,
    document: &DocumentView,
) {
    for peer in peer_labels(state, label) {
        let _ = app.emit_to(peer, DOCUMENT_CHANGED_EVENT, document);
    }
}

/// Collaboration scans each shared document once, then fans the result out
/// to every view. Scanning both labels would let the first consume the journal
/// merge and leave the second with no event to hear.
pub fn grouped_sessions(state: &AppState) -> Vec<(Arc<Mutex<DocumentSession>>, Vec<String>)> {
    let Ok(sessions) = state.sessions.lock() else {
        return Vec::new();
    };
    let mut groups: Vec<(Arc<Mutex<DocumentSession>>, Vec<String>)> = Vec::new();
    for (label, session) in sessions.iter() {
        if let Some((_, labels)) = groups
            .iter_mut()
            .find(|(document, _)| Arc::ptr_eq(document, &session.document))
        {
            labels.push(label.clone());
        } else {
            groups.push((Arc::clone(&session.document), vec![label.clone()]));
        }
    }
    groups
}

pub fn set_workbook_titles(app: &AppHandle, label: &str, document_name: &str) {
    let owner = owner_label(label);
    if let Some(window) = app.get_webview_window(owner) {
        let _ = window.set_title(&format!("{document_name} — FrameWork"));
    }
    if let Some(window) = app.get_webview_window(&scratchwork_label(owner)) {
        let _ = window.set_title(&format!("Scratchwork — {document_name} — FrameWork"));
    }
}

#[tauri::command]
pub fn open_scratchwork_window(
    window: tauri::WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let owner = owner_label(window.label());
    let label = scratchwork_label(owner);
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    let document = state.document_for(owner)?;
    let document_name = document
        .lock()
        .map_err(|error| error.to_string())?
        .store
        .view()
        .document
        .name;
    state
        .sessions
        .lock()
        .map_err(|error| error.to_string())?
        .insert(
            label.clone(),
            WindowSession {
                document,
                started_blank: false,
            },
        );

    let built = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("index.html?scratchwork=1".into()),
    )
    .title(format!("Scratchwork — {document_name} — FrameWork"))
    .inner_size(760.0, 520.0)
    .min_inner_size(420.0, 280.0)
    .build();
    if built.is_err()
        && let Ok(mut sessions) = state.sessions.lock()
    {
        sessions.remove(&label);
    }
    built.map(|_| ()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn focus_scratchwork_window(window: tauri::WebviewWindow, app: AppHandle) -> bool {
    let label = scratchwork_label(window.label());
    let Some(scratchwork) = app.get_webview_window(&label) else {
        return false;
    };
    let _ = scratchwork.show();
    let _ = scratchwork.set_focus();
    let _ = app.emit_to(label, menu::MENU_COMMAND_EVENT, "scratchpad");
    true
}

#[tauri::command]
pub fn publish_scratchwork_editor_state(
    window: tauri::WebviewWindow,
    app: AppHandle,
    editor: ScratchworkEditorState,
) -> Result<(), String> {
    if !is_scratchwork_label(window.label()) {
        return Err("Only a Scratchwork window can publish its editor state".into());
    }
    app.emit_to(owner_label(window.label()), EDITOR_STATE_EVENT, editor)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn edit_scratchwork_window(
    window: tauri::WebviewWindow,
    app: AppHandle,
    edit: ScratchworkEdit,
) -> Result<bool, String> {
    let label = scratchwork_label(window.label());
    if app.get_webview_window(&label).is_none() {
        return Ok(false);
    }
    app.emit_to(label, EDIT_EVENT, edit)
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn clear_scratchwork_window_editor(
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<bool, String> {
    let label = scratchwork_label(window.label());
    if app.get_webview_window(&label).is_none() {
        return Ok(false);
    }
    app.emit_to(label, CLEAR_EDITOR_EVENT, ())
        .map_err(|error| error.to_string())?;
    Ok(true)
}

/// A pop-out still shares the application menu. Commands that belong to the
/// workbook rather than this single editor are handed back to its owner.
#[tauri::command]
pub fn forward_scratchwork_command(
    window: tauri::WebviewWindow,
    app: AppHandle,
    command: String,
) -> Result<(), String> {
    if !is_scratchwork_label(window.label()) {
        return Err("Only a Scratchwork window can forward workbook commands".into());
    }
    let owner = owner_label(window.label());
    let owner_window = app
        .get_webview_window(owner)
        .ok_or_else(|| "The workbook window is no longer open".to_string())?;
    let _ = owner_window.show();
    let _ = owner_window.set_focus();
    app.emit_to(owner, menu::MENU_COMMAND_EVENT, command)
        .map_err(|error| error.to_string())
}

pub fn handle_destroyed(window: &tauri::Window, state: &AppState) {
    let label = window.label().to_string();
    if is_scratchwork_label(&label) {
        if let Ok(mut sessions) = state.sessions.lock() {
            sessions.remove(&label);
        }
        let _ = window
            .app_handle()
            .emit_to(owner_label(&label), WINDOW_CLOSED_EVENT, ());
        return;
    }

    let scratchwork = scratchwork_label(&label);
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.remove(&label);
        sessions.remove(&scratchwork);
    }
    if let Some(child) = window.app_handle().get_webview_window(&scratchwork) {
        let _ = child.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratchwork_labels_round_trip_to_their_workbook() {
        assert_eq!(scratchwork_label("main"), "scratchwork-main");
        assert_eq!(owner_label("scratchwork-document-123"), "document-123");
        assert_eq!(scratchwork_label("scratchwork-main"), "scratchwork-main");
    }
}
