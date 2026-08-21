use framework_core::{
    ArtifactSweep, CollaborationPaths, ConnectorRecipe, DataArtifact, DataObject, Document,
    DocumentView, EventJournal, ExcelRangePreview, ExcelWorkbookInfo, Operation, Store,
    create_data_artifact, create_excel_range_artifact,
    inspect_excel_workbook as read_excel_workbook, is_framework_document_path,
    preview_excel_range as read_excel_range_preview,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs, thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;
use uuid::Uuid;

mod cli_connectors;
mod database_connections;
mod menu;

const SCRATCH_DOCUMENT_NAME: &str = "untitled.fw";
const BLANK_DOCUMENT_TITLE: &str = "Untitled";
const DOCUMENT_OPENED_EVENT: &str = "framework-document-opened";
const DOCUMENT_OPEN_FAILED_EVENT: &str = "framework-document-open-failed";
const DOCUMENT_CHANGED_EVENT: &str = "framework-document-changed";
const COLLABORATION_FAILED_EVENT: &str = "framework-collaboration-failed";
const WRITER_ID_NAME: &str = "writer-id";
const SAMPLE_DOCUMENT_DIRECTORY: &str = ".framework-samples";
const SAMPLE_DOCUMENT_ENVIRONMENT: &str = "FRAMEWORK_SAMPLE_DIRECTORY";
const RECENT_DOCUMENTS_NAME: &str = "recent-documents.json";
const MCP_ENABLED_NAME: &str = "mcp-enabled";
const MAX_RECENT_DOCUMENTS: usize = 10;
const TUTORIAL_DIRECTORY_NAME: &str = "FrameWork Tutorials";
#[cfg(feature = "e2e")]
const E2E_TUTORIAL_DIRECTORY_ENVIRONMENT: &str = "FRAMEWORK_E2E_TUTORIAL_DIRECTORY";

struct DocumentSession {
    store: Store,
    path: PathBuf,
    journal: EventJournal,
    snapshot_dirty: bool,
    /// Whether `path` is the throwaway scratch this launch created rather
    /// than a document the user has anywhere they can find again. The window
    /// reports no path at all for one, because "saved locally" pointing at a
    /// temp directory is how work gets lost.
    scratch: bool,
}

struct AppState {
    sessions: Mutex<HashMap<String, WindowSession>>,
    writer_id: String,
}

struct WindowSession {
    document: Arc<Mutex<DocumentSession>>,
    /// Whether this window began without a requested document. New windows
    /// deliberately begin on a blank canvas without raising the library;
    /// only the first, unrequested application window does that.
    started_blank: bool,
}

impl AppState {
    fn document_for(&self, label: &str) -> Result<Arc<Mutex<DocumentSession>>, String> {
        self.sessions
            .lock()
            .map_err(|error| error.to_string())?
            .get(label)
            .map(|session| Arc::clone(&session.document))
            .ok_or_else(|| format!("Window {label} has no document session"))
    }

    fn replace_document(&self, label: &str, document: DocumentSession) -> Result<(), String> {
        let session = {
            let mut sessions = self.sessions.lock().map_err(|error| error.to_string())?;
            let session = sessions
                .get_mut(label)
                .ok_or_else(|| format!("Window {label} has no document session"))?;
            session.started_blank = false;
            Arc::clone(&session.document)
        };
        *session.lock().map_err(|error| error.to_string())? = document;
        Ok(())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenedDocument {
    document: DocumentView,
    path: String,
}

/// The import source stays on the canvas, while this id lets the interface
/// immediately select the derived frame that stacks it under the frame the
/// person started from.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportAndAppendResult {
    document: DocumentView,
    appended_frame_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliSourceInput {
    x: f64,
    y: f64,
    profile_id: String,
    source_label: String,
    query: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseSourceInput {
    x: f64,
    y: f64,
    connection_id: String,
    source_name: String,
    query: String,
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct SampleDocument {
    file_name: String,
    title: String,
    path: String,
    frame_count: usize,
    category: String,
}

/// A workbook we ship for learning, copied into a visible Documents folder
/// only when the person asks for it. It deliberately has a path of its own,
/// rather than being a sample: the point of a tutorial is to change it.
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct TutorialDocument {
    title: String,
    lesson: String,
    kind: String,
    path: String,
    exists: bool,
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct TutorialLibrary {
    directory: String,
    documents: Vec<TutorialDocument>,
}

/// A source workbook travels in the desktop bundle, so an installed build
/// can make the tutorial folder without needing this checkout or a network
/// connection. Start and answer-key copies use different parent directories:
/// their canonical files intentionally share document IDs, and collaboration
/// sidecars are keyed by that ID beneath a document's parent directory.
struct BundledTutorial {
    lesson: &'static str,
    kind: &'static str,
    relative_path: &'static str,
    contents: &'static [u8],
    assets: &'static [BundledTutorialAsset],
}

struct BundledTutorialAsset {
    relative_path: &'static str,
    contents: &'static [u8],
}

const EXCEL_START_ASSETS: &[BundledTutorialAsset] = &[
    BundledTutorialAsset {
        relative_path: "simple-customers.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/simple-customers.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "multi-table-operations.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/multi-table-operations.xlsx"),
    },
];

const EXCEL_FINISHED_ASSETS: &[BundledTutorialAsset] = &[
    BundledTutorialAsset {
        relative_path: "simple-customers.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/simple-customers.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "multi-table-operations.xlsx",
        contents: include_bytes!("../../tutorials/excel-import/source/multi-table-operations.xlsx"),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/22141794ab301df27ad6926c4a695aaf4c44a69ea570924cc20706e6753d26a4.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/22141794ab301df27ad6926c4a695aaf4c44a69ea570924cc20706e6753d26a4.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/31743e7aabd9104b7499a9bb55b533db5313c98f6dcca7f4d220b2f9d2620216.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/31743e7aabd9104b7499a9bb55b533db5313c98f6dcca7f4d220b2f9d2620216.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/8dc284978b1a45639111232b51562167bf56f6051329263ef5c0119ec0766411.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/8dc284978b1a45639111232b51562167bf56f6051329263ef5c0119ec0766411.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/de982e3a709421e05a6804a2a09cda224d0194b27cad4d4455129c120e702061.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/de982e3a709421e05a6804a2a09cda224d0194b27cad4d4455129c120e702061.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/e1527918fc567a68ca7d844f984ad658b63d0e05155e5776675222c0a36284ce.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/e1527918fc567a68ca7d844f984ad658b63d0e05155e5776675222c0a36284ce.parquet"
        ),
    },
    BundledTutorialAsset {
        relative_path: "finished-data/e1cc295c5c0f9b2c218dca3a3e08680ab000a1d8c3f84ddc92cadd334f2e51b8.parquet",
        contents: include_bytes!(
            "../../tutorials/excel-import/finished-data/e1cc295c5c0f9b2c218dca3a3e08680ab000a1d8c3f84ddc92cadd334f2e51b8.parquet"
        ),
    },
];

const BUNDLED_TUTORIALS: &[BundledTutorial] = &[
    BundledTutorial {
        lesson: "Your first FrameWork workbook",
        kind: "Start",
        relative_path: "Your first FrameWork workbook/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/first-workbook/first-workbook-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Your first FrameWork workbook",
        kind: "Answer key",
        relative_path: "Your first FrameWork workbook/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/first-workbook/first-workbook-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Importing an Excel workbook",
        kind: "Start",
        relative_path: "Importing an Excel workbook/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/excel-import/excel-import-start.fw"),
        assets: EXCEL_START_ASSETS,
    },
    BundledTutorial {
        lesson: "Importing an Excel workbook",
        kind: "Answer key",
        relative_path: "Importing an Excel workbook/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/excel-import/excel-import-finished.fw"),
        assets: EXCEL_FINISHED_ASSETS,
    },
    BundledTutorial {
        lesson: "Month-over-month formulas by pointing",
        kind: "Start",
        relative_path: "Month-over-month formulas by pointing/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/formula-clicks/formula-clicks-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-over-month formulas by pointing",
        kind: "Answer key",
        relative_path: "Month-over-month formulas by pointing/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/formula-clicks/formula-clicks-finished.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-end close",
        kind: "Start",
        relative_path: "Month-end close/Start/Workbook.fw",
        contents: include_bytes!("../../tutorials/month-end-close/month-end-close-start.fw"),
        assets: &[],
    },
    BundledTutorial {
        lesson: "Month-end close",
        kind: "Answer key",
        relative_path: "Month-end close/Answer key/Workbook.fw",
        contents: include_bytes!("../../tutorials/month-end-close/month-end-close-finished.fw"),
        assets: &[],
    },
];

#[derive(Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct RecentDocument {
    title: String,
    path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct McpSettings {
    enabled: bool,
    executable_path: Option<String>,
}

fn mcp_enabled_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(MCP_ENABLED_NAME))
}

/// The stdio server is a sibling executable in development and in builds
/// that choose to bundle it. Keeping discovery here means the interface can
/// give an exact command when one exists without pretending an installed app
/// contains a development checkout when it does not.
fn installed_mcp_executable() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let directory = executable.parent()?;
    #[cfg(windows)]
    let name = "framework-mcp.exe";
    #[cfg(not(windows))]
    let name = "framework-mcp";
    let candidate = directory.join(name);
    candidate.is_file().then_some(candidate)
}

#[tauri::command]
fn get_mcp_settings(app: AppHandle) -> Result<McpSettings, String> {
    Ok(McpSettings {
        enabled: mcp_enabled_path(&app)?.is_file(),
        executable_path: installed_mcp_executable().map(|path| path.display().to_string()),
    })
}

#[tauri::command]
fn set_mcp_enabled(app: AppHandle, enabled: bool) -> Result<McpSettings, String> {
    let path = mcp_enabled_path(&app)?;
    if enabled {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        }
        fs::write(&path, b"enabled\n").map_err(|error| error.to_string())?;
    } else if path.exists() {
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }
    get_mcp_settings(app)
}

#[tauri::command]
fn get_document(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    Ok(session.store.view())
}

#[tauri::command]
fn get_frame_page(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
    offset: usize,
    limit: usize,
) -> Result<framework_core::FramePage, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .get_frame_page(&frame_id, offset, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_frame_summary(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
) -> Result<framework_core::FrameSummary, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .get_frame_summary(&frame_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_block_line_page(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    block_id: String,
    line_id: String,
    offset: usize,
    limit: usize,
) -> Result<framework_core::BlockLinePage, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .get_block_line_page(&block_id, &line_id, offset, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_frame_query_plan(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
) -> Result<framework_core::FrameQueryPlan, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .get_frame_query_plan(&frame_id)
        .map_err(|error| error.to_string())
}

/// What the chain the editor is drafting would produce, step by step.
///
/// Unsaved and read-only: it answers from the query plan without running
/// anything, and a chain that stops at a broken step still reports the
/// schemas before it.
#[tauri::command]
fn preview_frame_pipeline(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
    steps: Vec<framework_core::FrameStepInput>,
) -> Result<framework_core::PipelineSchema, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .preview_frame_pipeline(&frame_id, steps)
        .map_err(|error| error.to_string())
}

/// The distinct values a conditional-formatting rule's formula produces,
/// commonest first — what the Rules panel fills a case list from.
#[tauri::command]
fn frame_formula_values(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
    formula: String,
    limit: usize,
) -> Result<Vec<String>, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .frame_formula_values(&frame_id, &formula, limit)
        .map_err(|error| error.to_string())
}

/// The first rows as they stand after one step of a draft chain.
///
/// Runs the query, unlike the schema preview: the limit is pushed into the
/// plan, but this is the one call in the editor that reads data.
#[tauri::command]
fn sample_frame_step(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
    steps: Vec<framework_core::FrameStepInput>,
    step_index: usize,
    limit: usize,
) -> Result<framework_core::StepSample, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .sample_frame_step(&frame_id, steps, step_index, limit)
        .map_err(|error| error.to_string())
}

/// Completion for a formula, optionally at a position in a chain.
///
/// `steps`/`step_index` come from the step editor, whose formulas see what
/// the steps before them leave behind rather than the frame's own columns —
/// after a summarize those are two different things, and completing against
/// the wrong one suggests names the formula cannot use.
/// What a value or result depends on, recursively, with each stop's
/// current value attached — the debug trace.
///
/// A frame shows up as a leaf: its own wrangle chain is walked separately,
/// through `sample_frame_step`.
#[tauri::command]
fn dependency_graph(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    object_id: String,
) -> Result<framework_core::DependencyNode, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .dependency_graph(&object_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn complete_formula(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    frame_id: String,
    formula_text: String,
    cursor_pos: usize,
    steps: Option<Vec<framework_core::FrameStepInput>>,
    step_index: Option<usize>,
) -> Result<framework_core::CompletionResult, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    Ok(match (steps, step_index) {
        (Some(steps), Some(step_index)) => session.store.complete_step_formula(
            &frame_id,
            steps,
            step_index,
            &formula_text,
            cursor_pos,
        ),
        _ => session
            .store
            .complete_formula(&frame_id, &formula_text, cursor_pos),
    })
}

/// Where the open document lives, or `None` while it is only the scratch.
///
/// The scratch has a path — it has to, the journal and the artifact sidecar
/// are files — but it is a temporary directory nobody should be told to look
/// in. Reporting no path is what makes the window say the canvas is unsaved
/// instead of claiming it is safely on disk.
#[tauri::command]
fn get_document_path(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    Ok((!session.scratch).then(|| session.path.display().to_string()))
}

/// Whether the window should raise the Data library on mount.
///
/// True when nothing named a document — the canvas behind it is the blank
/// scratch document, so the opener is the only thing on screen worth acting
/// on. False when a path came from the command line, a file association, or
/// a second-instance launch: that document *is* the request.
#[tauri::command]
fn should_open_library(window: tauri::WebviewWindow, state: State<'_, AppState>) -> bool {
    state
        .sessions
        .lock()
        .ok()
        .and_then(|sessions| {
            sessions
                .get(window.label())
                .map(|session| session.started_blank)
        })
        .unwrap_or(false)
}

#[tauri::command]
fn open_document(
    path: String,
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<OpenedDocument, String> {
    let opened = open_document_at(&app, window.label(), PathBuf::from(path), false)?;
    let _ = remember_recent_document(&app, &opened);
    Ok(opened)
}

#[tauri::command]
fn open_document_dialog(
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<Option<OpenedDocument>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("FrameWork document", &["fw"])
        .pick_file()
    else {
        return Ok(None);
    };
    let opened = open_document_at(&app, window.label(), path, false)?;
    let _ = remember_recent_document(&app, &opened);
    Ok(Some(opened))
}

#[tauri::command]
fn new_document_dialog(
    name: String,
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<Option<OpenedDocument>, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Document name cannot be empty".into());
    }
    let suggested_name = format!("{}.fw", name.replace(['/', '\\'], "-"));
    let Some(mut path) = rfd::FileDialog::new()
        .add_filter("FrameWork document", &["fw"])
        .set_file_name(suggested_name)
        .save_file()
    else {
        return Ok(None);
    };
    if path.extension().is_none() {
        path.set_extension("fw");
    }
    if !is_framework_document_path(&path) {
        return Err("FrameWork documents must use the .fw extension".into());
    }

    let store = Store::new(Document::blank(name));
    store.save(&path).map_err(|error| error.to_string())?;
    let opened = open_document_at(&app, window.label(), path, false)?;
    let _ = remember_recent_document(&app, &opened);
    Ok(Some(opened))
}

#[tauri::command]
fn save_document_as_dialog(
    window: tauri::WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<OpenedDocument>, String> {
    // A scratch canvas has no file name worth proposing — it is called
    // `untitled.fw` in a temporary directory — so the dialog proposes what
    // the user named the document instead.
    let current_name = {
        let session = state.document_for(window.label())?;
        let session = session.lock().map_err(|error| error.to_string())?;
        if session.scratch {
            format!(
                "{}.fw",
                session.store.view().document.name.replace(['/', '\\'], "-")
            )
        } else {
            session
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(SCRATCH_DOCUMENT_NAME)
                .to_string()
        }
    };
    let Some(mut path) = rfd::FileDialog::new()
        .add_filter("FrameWork document", &["fw"])
        .set_file_name(current_name)
        .save_file()
    else {
        return Ok(None);
    };
    if path.extension().is_none() {
        path.set_extension("fw");
    }
    if !is_framework_document_path(&path) {
        return Err("FrameWork documents must use the .fw extension".into());
    }
    if focus_window_for_path(&app, &path, Some(window.label()))? {
        return Err("That document is already open in another window".into());
    }

    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .save_as(&path)
        .map_err(|error| error.to_string())?;
    let journal = EventJournal::open(&path, &session.store.view().document.id)
        .map_err(|error| error.to_string())?;
    session.path = path.clone();
    session.journal = journal;
    session.snapshot_dirty = false;
    // Save As is how a scratch canvas becomes a document: it now has a home
    // the user chose and can find again.
    session.scratch = false;
    let payload = OpenedDocument {
        document: session.store.view(),
        path: path.display().to_string(),
    };
    drop(session);

    let title = format!("{} — FrameWork", payload.document.document.name);
    let _ = window.set_title(&title);
    let _ = remember_recent_document(&app, &payload);
    Ok(Some(payload))
}

#[tauri::command]
fn list_recent_documents(app: AppHandle) -> Result<Vec<RecentDocument>, String> {
    let path = recent_documents_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut documents: Vec<RecentDocument> = serde_json::from_str(&contents).unwrap_or_default();
    documents.retain(|document| {
        let path = Path::new(&document.path);
        path.is_file() && is_framework_document_path(path)
    });
    documents.truncate(MAX_RECENT_DOCUMENTS);
    Ok(documents)
}

fn cli_connector_profiles_path(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(feature = "e2e")]
    if let Some(path) = std::env::var_os("FRAMEWORK_E2E_CONNECTOR_PROFILE_PATH") {
        return Ok(PathBuf::from(path));
    }
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(cli_connectors::PROFILE_STORE_NAME))
}

fn database_connections_path(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(feature = "e2e")]
    if let Some(path) = std::env::var_os("FRAMEWORK_E2E_DATABASE_CONNECTION_PATH") {
        return Ok(PathBuf::from(path));
    }
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(database_connections::CONNECTION_STORE_NAME))
}

#[tauri::command]
fn list_cli_connector_profiles(
    app: AppHandle,
) -> Result<Vec<cli_connectors::CliConnectorProfile>, String> {
    cli_connectors::load_profiles(&cli_connector_profiles_path(&app)?)
}

#[tauri::command]
fn save_cli_connector_profile(
    app: AppHandle,
    profile: cli_connectors::CliConnectorProfile,
) -> Result<cli_connectors::CliConnectorProfile, String> {
    cli_connectors::save_profile(&cli_connector_profiles_path(&app)?, profile)
}

#[tauri::command]
fn list_database_connections(
    app: AppHandle,
) -> Result<Vec<database_connections::DatabaseConnection>, String> {
    database_connections::load(&database_connections_path(&app)?)
}

#[tauri::command]
fn save_database_connection(
    app: AppHandle,
    connection: database_connections::DatabaseConnection,
) -> Result<database_connections::DatabaseConnection, String> {
    database_connections::save(&database_connections_path(&app)?, connection)
}

fn recent_documents_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(RECENT_DOCUMENTS_NAME))
}

fn remember_recent_document(app: &AppHandle, opened: &OpenedDocument) -> Result<(), String> {
    let recent_path = recent_documents_path(app)?;
    let mut documents = if recent_path.exists() {
        fs::read_to_string(&recent_path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Vec<RecentDocument>>(&contents).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    documents.retain(|document| document.path != opened.path);
    documents.insert(
        0,
        RecentDocument {
            title: opened.document.document.name.clone(),
            path: opened.path.clone(),
        },
    );
    documents.truncate(MAX_RECENT_DOCUMENTS);
    if let Some(parent) = recent_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(&documents).map_err(|error| error.to_string())?;
    fs::write(recent_path, contents).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_sample_documents() -> Result<Vec<SampleDocument>, String> {
    let Some(directory) = sample_library_directory()? else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    collect_sample_paths(&directory, &mut paths)?;
    let mut samples = paths
        .into_iter()
        .filter_map(|path| {
            let store = Store::load(&path).ok()?;
            let document = store.view().document;
            let relative = path.strip_prefix(&directory).ok()?;
            let category = relative
                .parent()
                .and_then(|parent| parent.components().next())
                .and_then(|component| component.as_os_str().to_str())
                .map(sample_category_label)
                .unwrap_or_else(|| "Examples".into());
            Some(SampleDocument {
                file_name: relative.to_str()?.to_string(),
                title: document.name,
                path: path.display().to_string(),
                frame_count: document
                    .objects
                    .iter()
                    .filter(|object| matches!(object, framework_core::DataObject::Frame(_)))
                    .count(),
                category,
            })
        })
        .collect::<Vec<_>>();
    samples.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.title.cmp(&right.title))
    });
    Ok(samples)
}

fn collect_sample_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect_sample_paths(&path, paths)?;
        } else if is_framework_document_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn sample_category_label(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => "Examples".into(),
    }
}

fn sample_library_directory() -> Result<Option<PathBuf>, String> {
    if let Some(configured) = env::var_os(SAMPLE_DOCUMENT_ENVIRONMENT) {
        let path = PathBuf::from(configured);
        if path.is_dir() {
            return Ok(Some(
                path.canonicalize().map_err(|error| error.to_string())?,
            ));
        }
        return Err(format!(
            "{SAMPLE_DOCUMENT_ENVIRONMENT} does not point to a sample directory: {}",
            path.display()
        ));
    }

    let current_directory = env::current_dir().map_err(|error| error.to_string())?;
    if let Some(path) = find_sample_library_from(&current_directory) {
        return Ok(Some(path));
    }

    // During `tauri dev`, the process may start in `src-tauri` or a build
    // directory. CARGO_MANIFEST_DIR reliably points back to that source tree.
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(find_sample_library_from(&manifest_directory))
}

fn find_sample_library_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|ancestor| {
        let candidate = ancestor.join(SAMPLE_DOCUMENT_DIRECTORY);
        candidate
            .is_dir()
            .then(|| candidate.canonicalize().unwrap_or(candidate))
    })
}

/// The tutorial copies belong somewhere a person can browse, back up, and
/// share. Documents is preferable to app support for that reason; the latter
/// remains a fallback for platforms where Tauri cannot identify Documents.
fn tutorial_library_directory(app: &AppHandle) -> Result<PathBuf, String> {
    // Resetting tutorials is deliberately part of the native integration
    // suite because it crosses the UI, command, filesystem, and engine seam.
    // That must never grant a test permission to replace the person's real
    // tutorial workbooks. Only binaries compiled with the e2e feature honor
    // this private directory; production builds cannot be redirected by an
    // ambient environment variable with the same name.
    #[cfg(feature = "e2e")]
    if let Some(directory) = env::var_os(E2E_TUTORIAL_DIRECTORY_ENVIRONMENT) {
        return Ok(PathBuf::from(directory));
    }

    let base = app
        .path()
        .document_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|error| error.to_string())?;
    Ok(base.join(TUTORIAL_DIRECTORY_NAME))
}

fn tutorial_library(directory: &Path) -> TutorialLibrary {
    TutorialLibrary {
        directory: directory.display().to_string(),
        documents: BUNDLED_TUTORIALS
            .iter()
            .map(|tutorial| {
                let path = directory.join(tutorial.relative_path);
                TutorialDocument {
                    title: format!("{} — {}", tutorial.lesson, tutorial.kind),
                    lesson: tutorial.lesson.into(),
                    kind: tutorial.kind.into(),
                    exists: path.is_file(),
                    path: path.display().to_string(),
                }
            })
            .collect(),
    }
}

fn document_id_at(path: &Path) -> Option<String> {
    Store::load(path)
        .ok()
        .map(|store| store.document().id.to_string())
}

/// Deletes only the companion directory named by this one workbook's own
/// document ID. Resetting a tutorial must also forget its event log, or the
/// next open would replay the learner's old transformations into the freshly
/// copied source. Nothing adjacent to the known workbook path is touched.
fn clear_tutorial_sidecar(path: &Path, document_id: &str) -> Result<(), String> {
    let paths =
        CollaborationPaths::for_document(path, document_id).map_err(|error| error.to_string())?;
    if paths.root.exists() {
        fs::remove_dir_all(&paths.root).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Creates missing tutorial workbooks or explicitly replaces their known
/// working copies. The reset branch does not clear the tutorial directory:
/// notes, exports, and any other files a learner put beside a workbook remain
/// theirs. It writes only the known bundled lesson paths and clears only their
/// own collaboration sidecars. Lesson assets follow the same create/reset
/// contract, so a learner can freely edit the tiny Excel sources and recover
/// the canonical generated data with Reset tutorials.
fn materialize_tutorial_documents(
    directory: &Path,
    reset: bool,
) -> Result<TutorialLibrary, String> {
    for tutorial in BUNDLED_TUTORIALS {
        let destination = directory.join(tutorial.relative_path);
        if destination.exists() && !destination.is_file() {
            return Err(format!(
                "Cannot create tutorial workbook because this is not a file: {}",
                destination.display()
            ));
        }
        let parent = destination.parent().ok_or_else(|| {
            format!(
                "Tutorial workbook has no parent directory: {}",
                destination.display()
            )
        })?;
        if reset || !destination.exists() {
            let previous_document_id = reset.then(|| document_id_at(&destination)).flatten();
            if let Some(document_id) = previous_document_id.as_deref() {
                clear_tutorial_sidecar(&destination, document_id)?;
            }
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            fs::write(&destination, tutorial.contents).map_err(|error| error.to_string())?;

            // The bundled document identity is normally the same as the previous
            // copy. Clearing it after the write handles a corrupt or manually
            // re-identified working copy without broadening reset beyond this path.
            if reset {
                let document_id = document_id_at(&destination).ok_or_else(|| {
                    format!(
                        "Bundled tutorial is not a readable FrameWork document: {}",
                        destination.display()
                    )
                })?;
                if previous_document_id.as_deref() != Some(document_id.as_str()) {
                    clear_tutorial_sidecar(&destination, &document_id)?;
                }
            }
        }
        for asset in tutorial.assets {
            let asset_path = parent.join(asset.relative_path);
            if asset_path.exists() && !asset_path.is_file() {
                return Err(format!(
                    "Cannot create tutorial asset because this is not a file: {}",
                    asset_path.display()
                ));
            }
            if reset || !asset_path.exists() {
                let asset_parent = asset_path.parent().ok_or_else(|| {
                    format!("Tutorial asset has no parent: {}", asset_path.display())
                })?;
                fs::create_dir_all(asset_parent).map_err(|error| error.to_string())?;
                fs::write(&asset_path, asset.contents).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(tutorial_library(directory))
}

#[tauri::command]
fn list_tutorial_documents(app: AppHandle) -> Result<TutorialLibrary, String> {
    Ok(tutorial_library(&tutorial_library_directory(&app)?))
}

#[tauri::command]
fn create_tutorial_documents(app: AppHandle) -> Result<TutorialLibrary, String> {
    materialize_tutorial_documents(&tutorial_library_directory(&app)?, false)
}

#[tauri::command]
fn reset_tutorial_documents(
    window: tauri::WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TutorialLibrary, String> {
    let library = materialize_tutorial_documents(&tutorial_library_directory(&app)?, true)?;

    // Reset may be pressed while one of these workbooks is open. Reload that
    // one session too, otherwise its next edit would save the old in-memory
    // model back over the reset copy. The normal document-changed event keeps
    // the webview in lockstep without pretending this was a newly opened file.
    let active_session = state.document_for(window.label())?;
    let active_path = active_session
        .lock()
        .map_err(|error| error.to_string())?
        .path
        .clone();
    if library
        .documents
        .iter()
        .any(|tutorial| Path::new(&tutorial.path) == active_path)
    {
        let (replacement, warning) = load_session(active_path)?;
        let document = replacement.store.view();
        *active_session.lock().map_err(|error| error.to_string())? = replacement;
        if let Some(warning) = warning {
            let _ = app.emit_to(window.label(), COLLABORATION_FAILED_EVENT, warning);
        }
        app.emit_to(window.label(), DOCUMENT_CHANGED_EVENT, document)
            .map_err(|error| error.to_string())?;
    }
    Ok(library)
}

#[tauri::command]
fn open_sample_document(
    file_name: String,
    window: tauri::WebviewWindow,
    app: AppHandle,
) -> Result<OpenedDocument, String> {
    let requested = Path::new(&file_name);
    if requested.is_absolute()
        || !requested
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
        || !is_framework_document_path(requested)
    {
        return Err("Sample document paths must stay inside the sample library and use .fw".into());
    }
    let sample_directory = sample_library_directory()?
        .ok_or_else(|| "No .framework-samples directory could be found".to_string())?;
    let source_path = sample_directory.join(requested);
    if !source_path.exists() {
        return Err(format!(
            "Sample document not found: {}",
            source_path.display()
        ));
    }
    let source_path = source_path
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let sample_directory = sample_directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !source_path.starts_with(&sample_directory) {
        return Err("Sample document path escapes the sample library".into());
    }
    let working_directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("sample-documents");
    fs::create_dir_all(&working_directory).map_err(|error| error.to_string())?;
    let stem = requested
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("sample");
    let working_path = working_directory.join(format!("{stem}-{}.fw", Uuid::new_v4()));
    copy_sample_document(&source_path, &working_path)?;
    open_document_at(&app, window.label(), working_path, false)
}

/// A sample document can own imported Parquet artifacts just like any other
/// document. Copying only its visible `.fw` file leaves those values behind;
/// `save_copy` copies every owned artifact and rewrites the paths relative to
/// the new working document while preserving the human title. The
/// UUID-suffixed working filename is an implementation detail, not the
/// sample's name or an undoable user action.
fn copy_sample_document(source_path: &Path, working_path: &Path) -> Result<(), String> {
    let mut sample = Store::load(source_path).map_err(|error| error.to_string())?;
    sample
        .save_copy(working_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn apply_operation(
    window: tauri::WebviewWindow,
    operation: Operation,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    apply_session_operation(&mut session, &state.writer_id, operation)
}

/// Imports a data file, either linked to it or holding its own copy.
///
/// `linked` is the whole difference between the two kinds of import, and it
/// is one field: a connector is what a refresh reads from, so a frame with
/// one tracks the file it came from and a frame without one has values
/// nothing will overwrite — which is also what makes those values editable.
/// There is no second import path, and no conversion needed later.
/// Asks for a data file and answers with its path, without importing it.
///
/// The list dialog needs a path to hand to the core, not a frame on the
/// canvas — reading one column out of a file is not the same act as bringing
/// the whole file in, and should not leave a frame behind.
#[tauri::command]
fn pick_data_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Data files", &["csv", "tsv", "parquet"])
        .pick_file()
        .map(|path| path.display().to_string())
}

#[tauri::command]
fn import_dataset_file(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    path: Option<String>,
    linked: bool,
    state: State<'_, AppState>,
) -> Result<Option<DocumentView>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Data files", &["csv", "tsv", "parquet"])
                .pick_file()
            else {
                return Ok(None);
            };
            path
        }
    };
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Imported data")
        .to_string();
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let artifact = stage_import_file(&session.path, session.store.document_id(), &path)?;
    let operation = Operation::ImportFrameFromArtifact {
        name,
        artifact,
        connector: linked.then(|| ConnectorRecipe::File {
            source_path: path.display().to_string(),
        }),
        x,
        y,
    };
    apply_session_operation(&mut session, &state.writer_id, operation).map(Some)
}

#[tauri::command]
async fn import_cli_source(
    window: tauri::WebviewWindow,
    app: AppHandle,
    input: CliSourceInput,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let source_label = input.source_label.trim().to_string();
    if source_label.is_empty() {
        return Err("A command source needs a name or address".into());
    }
    let query = input.query.and_then(|query| {
        let query = query.trim();
        (!query.is_empty()).then(|| query.to_string())
    });
    let connector = ConnectorRecipe::Cli {
        profile_id: input.profile_id.clone(),
        source_label: source_label.clone(),
        query,
    };
    let profile =
        cli_connectors::profile_by_id(&cli_connector_profiles_path(&app)?, &input.profile_id)?;
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
        stage_cli_artifact(&document_path, &document_id, &profile, &staged_connector)
    })
    .await
    .map_err(|error| error.to_string())??;
    let name = Path::new(&source_label)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&source_label)
        .to_string();
    let mut session = session.lock().map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::ImportFrameFromArtifact {
            name,
            artifact,
            connector: Some(connector),
            x: input.x,
            y: input.y,
        },
    )
}

#[tauri::command]
async fn import_database_source(
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
        &mut session,
        &state.writer_id,
        Operation::ImportFrameFromArtifact {
            name: source_name,
            artifact,
            connector: Some(connector),
            x: input.x,
            y: input.y,
        },
    )
}

/// Opens an Excel workbook as a catalogue of sheets, explicit Excel Tables,
/// and conservative rectangular-region suggestions. Choosing what becomes
/// data stays with the person; every suggestion still goes through preview.
#[tauri::command]
fn inspect_excel_workbook(path: Option<String>) -> Result<Option<ExcelWorkbookInfo>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Excel workbooks", &["xlsx", "xlsm"])
                .pick_file()
            else {
                return Ok(None);
            };
            path
        }
    };
    read_excel_workbook(&path)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_excel_range(
    path: String,
    sheet_name: String,
    cell_range: String,
    has_header: bool,
    limit: usize,
) -> Result<ExcelRangePreview, String> {
    read_excel_range_preview(
        Path::new(&path),
        &sheet_name,
        &cell_range,
        has_header,
        limit,
    )
    .map_err(|error| error.to_string())
}

/// Imports cached cell answers from one explicit Excel rectangle. The frame
/// has no connector: formulas and workbook layout are not a refresh contract,
/// and the resulting Parquet is an ordinary static import artifact.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn import_excel_range(
    window: tauri::WebviewWindow,
    path: String,
    sheet_name: String,
    cell_range: String,
    has_header: bool,
    name: String,
    x: f64,
    y: f64,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    let (artifact, _) = create_excel_range_artifact(
        Path::new(&path),
        &data_directory,
        &sheet_name,
        &cell_range,
        has_header,
    )
    .map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::ImportFrameFromArtifact {
            name,
            artifact,
            connector: None,
            x,
            y,
        },
    )
}

/// Imports a file as an ordinary source frame and immediately stacks it under
/// an existing frame through the same `Union` step the Wrangle editor writes.
///
/// This deliberately does not rewrite the selected frame. Replacing its
/// source or rows would make an append irreversible in the very situation
/// where someone is correcting a monthly import. Instead the old frame and
/// the new source remain separate, named inputs of a third derived frame.
/// That is also why changing the mapping later is just editing a normal
/// Stack frame step rather than entering a special append mode.
#[tauri::command]
fn import_and_append_dataset_file(
    window: tauri::WebviewWindow,
    frame_id: String,
    x: f64,
    y: f64,
    path: Option<String>,
    linked: bool,
    state: State<'_, AppState>,
) -> Result<Option<ImportAndAppendResult>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Data files", &["csv", "tsv", "parquet"])
                .pick_file()
            else {
                return Ok(None);
            };
            path
        }
    };
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let result = import_and_append_file(
        &mut session,
        &state.writer_id,
        &frame_id,
        &path,
        x,
        y,
        linked,
    )?;
    Ok(Some(result))
}

/// The non-dialog half of [`import_and_append_dataset_file`]. Keeping it
/// separate makes the important promise testable: incompatible headers are
/// rejected before the staged source becomes a visible frame or enters undo.
fn import_and_append_file(
    session: &mut DocumentSession,
    writer_id: &str,
    target_frame_id: &str,
    source_path: &Path,
    x: f64,
    y: f64,
    linked: bool,
) -> Result<ImportAndAppendResult, String> {
    let source_name = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Imported data")
        .to_string();
    let target_name = session
        .store
        .document()
        .frame(target_frame_id)
        .map_err(|error| error.to_string())?
        .name
        .clone();
    let artifact = stage_import_file(&session.path, session.store.document_id(), source_path)?;
    let import = || Operation::ImportFrameFromArtifact {
        name: source_name.clone(),
        artifact: artifact.clone(),
        connector: linked.then(|| ConnectorRecipe::File {
            source_path: source_path.display().to_string(),
        }),
        x,
        y,
    };
    let linked_frame = || Operation::AddLinkedFrame {
        // The selected frame is the top half of the stack. The imported
        // frame is named only by the following Union step, so its rows go
        // underneath it instead of a linked frame accidentally reading the
        // import and then attempting to union that frame with itself.
        source_frame_id: target_frame_id.to_string(),
        name: format!("{target_name} appended"),
        x: x + 72.0,
        y: y + 72.0,
    };

    // Validate all three ordinary edits in a clone before the first one is
    // journaled. In particular, Union rejects two frames without a common
    // header; importing that file onto the real canvas first would leave an
    // unexpected artifact card behind just to tell someone to rename it.
    let mut preflight = session.store.clone();
    let imported_id = apply_and_find_added_frame(&mut preflight, import())?;
    let appended_id = apply_and_find_added_frame(&mut preflight, linked_frame())?;
    preflight
        .apply(Operation::SetFramePipeline {
            frame_id: appended_id,
            steps: vec![framework_core::FrameStepInput::Union {
                frame_id: imported_id,
            }],
        })
        .map_err(|error| error.to_string())?;

    // These are intentionally the public, journaled operations rather than a
    // desktop-only mutation. Each is visible in history and collaboration as
    // the same import, linked-frame, and Stack-step actions it would have
    // been if someone had performed them one by one.
    let imported_id = apply_session_and_find_added_frame(session, writer_id, import())?;
    let appended_id = apply_session_and_find_added_frame(session, writer_id, linked_frame())?;
    let document = apply_session_operation(
        session,
        writer_id,
        Operation::SetFramePipeline {
            frame_id: appended_id.clone(),
            steps: vec![framework_core::FrameStepInput::Union {
                frame_id: imported_id,
            }],
        },
    )?;
    Ok(ImportAndAppendResult {
        document,
        appended_frame_id: appended_id,
    })
}

/// The regular add-frame operations mint ids inside core preparation. Rather
/// than replicate that id policy in the desktop shell, identify the one new
/// frame from the returned document. There can be only one in these actions;
/// treating any other answer as an error protects the subsequent Union from
/// pointing at an accidental frame.
fn apply_and_find_added_frame(store: &mut Store, operation: Operation) -> Result<String, String> {
    let before = frame_ids(&store.view().document);
    let after = store.apply(operation).map_err(|error| error.to_string())?;
    find_added_frame_id(&before, &after.document)
}

fn apply_session_and_find_added_frame(
    session: &mut DocumentSession,
    writer_id: &str,
    operation: Operation,
) -> Result<String, String> {
    let before = frame_ids(&session.store.view().document);
    let after = apply_session_operation(session, writer_id, operation)?;
    find_added_frame_id(&before, &after.document)
}

fn frame_ids(document: &Document) -> HashSet<String> {
    document
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Frame(frame) => Some(frame.id.clone()),
            _ => None,
        })
        .collect()
}

fn find_added_frame_id(before: &HashSet<String>, document: &Document) -> Result<String, String> {
    let added = document
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Frame(frame) if !before.contains(&frame.id) => Some(frame.id.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    match added.as_slice() {
        [id] => Ok(id.clone()),
        _ => Err("Import could not identify the frame it just created".into()),
    }
}

#[tauri::command]
async fn refresh_frame_connector(
    window: tauri::WebviewWindow,
    app: AppHandle,
    frame_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let (document_path, document_id, connector) = {
        let session = session.lock().map_err(|error| error.to_string())?;
        let connector = session
            .store
            .frame_connector(&frame_id)
            .cloned()
            .ok_or_else(|| "This frame has no refreshable connector".to_string())?;
        (
            session.path.clone(),
            session.store.document_id().to_string(),
            connector,
        )
    };
    let profile_path = cli_connector_profiles_path(&app)?;
    let database_path = database_connections_path(&app)?;
    let staged_connector = connector.clone();
    let artifact = tauri::async_runtime::spawn_blocking(move || match &staged_connector {
        ConnectorRecipe::File { source_path } => {
            stage_import_file(&document_path, &document_id, Path::new(source_path))
        }
        ConnectorRecipe::Cli { profile_id, .. } => {
            let profile = cli_connectors::profile_by_id(&profile_path, profile_id)?;
            stage_cli_artifact(&document_path, &document_id, &profile, &staged_connector)
        }
        ConnectorRecipe::Database { connection_id, .. } => {
            let connection = database_connections::by_id(&database_path, connection_id)?;
            stage_database_artifact(&document_path, &document_id, &connection, &staged_connector)
        }
    })
    .await
    .map_err(|error| error.to_string())??;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    if session.store.frame_connector(&frame_id) != Some(&connector) {
        return Err("The connector changed while its data was being read; refresh again".into());
    }
    let unchanged = session.store.frame_artifact_id(&frame_id) == Some(artifact.id.as_str());
    if unchanged {
        return Ok(session.store.view());
    }
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::RefreshFrameArtifact { frame_id, artifact },
    )
}

/// Points an imported frame at a different file.
///
/// `None` opens a picker; the result is `None` when the user cancels. The
/// core refuses a file whose columns do not match, so a mistaken pick fails
/// with an explanation rather than breaking every formula downstream.
#[tauri::command]
fn set_frame_source(
    window: tauri::WebviewWindow,
    frame_id: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<DocumentView>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Data files", &["csv", "tsv", "parquet"])
                .pick_file()
            else {
                return Ok(None);
            };
            path
        }
    };
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let artifact = stage_import_file(&session.path, session.store.document_id(), &path)?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::SetFrameSource {
            frame_id,
            artifact,
            connector: ConnectorRecipe::File {
                source_path: path.display().to_string(),
            },
        },
    )
    .map(Some)
}

/// Works out a value from live data and writes the answer down beside the
/// document, or refreshes the answer already written. The cheap alternative
/// to snapshotting a whole frame to get at one number.
#[tauri::command]
fn freeze_value(
    window: tauri::WebviewWindow,
    object_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    // The answer is computed here rather than in `prepare`, the way a
    // frame's snapshot is: reading live is this side's job, so every replica
    // is handed the same recorded number instead of each reading data it may
    // not have.
    let frozen = session
        .store
        .write_value_snapshot(&object_id, &data_directory)
        .map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::SetFrozenValue {
            object_id,
            frozen: Some(frozen),
        },
    )
}

/// Lets a frozen value go back to being worked out every time.
#[tauri::command]
fn thaw_value(
    window: tauri::WebviewWindow,
    object_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::SetFrozenValue {
            object_id,
            frozen: None,
        },
    )
}

/// Caches a derived frame to a parquet snapshot beside the document, or
/// refreshes the snapshot it already has. The write happens first; the
/// operation that points the frame at it is journaled like any other edit.
#[tauri::command]
fn materialize_frame(
    window: tauri::WebviewWindow,
    frame_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    let artifact = session
        .store
        .write_frame_snapshot(&frame_id, &data_directory)
        .map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::SetFrameMaterialization { frame_id, artifact },
    )
}

/// What a document-wide refresh did, frame by frame.
///
/// Named rather than counted because the interesting cases are the ones a
/// number cannot describe: three refreshed and one failed, or nothing
/// refreshed at all because the only stale snapshot sits under a broken one.
#[derive(serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct SnapshotRefresh {
    document: DocumentView,
    /// The frames recomputed, in the order they were done.
    refreshed: Vec<String>,
    failures: Vec<SnapshotRefreshFailure>,
}

#[derive(serde::Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
struct SnapshotRefreshFailure {
    frame: String,
    error: String,
}

/// Recomputes every snapshot that has fallen behind, parents before the
/// frames that read from them.
///
/// A frame under one that just failed is skipped rather than refreshed:
/// computing it would read the snapshot that could not be updated, and
/// stamp the result with a fingerprint claiming it is current. Leaving it
/// stale is the honest outcome — it says what it is until the frame above
/// it can be fixed.
#[tauri::command]
fn refresh_stale_snapshots(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<SnapshotRefresh, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    let mut refreshed = Vec::new();
    let mut failures = Vec::new();
    for frame_id in session.store.snapshot_refresh_order() {
        if !session.store.snapshot_is_stale(&frame_id)
            || session.store.upstream_snapshot_is_stale(&frame_id)
        {
            continue;
        }
        let name = frame_name(&session, &frame_id);
        let outcome = session
            .store
            .write_frame_snapshot(&frame_id, &data_directory)
            .map_err(|error| error.to_string())
            .and_then(|artifact| {
                apply_session_operation(
                    &mut session,
                    &state.writer_id,
                    Operation::SetFrameMaterialization { frame_id, artifact },
                )
            });
        match outcome {
            Ok(_) => refreshed.push(name),
            Err(error) => failures.push(SnapshotRefreshFailure { frame: name, error }),
        }
    }
    Ok(SnapshotRefresh {
        document: session.store.view(),
        refreshed,
        failures,
    })
}

fn frame_name(session: &DocumentSession, frame_id: &str) -> String {
    session
        .store
        .document()
        .objects
        .iter()
        .find(|object| object.id() == frame_id)
        .map(|object| object.name().to_string())
        .unwrap_or_else(|| "This frame".into())
}

/// Makes a frame's current values the document's own data: writes them to a
/// parquet beside the document and cuts the frame loose from whatever it was
/// reading. The write happens first, like a snapshot's; the operation that
/// points the frame at it is journaled like any other edit.
#[tauri::command]
fn adopt_frame_rows(
    window: tauri::WebviewWindow,
    frame_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    let artifact = session
        .store
        .write_owned_frame_data(&frame_id, &data_directory)
        .map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::AdoptFrameRows { frame_id, artifact },
    )
}

/// Writes a frame's current values to a parquet and adds them as a second,
/// frozen frame, leaving the original exactly as it was.
///
/// The other half of taking ownership, and the safer half. Ownership changes
/// what a frame *is* — the live one stops being live — which is the wrong
/// trade when the connection is the point and you only wanted this quarter's
/// numbers held still. A copy has no connector and no chain, so nothing will
/// ever move it, and it is editable for the same reason.
///
/// It needs no operation of its own: an artifact plus an import is exactly
/// what this is, and the import path already knows how to read a parquet
/// into a frame.
#[tauri::command]
fn freeze_frame_copy(
    window: tauri::WebviewWindow,
    frame_id: String,
    x: f64,
    y: f64,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    let artifact = session
        .store
        .write_owned_frame_data(&frame_id, &data_directory)
        .map_err(|error| error.to_string())?;
    let name = format!("{} (frozen)", frame_name(&session, &frame_id));
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::ImportFrameFromArtifact {
            name,
            artifact,
            connector: None,
            x,
            y,
        },
    )
}

/// Cuts every outside dependency the document has, in one edit.
///
/// What you do before sending a document to somebody: no connector is left
/// pointing at a file that exists on this machine only, and any frame that
/// was reading a path directly is given data of its own. Afterwards the
/// document and its sidecar are the whole of it.
#[tauri::command]
fn package_document(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    // A frame with a connector already has its own copy and only needs the
    // connector dropped, which the operation works out for itself. One that
    // reads a path directly has no copy at all, so it is written one here.
    let unowned = session
        .store
        .document()
        .objects
        .iter()
        .filter_map(|object| match object {
            DataObject::Frame(frame) if frame.source_file.is_some() && frame.artifact.is_none() => {
                Some(frame.id.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut adopted = Vec::new();
    for frame_id in unowned {
        let artifact = session
            .store
            .write_owned_frame_data(&frame_id, &data_directory)
            .map_err(|error| error.to_string())?;
        adopted.push((frame_id, artifact));
    }
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::PackageDocument { adopted },
    )
}

/// Deletes the data files nothing points at any more.
///
/// Not journaled and not undoable, because it changes nothing about the
/// document — it removes versions of files that were already unreachable
/// from it, from its history, and from any event still waiting to be merged.
#[tauri::command]
fn compact_document_data(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<ArtifactSweep, String> {
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    let data_directory =
        CollaborationPaths::for_document(&session.path, session.store.document_id())
            .map_err(|error| error.to_string())?
            .root
            .join("data");
    session
        .store
        .collect_unreferenced_artifacts(&session.journal, &data_directory)
        .map_err(|error| error.to_string())
}

/// Drops a frame's snapshot so it reads live again.
#[tauri::command]
fn clear_frame_materialization(
    window: tauri::WebviewWindow,
    frame_id: String,
    state: State<'_, AppState>,
) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    apply_session_operation(
        &mut session,
        &state.writer_id,
        Operation::ClearFrameMaterialization { frame_id },
    )
}

fn stage_import_file(
    document_path: &Path,
    document_id: &str,
    source_path: &Path,
) -> Result<DataArtifact, String> {
    let data_directory = CollaborationPaths::for_document(document_path, document_id)
        .map_err(|error| error.to_string())?
        .root
        .join("data");
    create_data_artifact(source_path, &data_directory).map_err(|error| error.to_string())
}

fn stage_cli_artifact(
    document_path: &Path,
    document_id: &str,
    profile: &cli_connectors::CliConnectorProfile,
    connector: &ConnectorRecipe,
) -> Result<DataArtifact, String> {
    let output = cli_connectors::run_profile(profile, connector)?;
    let mut artifact = stage_import_file(document_path, document_id, &output.path)?;
    // The temporary filename is an implementation detail. Lineage and error
    // messages name the source the person configured, not `output.csv`.
    artifact.source_name = connector.source_name();
    Ok(artifact)
}

fn stage_database_artifact(
    document_path: &Path,
    document_id: &str,
    connection: &database_connections::DatabaseConnection,
    connector: &ConnectorRecipe,
) -> Result<DataArtifact, String> {
    let ConnectorRecipe::Database { query, .. } = connector else {
        return Err("That source is not a database connector".into());
    };
    let output = database_connections::run_query(connection, query)?;
    let mut artifact = stage_import_file(document_path, document_id, &output.path)?;
    artifact.source_name = connector.source_name();
    Ok(artifact)
}

#[tauri::command]
fn export_frame_csv(
    window: tauri::WebviewWindow,
    frame_id: String,
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            let suggested = {
                let session = state.document_for(window.label())?;
                let session = session.lock().map_err(|error| error.to_string())?;
                session
                    .store
                    .view()
                    .document
                    .objects
                    .iter()
                    .find(|object| object.id() == frame_id)
                    .map(|object| format!("{}.csv", object.name()))
                    .unwrap_or_else(|| "frame.csv".into())
            };
            let Some(mut path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name(suggested)
                .save_file()
            else {
                return Ok(None);
            };
            if path.extension().is_none() {
                path.set_extension("csv");
            }
            path
        }
    };
    let session = state.document_for(window.label())?;
    let session = session.lock().map_err(|error| error.to_string())?;
    session
        .store
        .export_frame_csv(&frame_id, &path)
        .map_err(|error| error.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn export_document_excel(
    window: tauri::WebviewWindow,
    frame_ids: Vec<String>,
    path: Option<String>,
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
        .export_excel(&frame_ids, &path)
        .map_err(|error| error.to_string())?;
    Ok(Some(path.display().to_string()))
}

fn apply_session_operation(
    session: &mut DocumentSession,
    writer_id: &str,
    operation: Operation,
) -> Result<DocumentView, String> {
    let event = session
        .store
        .prepare_event(writer_id, operation)
        .map_err(|error| error.to_string())?;
    session
        .journal
        .append(&event)
        .map_err(|error| error.to_string())?;
    let view = session
        .store
        .apply_event(&event)
        .map_err(|error| error.to_string())?;
    persist_session(session)?;
    Ok(view)
}

#[tauri::command]
fn undo(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let view = session.store.undo();
    persist_session(&mut session)?;
    Ok(view)
}

#[tauri::command]
fn redo(window: tauri::WebviewWindow, state: State<'_, AppState>) -> Result<DocumentView, String> {
    let session = state.document_for(window.label())?;
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let view = session.store.redo();
    persist_session(&mut session)?;
    Ok(view)
}

/// Greys the menu's Undo and Redo out when there is nothing behind or ahead.
///
/// The webview drives this rather than the store, because the store is asked
/// for a document view by a dozen paths and every one of them would have to
/// remember to report. The view it hands back already carries the two flags,
/// so the window pushes them once, wherever a new one arrives.
#[tauri::command]
fn set_history_menu_state(
    window: tauri::WebviewWindow,
    app: AppHandle,
    can_undo: bool,
    can_redo: bool,
) {
    // e2e builds attach no menu, so the items are simply not in managed
    // state; a build with nothing to grey out has nothing to do here.
    if window.is_focused().unwrap_or(false)
        && let Some(history) = app.try_state::<menu::HistoryMenuItems<tauri::Wry>>()
    {
        history.set(can_undo, can_redo);
    }
}

fn build_document_window(
    app: &AppHandle,
    session: DocumentSession,
    started_blank: bool,
) -> Result<tauri::WebviewWindow, String> {
    let label = format!("document-{}", Uuid::new_v4());
    let title = format!("{} — FrameWork", session.store.view().document.name);
    app.state::<AppState>()
        .sessions
        .lock()
        .map_err(|error| error.to_string())?
        .insert(
            label.clone(),
            WindowSession {
                document: Arc::new(Mutex::new(session)),
                started_blank,
            },
        );

    let built =
        tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App("index.html".into()))
            .title(title)
            .inner_size(1440.0, 900.0)
            .min_inner_size(980.0, 640.0)
            .build();
    if built.is_err()
        && let Ok(mut sessions) = app.state::<AppState>().sessions.lock()
    {
        sessions.remove(&label);
    }
    built.map_err(|error| error.to_string())
}

#[tauri::command]
async fn new_window(app: AppHandle) -> Result<(), String> {
    let session = scratch_session()?;
    build_document_window(&app, session, false)?;
    Ok(())
}

fn open_document_window(app: &AppHandle, path: PathBuf) -> Result<(), String> {
    if !is_framework_document_path(&path) {
        return Err("FrameWork documents must use the .fw extension".into());
    }

    // A file is one editing session. Asking for it twice raises the existing
    // window instead of installing a second in-memory writer over the same
    // snapshot and leaving the two windows to overwrite each other.
    if focus_window_for_path(app, &path, None)? {
        return Ok(());
    }

    let (session, warning) = load_session(path)?;
    let window = build_document_window(app, session, false)?;
    if let Some(warning) = warning {
        let label = window.label().to_string();
        let handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let _ = handle.emit_to(label, COLLABORATION_FAILED_EVENT, warning);
        });
    }
    Ok(())
}

fn focus_window_for_path(
    app: &AppHandle,
    path: &Path,
    except_label: Option<&str>,
) -> Result<bool, String> {
    let open_windows = {
        let state = app.state::<AppState>();
        let sessions = state.sessions.lock().map_err(|error| error.to_string())?;
        sessions
            .iter()
            .filter(|(label, _)| Some(label.as_str()) != except_label)
            .map(|(label, session)| (label.clone(), Arc::clone(&session.document)))
            .collect::<Vec<_>>()
    };
    for (label, session) in open_windows {
        if session.lock().map_err(|error| error.to_string())?.path == path {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn open_document_at(
    app: &AppHandle,
    window_label: &str,
    path: PathBuf,
    emit_opened_event: bool,
) -> Result<OpenedDocument, String> {
    if !is_framework_document_path(&path) {
        return Err("FrameWork documents must use the .fw extension".into());
    }
    if focus_window_for_path(app, &path, Some(window_label))? {
        return Err("That document is already open in another window".into());
    }
    let mut store = Store::load(&path).map_err(|error| error.to_string())?;
    let journal =
        EventJournal::open(&path, &store.view().document.id).map_err(|error| error.to_string())?;
    let merge = journal
        .merge_into(&mut store)
        .map_err(|error| error.to_string())?;
    if merge.applied > 0 {
        store.save(&path).map_err(|error| error.to_string())?;
    }
    let payload = OpenedDocument {
        document: store.view(),
        path: path.display().to_string(),
    };
    let title = format!("{} — FrameWork", payload.document.document.name);

    let state = app.state::<AppState>();
    state.replace_document(
        window_label,
        DocumentSession {
            store,
            path,
            journal,
            snapshot_dirty: false,
            scratch: false,
        },
    )?;

    if let Some(window) = app.get_webview_window(window_label) {
        let _ = window.set_title(&title);
        let _ = window.show();
        let _ = window.set_focus();
    }
    if emit_opened_event {
        app.emit_to(window_label, DOCUMENT_OPENED_EVENT, &payload)
            .map_err(|error| error.to_string())?;
    }
    Ok(payload)
}

fn try_open_from_arguments<I>(app: &AppHandle, arguments: I, working_directory: &Path)
where
    I: IntoIterator<Item = OsString>,
{
    let Some(path) = document_path_from_arguments(arguments, working_directory) else {
        return;
    };
    if let Err(error) = open_document_window(app, path) {
        let _ = app.emit(DOCUMENT_OPEN_FAILED_EVENT, error);
    }
}

fn document_path_from_arguments<I>(arguments: I, working_directory: &Path) -> Option<PathBuf>
where
    I: IntoIterator<Item = OsString>,
{
    arguments.into_iter().find_map(|argument| {
        let path = PathBuf::from(argument);
        if !is_framework_document_path(&path) {
            return None;
        }
        Some(if path.is_absolute() {
            path
        } else {
            working_directory.join(path)
        })
    })
}

/// The document this launch starts on, and whether it chose that document
/// itself.
///
/// A path on the command line is an instruction and is followed. Without
/// one, the launch starts on an empty scratch document in a fresh temporary
/// directory and lets the Data library ask which document the user actually
/// wants.
///
/// Two alternatives were considered and rejected. Reopening the last
/// document automatically puts an unattended writer on real work: under
/// `tauri dev` every Rust edit relaunches the app, which is how a Save As
/// lands on the wrong file. Keeping one scratch document in the application
/// data directory is the same trap in miniature — whatever was left on it
/// yesterday is what greets you today, which is exactly how every launch
/// used to open in the Commerce playground. A per-launch temporary
/// directory has neither problem, and the demo becomes a sample document to
/// open on purpose.
fn initial_session() -> Result<(DocumentSession, bool, Option<String>), String> {
    let working_directory = env::current_dir().map_err(|error| error.to_string())?;
    if let Some(requested_path) = document_path_from_arguments(env::args_os(), &working_directory) {
        if requested_path.exists() {
            let (session, warning) = load_session(requested_path)?;
            return Ok((session, false, warning));
        }
        return Ok((blank_session(requested_path, false)?, false, None));
    }

    Ok((scratch_session()?, true, None))
}

/// An unsaved window still writes every edit immediately; its private
/// temporary directory is the disk home until Save As gives it a visible one.
fn scratch_session() -> Result<DocumentSession, String> {
    // A directory of its own, so the `.framework` sidecar the save writes
    // beside the document lands in the throwaway too, independently for
    // every open window.
    let scratch_directory = env::temp_dir().join(format!("framework-untitled-{}", Uuid::new_v4()));
    fs::create_dir_all(&scratch_directory).map_err(|error| error.to_string())?;
    blank_session(scratch_directory.join(SCRATCH_DOCUMENT_NAME), true)
}

/// A brand-new empty document at `path`.
///
/// The document ID is new every time, which is part of what keeps the canvas
/// empty: an ID carried over would pick its own event journal back up on
/// load and replay yesterday's edits into today's blank canvas.
fn blank_session(path: PathBuf, scratch: bool) -> Result<DocumentSession, String> {
    let store = Store::new(Document::blank(BLANK_DOCUMENT_TITLE));
    store.save(&path).map_err(|error| error.to_string())?;
    let journal =
        EventJournal::open(&path, &store.view().document.id).map_err(|error| error.to_string())?;
    Ok(DocumentSession {
        store,
        path,
        journal,
        snapshot_dirty: false,
        scratch,
    })
}

/// Opens a document, returning it alongside any collaboration problem that
/// did not stop it from opening.
///
/// The snapshot is what the local user's work lives in; the journal only
/// carries other writers' edits. A journal this build cannot replay -- one
/// written by a newer version, or by an older one whose operations have
/// since been renamed -- must therefore not keep the document shut. The
/// background watcher has always treated a failed merge this way (see
/// `watch_collaboration`); startup now agrees with it, instead of being the
/// one moment where the same failure is fatal.
fn load_session(path: PathBuf) -> Result<(DocumentSession, Option<String>), String> {
    let mut store = Store::load(&path).map_err(|error| error.to_string())?;
    let journal =
        EventJournal::open(&path, &store.view().document.id).map_err(|error| error.to_string())?;
    let mut warning = None;
    match journal.merge_into(&mut store) {
        Ok(merge) if merge.applied > 0 => {
            store.save(&path).map_err(|error| error.to_string())?;
        }
        Ok(_) => {}
        Err(error) => warning = Some(error.to_string()),
    }
    Ok((
        DocumentSession {
            store,
            path,
            journal,
            snapshot_dirty: false,
            scratch: false,
        },
        warning,
    ))
}

fn persist_session(session: &mut DocumentSession) -> Result<(), String> {
    session.snapshot_dirty = true;
    session
        .store
        .save(&session.path)
        .map_err(|error| error.to_string())?;
    session.snapshot_dirty = false;
    Ok(())
}

fn load_or_create_writer_id(app_data_directory: &Path) -> Result<String, String> {
    fs::create_dir_all(app_data_directory).map_err(|error| error.to_string())?;
    let path = app_data_directory.join(WRITER_ID_NAME);
    if path.exists() {
        let writer_id = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        return Uuid::parse_str(writer_id.trim())
            .map(|value| value.to_string())
            .map_err(|_| format!("{} does not contain a valid writer ID", path.display()));
    }
    let writer_id = Uuid::new_v4().to_string();
    fs::write(&path, format!("{writer_id}\n")).map_err(|error| error.to_string())?;
    Ok(writer_id)
}

fn rescan_collaboration(
    session: &Arc<Mutex<DocumentSession>>,
) -> Result<Option<DocumentView>, String> {
    let mut session = session.lock().map_err(|error| error.to_string())?;
    let journal = session.journal.clone();
    let merge = journal
        .merge_into(&mut session.store)
        .map_err(|error| error.to_string())?;
    let should_emit = merge.applied > 0 || session.snapshot_dirty;
    if !should_emit {
        return Ok(None);
    }
    persist_session(&mut session)?;
    Ok(Some(session.store.view()))
}

fn watch_collaboration(app: AppHandle) {
    thread::spawn(move || {
        let mut last_errors = HashMap::<String, String>::new();
        loop {
            thread::sleep(Duration::from_millis(750));
            let sessions = match app.state::<AppState>().sessions.lock() {
                Ok(sessions) => sessions
                    .iter()
                    .map(|(label, session)| (label.clone(), Arc::clone(&session.document)))
                    .collect::<Vec<_>>(),
                Err(_) => continue,
            };
            for (label, session) in sessions {
                match rescan_collaboration(&session) {
                    Ok(Some(document)) => {
                        last_errors.remove(&label);
                        let _ = app.emit_to(&label, DOCUMENT_CHANGED_EVENT, document);
                    }
                    Ok(None) => {
                        last_errors.remove(&label);
                    }
                    Err(error) if last_errors.get(&label) != Some(&error) => {
                        let _ = app.emit_to(&label, COLLABORATION_FAILED_EVENT, &error);
                        last_errors.insert(label, error);
                    }
                    Err(_) => {}
                }
            }
        }
    });
}

fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let Some(state) = window.try_state::<AppState>() else {
        return;
    };
    match event {
        tauri::WindowEvent::Destroyed => {
            if let Ok(mut sessions) = state.sessions.lock() {
                sessions.remove(window.label());
            }
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

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // The wdio plugin's commands are invoked from the webview, so document
    // windows need their permission only in the build that contains it.
    #[cfg(feature = "e2e")]
    app.add_capability(
        r#"{
          "identifier": "e2e-wdio",
          "description": "WebDriver harness access to the wdio plugin's commands.",
          "windows": ["main", "document-*"],
          "permissions": ["wdio:default"]
        }"#,
    )?;
    let app_data_directory = app.path().app_data_dir()?;
    let (session, started_blank, warning) = initial_session().map_err(std::io::Error::other)?;
    let writer_id = load_or_create_writer_id(&app_data_directory).map_err(std::io::Error::other)?;
    let title = format!("{} — FrameWork", session.store.view().document.name);
    app.manage(AppState {
        sessions: Mutex::new(HashMap::from([(
            "main".to_string(),
            WindowSession {
                document: Arc::new(Mutex::new(session)),
                started_blank,
            },
        )])),
        writer_id,
    });
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&title);
    }
    if let Some(warning) = warning {
        // The window is not listening yet, so let it mount first.
        let handle = app.handle().clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let _ = handle.emit_to("main", COLLABORATION_FAILED_EVENT, &warning);
        });
    }
    watch_collaboration(app.handle().clone());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // The e2e harness talks W3C WebDriver to a server the first plugin runs
    // inside the app, because macOS has no external driver for WKWebView. It
    // is an HTTP server on 127.0.0.1, listening only when the harness sets
    // TAURI_WEBDRIVER_PORT. The second plugin is the harness's command
    // surface (window states, script eval); its permission is granted at
    // runtime in setup() below, so capabilities/ never names a plugin that
    // non-e2e builds do not compile. The `e2e` cargo feature keeps both out
    // of every build the harness itself did not ask for.
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_wdio::init());

    // Single-instance forwarding is keyed by the app identifier, which a test
    // build shares with a running `tauri dev` — so without this exclusion an
    // e2e launch would hand its arguments to whatever FrameWork window is
    // already open and exit, driving the wrong instance. Test builds are
    // deliberately many-instance.
    #[cfg(all(desktop, not(feature = "e2e")))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, arguments, cwd| {
        let app = app.clone();
        thread::spawn(move || {
            try_open_from_arguments(
                &app,
                arguments.into_iter().map(OsString::from),
                Path::new(&cwd),
            );
        });
    }));

    // The updater never acts on its own. It is registered so the webview can
    // ask — from Check for Updates, or once when a window opens — and it does
    // nothing until asked. It stays compiled into e2e builds on purpose:
    // capabilities/ is checked against the plugins that exist, so excluding it
    // here would make `updater:default` name a plugin the test build lacks.
    // The frontend is what declines to check under e2e, which is where the
    // decision belongs anyway. tauri_plugin_process is here only so a staged
    // update can relaunch the app.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    // The native menu owns accelerators like ⌘J and ⌘Z: macOS consumes the
    // key before the webview sees it, so the window's own keydown handler
    // defers whenever a menu exists. WebDriver, in turn, synthesizes DOM
    // events that NSMenu never sees — under a native menu, every menu-owned
    // accelerator is undrivable. So e2e builds run the app the way the
    // browser dev server does: no menu, in-page accelerators. That path
    // already exists and is worth testing; the menu wiring itself is native
    // and was never reachable from WebDriver anyway. The init script tells
    // the page which shell it got, because "running inside Tauri" stopped
    // being a proxy for "a menu exists" the moment this build was possible.
    #[cfg(not(feature = "e2e"))]
    let builder = builder
        .menu(menu::build)
        .on_menu_event(|app, event| menu::forward(app, event.id().0.as_str()));

    let builder = builder.on_window_event(handle_window_event);

    let app = builder
        .setup(setup_app)
        .invoke_handler(tauri::generate_handler![
            get_document,
            get_document_path,
            get_mcp_settings,
            set_mcp_enabled,
            should_open_library,
            new_window,
            get_frame_page,
            get_frame_summary,
            get_block_line_page,
            get_frame_query_plan,
            preview_frame_pipeline,
            sample_frame_step,
            frame_formula_values,
            dependency_graph,
            complete_formula,
            open_document,
            open_document_dialog,
            new_document_dialog,
            save_document_as_dialog,
            list_recent_documents,
            list_cli_connector_profiles,
            save_cli_connector_profile,
            list_database_connections,
            save_database_connection,
            list_sample_documents,
            open_sample_document,
            list_tutorial_documents,
            create_tutorial_documents,
            reset_tutorial_documents,
            apply_operation,
            import_dataset_file,
            import_cli_source,
            import_database_source,
            inspect_excel_workbook,
            preview_excel_range,
            import_excel_range,
            import_and_append_dataset_file,
            pick_data_file,
            refresh_frame_connector,
            set_frame_source,
            materialize_frame,
            freeze_value,
            thaw_value,
            adopt_frame_rows,
            freeze_frame_copy,
            package_document,
            compact_document_data,
            refresh_stale_snapshots,
            clear_frame_materialization,
            export_frame_csv,
            export_document_excel,
            undo,
            redo,
            set_history_menu_state
        ])
        .build(tauri::generate_context!())
        .expect("error while building FrameWork");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = event {
            let arguments = urls
                .into_iter()
                .filter_map(|url| url.to_file_path().ok())
                .map(PathBuf::into_os_string);
            let working_directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let app = app.clone();
            thread::spawn(move || {
                try_open_from_arguments(&app, arguments, &working_directory);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_cross_platform_document_arguments() {
        let cwd = Path::new("/shared/projects");
        let path = document_path_from_arguments(
            [OsString::from("framework"), OsString::from("Orders.FW")],
            cwd,
        )
        .unwrap();
        assert_eq!(path, cwd.join("Orders.FW"));
    }

    #[test]
    fn ignores_unrelated_startup_arguments() {
        assert!(
            document_path_from_arguments(
                [OsString::from("framework"), OsString::from("--verbose")],
                Path::new("."),
            )
            .is_none()
        );
    }

    #[test]
    fn finds_sample_library_from_a_nested_runtime_directory() {
        let root = env::temp_dir().join(format!("framework-sample-search-{}", Uuid::new_v4()));
        let runtime_directory = root.join("src-tauri/target/debug");
        let sample_directory = root.join(SAMPLE_DOCUMENT_DIRECTORY);
        fs::create_dir_all(&runtime_directory).unwrap();
        fs::create_dir_all(&sample_directory).unwrap();

        assert_eq!(
            find_sample_library_from(&runtime_directory),
            Some(sample_directory.canonicalize().unwrap())
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_sample_working_copy_brings_its_imported_data_and_keeps_its_title() {
        let root = env::temp_dir().join(format!("framework-sample-copy-{}", Uuid::new_v4()));
        let source_directory = root.join("library");
        let working_path = root.join("working/import-demo.fw");
        fs::create_dir_all(&source_directory).unwrap();
        let source_path = source_directory.join("excel-import-workbook.fw");
        let csv_path = source_directory.join("inventory.csv");
        fs::write(&csv_path, "SKU,On Hand\nSKU-101,12\n").unwrap();
        let mut sample = Store::new(Document::blank("Excel import workbook"));
        let data_directory = CollaborationPaths::for_document(&source_path, sample.document_id())
            .unwrap()
            .root
            .join("data");
        let artifact = create_data_artifact(&csv_path, &data_directory).unwrap();
        sample
            .apply(Operation::ImportFrameFromArtifact {
                name: "Inventory".into(),
                artifact,
                connector: None,
                x: 0.0,
                y: 0.0,
            })
            .unwrap();
        let frame_id = sample
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap();
        sample.save(&source_path).unwrap();

        copy_sample_document(&source_path, &working_path).unwrap();
        fs::remove_dir_all(&source_directory).unwrap();

        let copied = Store::load(&working_path).unwrap();
        assert_eq!(copied.document().name, "Excel import workbook");
        let page = copied.get_frame_page(&frame_id, 0, 10).unwrap();
        assert_eq!(page.rows[0], ["SKU-101", "12"]);
        fs::remove_dir_all(root).unwrap();
    }

    /// The scratch canvas a bare launch lands on must actually be blank.
    ///
    /// It regressed once already in the other direction: seeding the default
    /// document from `Document::demo()` and then loading that file on every
    /// later launch meant the app always opened in the Commerce playground.
    #[test]
    fn a_scratch_session_is_empty_and_shares_nothing_with_the_last_one() {
        let root = env::temp_dir().join(format!("framework-scratch-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join(SCRATCH_DOCUMENT_NAME);

        let first = blank_session(path.clone(), true).unwrap();
        let first_document = first.store.view().document;
        assert!(first_document.objects.is_empty());
        assert!(first_document.views.is_empty());
        assert_eq!(first_document.name, BLANK_DOCUMENT_TITLE);
        assert!(first.scratch);

        // A second launch on the same path carries nothing over -- including
        // the document ID, which is what keeps its event journal from
        // replaying the last session's edits into the blank canvas.
        let second = blank_session(path, true).unwrap();
        let second_document = second.store.view().document;
        assert!(second_document.objects.is_empty());
        assert_ne!(first_document.id, second_document.id);

        fs::remove_dir_all(root).unwrap();
    }

    /// A document named on the command line is opened, not replaced.
    #[test]
    fn a_requested_document_is_not_treated_as_scratch() {
        let root = env::temp_dir().join(format!("framework-requested-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("analysis.fw");
        Store::new(Document::blank("Analysis")).save(&path).unwrap();

        let (session, warning) = load_session(path.clone()).unwrap();

        assert!(!session.scratch, "an opened document has a home to report");
        assert_eq!(session.store.view().document.name, "Analysis");
        assert!(warning.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stages_imports_inside_the_saved_document_companion_directory() {
        let root = env::temp_dir().join(format!("framework-import-stage-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let document_path = root.join("analysis.fw");
        let source_path = root.join("orders.csv");
        fs::write(&source_path, "name,amount\nWidget,12\n").unwrap();
        let document_id = Uuid::new_v4().to_string();

        let staged = stage_import_file(&document_path, &document_id, &source_path).unwrap();

        let staged_path = Path::new(&staged.path);
        assert!(staged_path.starts_with(root.join(".framework").join(&document_id).join("data")));
        assert_eq!(
            staged_path.extension().and_then(|value| value.to_str()),
            Some("parquet")
        );
        assert_eq!(staged.row_count, 1);
        assert_eq!(staged.source_name, "orders.csv");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_and_append_preflights_headers_then_keeps_both_sources_in_lineage() {
        let root = env::temp_dir().join(format!("framework-import-append-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let document_path = root.join("analysis.fw");
        let mut session = blank_session(document_path, false).unwrap();
        let writer_id = Uuid::new_v4().to_string();
        apply_session_operation(
            &mut session,
            &writer_id,
            Operation::AddFrame {
                name: "Ledger".into(),
                grid: vec![
                    vec!["Account".into(), "Amount".into()],
                    vec!["Sales".into(), "12".into()],
                ],
                x: 0.0,
                y: 0.0,
            },
        )
        .unwrap();
        let ledger_id = session
            .store
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "Ledger" => Some(frame.id.clone()),
                _ => None,
            })
            .unwrap();
        let incompatible = root.join("unrelated.csv");
        fs::write(&incompatible, "Region,Units\nWest,2\n").unwrap();
        let before = session.store.view().document;
        let error = import_and_append_file(
            &mut session,
            &writer_id,
            &ledger_id,
            &incompatible,
            50.0,
            50.0,
            false,
        )
        .unwrap_err();
        assert!(error.contains("shares a name"));
        assert_eq!(session.store.view().document, before);

        let source = root.join("august.csv");
        fs::write(&source, "Account,Amount\nReturns,3\n").unwrap();
        let result = import_and_append_file(
            &mut session,
            &writer_id,
            &ledger_id,
            &source,
            50.0,
            50.0,
            true,
        )
        .unwrap();
        let source_frame = session
            .store
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                DataObject::Frame(frame) if frame.name == "august" => Some(frame),
                _ => None,
            })
            .unwrap();
        assert!(source_frame.connector.is_some());
        session
            .store
            .get_frame_page(&source_frame.id, 0, 10)
            .expect("imported source page");
        let appended = session
            .store
            .document()
            .frame(&result.appended_frame_id)
            .unwrap();
        assert_eq!(appended.name, "Ledger appended");
        assert_eq!(appended.derivation.as_ref().unwrap().steps.len(), 1);
        let page = session
            .store
            .get_frame_page(&result.appended_frame_id, 0, 10)
            .unwrap();
        assert_eq!(
            page.rows,
            vec![
                vec!["Sales".to_string(), "12".to_string()],
                vec!["Returns".to_string(), "3".to_string()]
            ]
        );
        assert_eq!(session.store.view().document.objects.len(), 3);
        // The command is one gesture, but uses the same three public edits a
        // person could have made themselves. Undo therefore exposes every
        // safe resting point rather than leaving a bespoke mutation behind.
        assert!(result.document.can_undo);
        assert_eq!(session.store.undo().document.objects.len(), 3);
        assert_eq!(session.store.undo().document.objects.len(), 2);
        assert_eq!(session.store.undo().document.objects.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tutorial_creation_preserves_existing_work_and_reset_replaces_only_known_copies() {
        let root = env::temp_dir().join(format!("framework-tutorials-{}", Uuid::new_v4()));
        let untouched_note = root.join("my-notes.txt");
        let untouched_workbook = root.join("my-own-workbook.fw");
        fs::create_dir_all(&root).unwrap();
        fs::write(&untouched_note, "Keep this note").unwrap();
        Store::new(Document::blank("My own workbook"))
            .save(&untouched_workbook)
            .unwrap();

        let initial = tutorial_library(&root);
        assert_eq!(initial.documents.len(), BUNDLED_TUTORIALS.len());
        assert!(initial.documents.iter().all(|tutorial| !tutorial.exists));

        let created = materialize_tutorial_documents(&root, false).unwrap();
        assert!(created.documents.iter().all(|tutorial| tutorial.exists));
        let first_path = PathBuf::from(&created.documents[0].path);
        assert_eq!(
            fs::read(&first_path).unwrap(),
            BUNDLED_TUTORIALS[0].contents
        );
        let excel_start = BUNDLED_TUTORIALS
            .iter()
            .find(|tutorial| {
                tutorial.lesson == "Importing an Excel workbook" && tutorial.kind == "Start"
            })
            .unwrap();
        let excel_parent = root
            .join(excel_start.relative_path)
            .parent()
            .unwrap()
            .to_path_buf();
        let excel_asset = &excel_start.assets[0];
        let excel_asset_path = excel_parent.join(excel_asset.relative_path);
        assert_eq!(fs::read(&excel_asset_path).unwrap(), excel_asset.contents);
        let excel_answer_path = created
            .documents
            .iter()
            .find(|tutorial| tutorial.title == "Importing an Excel workbook — Answer key")
            .map(|tutorial| PathBuf::from(&tutorial.path))
            .unwrap();
        let excel_answer = Store::load(&excel_answer_path).unwrap();
        let orders = excel_answer
            .document()
            .objects
            .iter()
            .find_map(|object| match object {
                framework_core::DataObject::Frame(frame) if frame.name == "Orders" => Some(frame),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            excel_answer
                .get_frame_page(&orders.id, 0, 50)
                .unwrap()
                .total_rows,
            20
        );

        // Create is safe to use repeatedly. A learner's current answer stays
        // intact until they explicitly choose Reset tutorials.
        fs::write(&first_path, "learner's changes").unwrap();
        materialize_tutorial_documents(&root, false).unwrap();
        assert_eq!(
            fs::read_to_string(&first_path).unwrap(),
            "learner's changes"
        );
        fs::write(&excel_asset_path, "learner changed the source workbook").unwrap();
        materialize_tutorial_documents(&root, false).unwrap();
        assert_eq!(
            fs::read_to_string(&excel_asset_path).unwrap(),
            "learner changed the source workbook"
        );

        // The exact sidecar is discarded too, so reopening the reset workbook
        // cannot merge an old event stream back into its canonical snapshot.
        let canonical_id = Store::load(&root.join(BUNDLED_TUTORIALS[1].relative_path))
            .unwrap()
            .document()
            .id
            .to_string();
        let sidecar = CollaborationPaths::for_document(
            &root.join(BUNDLED_TUTORIALS[1].relative_path),
            &canonical_id,
        )
        .unwrap();
        fs::create_dir_all(&sidecar.events).unwrap();
        fs::write(sidecar.events.join("left-behind.json"), "old event").unwrap();

        let reset = materialize_tutorial_documents(&root, true).unwrap();
        assert!(reset.documents.iter().all(|tutorial| tutorial.exists));
        assert_eq!(
            fs::read(&first_path).unwrap(),
            BUNDLED_TUTORIALS[0].contents
        );
        assert!(!sidecar.root.exists());
        assert_eq!(fs::read(&excel_asset_path).unwrap(), excel_asset.contents);
        assert_eq!(
            fs::read_to_string(&untouched_note).unwrap(),
            "Keep this note"
        );
        assert!(untouched_workbook.is_file());

        fs::remove_dir_all(root).unwrap();
    }
}
