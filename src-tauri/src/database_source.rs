//! The same query can create a frame or replace its Read source.
use super::*;

#[tauri::command]
pub(super) async fn import_database_source(
    window: tauri::WebviewWindow,
    app: AppHandle,
    input: DatabaseSourceInput,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let source_name = input.source_name.trim().to_string();
    let query = input.query.trim().to_string();
    if source_name.is_empty() || query.is_empty() {
        return Err("A database table needs a name and SQL query".into());
    }
    let connector = ConnectorRecipe::Database {
        connection_id: input.connection_id.clone(),
        source_name: source_name.clone(),
        query,
    };
    let connection =
        database_connections::by_id(&database_connections_path(&app)?, &input.connection_id)?;
    let session = state.document_for(window.label())?;
    let (document_path, document_id) = {
        let session = session.lock().map_err(|error| error.to_string())?;
        (
            session.path.clone(),
            session.store.document_id().to_string(),
        )
    };
    let staged_connector = connector.clone();
    let artifact = tauri::async_runtime::spawn_blocking(move || {
        stage_database_artifact(&document_path, &document_id, &connection, &staged_connector)
    })
    .await
    .map_err(|error| error.to_string())??;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    apply_session_operation(
        &window,
        &mut session,
        &state.writer_id,
        match input.frame_id {
            Some(frame_id) => Operation::SetFrameSource {
                frame_id,
                artifact,
                connector,
            },
            None => Operation::ImportFrameFromArtifact {
                name: source_name,
                artifact,
                connector: Some(connector),
                file_origin: None,
                x: input.x,
                y: input.y,
            },
        },
    )
}
