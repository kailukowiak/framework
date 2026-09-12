//! Availability checks can block on cloud-backed or permission-gated files.
//! They must never run on the window thread: startup renders the library while
//! these reads are in flight, and the user must still be able to dismiss it.
use crate::{
    MAX_RECENT_DOCUMENTS, RecentDocument, is_framework_document_path, recent_documents_path,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::AppHandle;

#[tauri::command]
pub(crate) async fn list_recent_documents(app: AppHandle) -> Result<Vec<RecentDocument>, String> {
    let path = recent_documents_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || read_recent(path))
        .await
        .map_err(|error| error.to_string())?
}

fn read_recent(path: PathBuf) -> Result<Vec<RecentDocument>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut documents: Vec<RecentDocument> = serde_json::from_str(&contents).unwrap_or_default();
    documents.retain(|document| {
        let path = Path::new(&document.path);
        path.is_file() && is_framework_document_path(path)
    });
    // Recomputed on every list, not trusted from the stored JSON: a file
    // that was readable when it was recorded can go dark later (a macOS TCC
    // deny arriving, or permissions changing underneath it), and the entry
    // should reflect that rather than repeat what was true when it was added.
    for document in &mut documents {
        document.readable = fs::File::open(&document.path).is_ok();
    }
    documents.truncate(MAX_RECENT_DOCUMENTS);
    Ok(documents)
}
