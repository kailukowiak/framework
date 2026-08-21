import { invoke } from "@tauri-apps/api/core";
import type {
  CompletionResult,
  DocumentView,
  Operation,
  FrameStepInput,
} from "./types";

// ---------------------------------------------------------------------------
// The shapes commands answer with, generated from the Rust structs the same
// way as the document model in `types.ts`. Most live in `framework-core` and
// regenerate with the command named there; `SampleDocument`, `RecentDocument`
// and `SnapshotRefresh` are the desktop shell's own, defined next to their
// commands in `src-tauri/src/lib.rs`, and regenerate with
//
//     cargo test -p framework-desktop export_bindings
// ---------------------------------------------------------------------------

import type { ArtifactSweep } from "./bindings/ArtifactSweep";
import type { BlockLinePage } from "./bindings/BlockLinePage";
import type { DependencyKind } from "./bindings/DependencyKind";
import type { DependencyNode } from "./bindings/DependencyNode";
import type { PipelineSchema } from "./bindings/PipelineSchema";
import type { RecentDocument } from "./bindings/RecentDocument";
import type { SampleDocument } from "./bindings/SampleDocument";
import type { SnapshotRefresh } from "./bindings/SnapshotRefresh";
import type { StepSample } from "./bindings/StepSample";
import type { StepSchema } from "./bindings/StepSchema";
import type { FramePage } from "./bindings/FramePage";
import type { FrameQueryPlan } from "./bindings/FrameQueryPlan";
import type { FrameSummary } from "./bindings/FrameSummary";
import type { TutorialDocument } from "./bindings/TutorialDocument";
import type { TutorialLibrary } from "./bindings/TutorialLibrary";
import type { CliConnectorProfile } from "./bindings/CliConnectorProfile";
import type { CliConnectionKind } from "./bindings/CliConnectionKind";
import type { CliOutputFormat } from "./bindings/CliOutputFormat";
import type { DatabaseConnection } from "./bindings/DatabaseConnection";

export type {
  ArtifactSweep,
  BlockLinePage,
  DependencyKind,
  DependencyNode,
  PipelineSchema,
  RecentDocument,
  SampleDocument,
  SnapshotRefresh,
  StepSample,
  StepSchema,
  FramePage,
  FrameQueryPlan,
  FrameSummary,
  TutorialDocument,
  TutorialLibrary,
  CliConnectorProfile,
  CliConnectionKind,
  CliOutputFormat,
  DatabaseConnection,
};

export async function getDocument(): Promise<DocumentView> {
  return invoke("get_document");
}

export async function getDocumentPath(): Promise<string | null> {
  return invoke("get_document_path");
}

export async function getBlockLinePage(
  blockId: string,
  lineId: string,
  offset: number,
  limit: number
): Promise<BlockLinePage> {
  return invoke("get_block_line_page", { blockId, lineId, offset, limit });
}

export interface McpSettings {
  enabled: boolean;
  executablePath: string | null;
}

/** Machine-local permission and the server executable this build can find. */
export async function getMcpSettings(): Promise<McpSettings> {
  return invoke("get_mcp_settings");
}

/**
 * Allows or refuses subsequent requests from the separately launched stdio
 * server. The server checks this preference for every tool call, so an
 * already-connected client loses access as soon as the switch is turned off.
 */
export async function setMcpEnabled(enabled: boolean): Promise<McpSettings> {
  return invoke("set_mcp_enabled", { enabled });
}

/**
 * Whether this launch picked its own document rather than being handed one.
 * True means the canvas behind the dialog is the blank scratch document, so
 * the Data library is what the window should actually be showing.
 */
export async function shouldOpenLibrary(): Promise<boolean> {
  return invoke("should_open_library");
}

/** Opens an independent blank workbook window. */
export async function newWindow(): Promise<void> {
  return invoke("new_window");
}

export async function openDocument(
  path: string
): Promise<{ document: DocumentView; path: string }> {
  return invoke("open_document", { path });
}

export async function openDocumentDialog(): Promise<{
  document: DocumentView;
  path: string;
} | null> {
  return invoke("open_document_dialog");
}

export async function newDocumentDialog(
  name: string
): Promise<{ document: DocumentView; path: string } | null> {
  return invoke("new_document_dialog", { name });
}

export async function listRecentDocuments(): Promise<RecentDocument[]> {
  return invoke("list_recent_documents");
}

export async function listCliConnectorProfiles(): Promise<CliConnectorProfile[]> {
  return invoke("list_cli_connector_profiles");
}

export async function saveCliConnectorProfile(
  profile: CliConnectorProfile
): Promise<CliConnectorProfile> {
  return invoke("save_cli_connector_profile", { profile });
}

export interface CliSourceInput {
  profileId: string;
  sourceLabel: string;
  query: string | null;
}

export async function importCliSource(
  position: { x: number; y: number },
  source: CliSourceInput
): Promise<DocumentView> {
  return invoke("import_cli_source", { input: { ...position, ...source } });
}

export async function listDatabaseConnections(): Promise<DatabaseConnection[]> {
  return invoke("list_database_connections");
}

export async function saveDatabaseConnection(
  connection: DatabaseConnection
): Promise<DatabaseConnection> {
  return invoke("save_database_connection", { connection });
}

export interface DatabaseSourceInput {
  connectionId: string;
  sourceName: string;
  query: string;
}

export async function importDatabaseSource(
  position: { x: number; y: number },
  source: DatabaseSourceInput
): Promise<DocumentView> {
  return invoke("import_database_source", { input: { ...position, ...source } });
}

export async function saveDocumentAsDialog(): Promise<{
  document: DocumentView;
  path: string;
} | null> {
  return invoke("save_document_as_dialog");
}

/**
 * Imports a data file, either linked to it or holding its own copy.
 *
 * `linked` keeps a connector, so the frame can be refreshed and a refresh
 * replaces its values; without one the values are the document's own and can
 * be edited. Resolves to `null` when the file picker was cancelled.
 */
export async function importDatasetFile(
  position: { x: number; y: number },
  linked: boolean
): Promise<DocumentView | null> {
  return invoke("import_dataset_file", {
    x: position.x,
    y: position.y,
    linked,
  });
}

export interface ExcelTableInfo {
  name: string;
  sheetName: string;
  cellRange: string;
}

export interface ExcelSheetInfo {
  name: string;
  usedRange: string | null;
  rowCount: number;
  columnCount: number;
}

export interface ExcelRegionInfo {
  sheetName: string;
  cellRange: string;
  rowCount: number;
  columnCount: number;
}

export interface ExcelWorkbookInfo {
  path: string;
  fileName: string;
  sheets: ExcelSheetInfo[];
  tables: ExcelTableInfo[];
  suggestedRegions: ExcelRegionInfo[];
}

export interface ExcelRangePreview {
  columns: string[];
  rows: string[][];
  rowCount: number;
  formulaCellCount: number;
  errorCellCount: number;
}

/** Reads workbook structure only; resolves to null when its picker is cancelled. */
export async function inspectExcelWorkbook(
  path?: string
): Promise<ExcelWorkbookInfo | null> {
  return invoke("inspect_excel_workbook", { path: path ?? null });
}

/** Returns cached values from one explicit rectangle without importing it. */
export async function previewExcelRange(
  path: string,
  sheetName: string,
  cellRange: string,
  hasHeader: boolean,
  limit = 20
): Promise<ExcelRangePreview> {
  return invoke("preview_excel_range", {
    path,
    sheetName,
    cellRange,
    hasHeader,
    limit,
  });
}

/** Imports cached Excel answers as a static frame; formulas are never retained. */
export async function importExcelRange(
  workbook: Pick<ExcelWorkbookInfo, "path">,
  selection: {
    sheetName: string;
    cellRange: string;
    hasHeader: boolean;
    name: string;
  },
  position: { x: number; y: number }
): Promise<DocumentView> {
  return invoke("import_excel_range", {
    path: workbook.path,
    ...selection,
    ...position,
  });
}

export interface ImportAndAppendResult {
  document: DocumentView;
  /** The derived frame that contains the existing rows followed by the import. */
  appendedFrameId: string;
}

/**
 * Imports a file as a source frame, then makes a derived frame that stacks it
 * below `frameId`. The desktop command preflights the complete chain before
 * publishing any of its normal import/derive/pipeline operations, so a file
 * with no matching headers leaves the canvas alone.
 */
export async function importAndAppendDatasetFile(
  frameId: string,
  position: { x: number; y: number },
  linked: boolean
): Promise<ImportAndAppendResult | null> {
  return invoke("import_and_append_dataset_file", {
    frameId,
    x: position.x,
    y: position.y,
    linked,
  });
}

/**
 * Asks for a data file and answers with its path, importing nothing.
 *
 * For the cases that want a file to read from rather than a frame on the
 * canvas — a list taken out of one column, say. `null` when cancelled.
 */
export async function pickDataFile(): Promise<string | null> {
  return invoke("pick_data_file");
}

export async function refreshFrameConnector(frameId: string): Promise<DocumentView> {
  return invoke("refresh_frame_connector", { frameId });
}

/**
 * Points an imported frame at a different file, keeping the frame itself.
 *
 * Opens a picker when no path is given; resolves to null when the user
 * cancels. Rejects when the chosen file's columns do not match the ones the
 * frame already has — the column IDs downstream formulas hold have to keep
 * meaning what they meant.
 */
export async function setFrameSource(
  frameId: string,
  path?: string
): Promise<DocumentView | null> {
  return invoke("set_frame_source", { frameId, path: path ?? null });
}

/** Caches a derived frame to a snapshot, or refreshes the one it has. */
export async function materializeFrame(frameId: string): Promise<DocumentView> {
  return invoke("materialize_frame", { frameId });
}

/**
 * Works out a value from live data and writes the answer down — the cheap
 * alternative to snapshotting a whole frame to read one number out of it.
 * Refreshing is the same call again.
 */
export async function freezeValue(objectId: string): Promise<DocumentView> {
  return invoke("freeze_value", { objectId });
}

/** Lets a frozen value go back to being worked out every time. */
export async function thawValue(objectId: string): Promise<DocumentView> {
  return invoke("thaw_value", { objectId });
}

/**
 * Recomputes every snapshot that has fallen behind, parents first.
 *
 * A frame under one that failed is left alone rather than rebuilt from the
 * snapshot that could not be updated, so `refreshed.length` can be short of
 * what the canvas was showing as stale.
 */
export async function refreshStaleSnapshots(): Promise<SnapshotRefresh> {
  return invoke("refresh_stale_snapshots");
}

/**
 * Makes a frame's current values the document's own data.
 *
 * Writes them to a parquet beside the document and lets go of the chain,
 * connector, or snapshot they came from — after which they can be typed
 * into, because nothing is left that would overwrite them.
 */
export async function adoptFrameRows(frameId: string): Promise<DocumentView> {
  return invoke("adopt_frame_rows", { frameId });
}

/**
 * Adds a second frame holding this one's current values, frozen.
 *
 * The original is left alone — connector, chain and all. The copy has
 * neither, so nothing will move it and it can be edited.
 */
export async function freezeFrameCopy(
  frameId: string,
  x: number,
  y: number
): Promise<DocumentView> {
  return invoke("freeze_frame_copy", { frameId, x, y });
}

/**
 * Cuts every outside dependency the document has, in one edit.
 *
 * Drops every connector and gives data of its own to any frame that was
 * reading a path directly. Afterwards the document and its sidecar are the
 * whole of it — and every frame in it is editable.
 */
export async function packageDocument(): Promise<DocumentView> {
  return invoke("package_document");
}

/**
 * Deletes the data files nothing points at any more.
 *
 * Not an edit and not undoable: it removes versions already unreachable from
 * the document, from its undo history, and from any event still waiting to be
 * merged.
 */
export async function compactDocumentData(): Promise<ArtifactSweep> {
  return invoke("compact_document_data");
}

/** Drops a frame's snapshot so it reads live again. */
export async function clearFrameMaterialization(
  frameId: string
): Promise<DocumentView> {
  return invoke("clear_frame_materialization", { frameId });
}

export async function getFrameQueryPlan(frameId: string): Promise<FrameQueryPlan> {
  return invoke("get_frame_query_plan", { frameId });
}

/**
 * What the chain being drafted would produce, step by step.
 *
 * Answered from the query plan, so it costs no scan — and it is deliberately
 * about the *unsaved* draft: the editor asks what its current steps would do
 * before committing them. A chain that stops at a step it cannot work out is
 * an ordinary answer, with the schemas before that step intact.
 */
export async function previewFramePipeline(
  frameId: string,
  steps: FrameStepInput[]
): Promise<PipelineSchema> {
  return invoke("preview_frame_pipeline", { frameId, steps });
}

export async function getFramePage(
  frameId: string,
  offset: number,
  limit: number
): Promise<FramePage> {
  return invoke("get_frame_page", { frameId, offset, limit });
}

/**
 * Profiles every configured footer row in one scan of the displayed frame.
 * Kept separate from the document view so a visible profile on a large
 * import does not re-run after an unrelated canvas edit.
 */
export async function getFrameSummary(frameId: string): Promise<FrameSummary> {
  return invoke("get_frame_summary", { frameId });
}

/**
 * The distinct values a conditional-formatting formula produces, commonest
 * first — what a category rule's case list is filled from.
 *
 * Asked rather than worked out here because only the engine knows: the
 * formula may span columns, the rows may live in a file this process has
 * never read, and the frame's display filter decides which of them count.
 * The colors are chosen on this side, which is why this returns labels
 * rather than a rule.
 */
export async function frameFormulaValues(
  frameId: string,
  formula: string,
  limit: number
): Promise<string[]> {
  return invoke("frame_formula_values", { frameId, formula, limit });
}

/**
 * The first rows as they stand after one step of a chain being drafted.
 *
 * The one call in the step editor that runs the query. The limit is pushed
 * into the plan, so a step over millions of rows reads what it needs — but
 * it is still work, which is why the editor fetches it once rather than as
 * you type.
 */
export async function sampleFrameStep(
  frameId: string,
  steps: FrameStepInput[],
  stepIndex: number,
  limit: number
): Promise<StepSample> {
  return invoke("sample_frame_step", { frameId, steps, stepIndex, limit });
}

/**
 * What a value or result depends on, recursively, with each stop's current
 * value attached — the debug trace.
 *
 * A frame comes back as a leaf: its own wrangle chain is a different kind
 * of path, walked separately with `sampleFrameStep`.
 */
export async function dependencyGraph(objectId: string): Promise<DependencyNode> {
  return invoke("dependency_graph", { objectId });
}

/**
 * Type-aware completion for a formula.
 *
 * `scope` narrows it to a position in a chain being drafted: a step sees
 * what the steps before it leave behind, which after a summarize is not the
 * frame's own columns. Without it, completion answers about the frame.
 */
export async function completeFormula(
  frameId: string,
  formulaText: string,
  cursorPos: number,
  scope?: { steps: FrameStepInput[]; stepIndex: number }
): Promise<CompletionResult> {
  return invoke("complete_formula", {
    frameId,
    formulaText,
    cursorPos,
    steps: scope?.steps ?? null,
    stepIndex: scope?.stepIndex ?? null,
  });
}

export async function exportFrameCsv(frameId: string): Promise<string | null> {
  return invoke("export_frame_csv", { frameId });
}

export async function exportDocumentExcel(frameIds: string[]): Promise<string | null> {
  return invoke("export_document_excel", { frameIds });
}

export async function listSampleDocuments(): Promise<SampleDocument[]> {
  return invoke("list_sample_documents");
}

/** The visible working-copy location and the bundled tutorial workbook slots. */
export async function listTutorialDocuments(): Promise<TutorialLibrary> {
  return invoke("list_tutorial_documents");
}

/** Creates only tutorial workbooks that are missing; existing learner work stays put. */
export async function createTutorialDocuments(): Promise<TutorialLibrary> {
  return invoke("create_tutorial_documents");
}

/** Explicitly replaces the eight known tutorial copies and clears their own history. */
export async function resetTutorialDocuments(): Promise<TutorialLibrary> {
  return invoke("reset_tutorial_documents");
}

export async function openSampleDocument(
  fileName: string
): Promise<{ document: DocumentView; path: string | null }> {
  return invoke("open_sample_document", { fileName });
}

export async function applyOperation(operation: Operation): Promise<DocumentView> {
  return invoke("apply_operation", { operation });
}

export async function undo(): Promise<DocumentView> {
  return invoke("undo");
}

export async function redo(): Promise<DocumentView> {
  return invoke("redo");
}

/**
 * Greys out Edit ▸ Undo and Edit ▸ Redo when there is nothing to undo or redo.
 *
 * This is the affordance the header's two arrow buttons used to carry: a
 * disabled control is how you find out the history is empty. The menu can only
 * learn it from here, so every document view that arrives pushes it back.
 */
export async function setHistoryMenuState(
  canUndo: boolean,
  canRedo: boolean
): Promise<void> {
  return invoke("set_history_menu_state", { canUndo, canRedo });
}
