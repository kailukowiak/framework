import {
  ArrowDownToLine,
  BarChart3,
  Braces,
  ChevronRight,
  CircleAlert,
  Check,
  Copy,
  Database,
  FunctionSquare,
  FolderOpen,
  Frame,
  GitBranch,
  GitMerge,
  KeyRound,
  ListOrdered,
  Plus,
  SquareFunction,
  FolderInput,
  FolderOutput,
  FolderPlus,
  Table2 as FrameIcon,
  Trash2,
  Type,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { GridEditingMenu } from "./GridEditingMenu";
import { ContextMenuGroup, ContextMenuSurface } from "./ContextMenuSurface";
import { ColumnContextAuthoringActions } from "./ColumnContextAuthoringActions";
import { CanvasStatus } from "./CanvasStatus";
import { canvasFormulaPointerHandler } from "./CanvasFormulaPicking";
import {
  ColumnAuthoringDialogs,
  type RecurrenceState,
  type RunningCalculationState,
  type SequenceFillState,
} from "./ColumnAuthoringDialogs";
import { ReferenceHighlights } from "./ReferenceHighlights";
import { PreferencesDialog } from "./PreferencesDialog";
import { KeyboardShortcutsDialog } from "./KeyboardShortcutsDialog";
import { useApplicationMenu } from "./useApplicationMenu";
import { useFitViewToWindow } from "./useFitViewToWindow";
import { useCanvasNavigation } from "./useCanvasNavigation";
import { useThousandsSeparatorsPreference } from "./hooks/useThousandsSeparatorsPreference";
import { useInterfaceScalePreference } from "./hooks/useInterfaceScalePreference";
import { useMcpSettings } from "./hooks/useMcpSettings";
import {
  usePipelineColumnRequests,
  scopedPipelineRequest,
  type AddCalculatedColumnEditorRequest,
  type TransformColumnEditorRequest,
  type FilterColumnEditorRequest,
  type HidePipelineColumnEditorRequest,
  type RearrangeColumnsEditorRequest,
} from "./hooks/usePipelineColumnRequests";
import { useContextMenu } from "./hooks/useContextMenu";
import { useScratchwork } from "./hooks/useScratchwork";
import { useImportFlow } from "./hooks/useImportFlow";
import { useGridClipboard } from "./hooks/useGridClipboard";
import { useCanvasViewport } from "./hooks/useCanvasViewport";
import { useDocumentLifecycle } from "./hooks/useDocumentLifecycle";
import { useConnectorRefreshApproval } from "./hooks/useConnectorRefreshApproval";
import { ConnectorRefreshConfirmDialog } from "./ConnectorRefreshConfirmDialog";
import { useGridKeyboardNavigation } from "./hooks/useGridKeyboardNavigation";
import { useDocumentBootstrap } from "./hooks/useDocumentBootstrap";
import { FrameInspector } from "./FrameInspector";
import { JoinDialog } from "./JoinDialog";
import { DataSidebar } from "./DataSidebar";
import { LeftRail } from "./LeftRail";
import { defaultPlotSpec, viewHolding } from "./lib/canvasCards";
import { CanvasObject, LineageCords } from "./CanvasObject";
import { PlotInspector, ValueInspector } from "./PlotInspector";
import { NewDocumentDialog } from "./NewDocumentDialog";
import { DatasetDialog } from "./DatasetDialog";
import { ExcelImportDialog, type ExcelRangeSelection } from "./ExcelImportDialog";
import { InsertListDialog } from "./InsertListDialog";
import { ImportChoiceDialog } from "./ImportChoiceDialog";
import { ProjectPanel } from "./ProjectPanel";
import { BlockCard } from "./BlockCard";
import { scalarFormulaReferences } from "./ScalarCards";
import {
  ARROW_DIRECTIONS,
  NumberDisplayContext,
  activeTabObject,
  activeTabFrame,
  chainFilterCount,
  gridBoundsFor,
  gridCellAt,
  isTextEntryTarget,
  nextColumnName,
  resolveGridContext,
  tabObjects,
  visualGridPosition,
  type ContextMenuState,
  type FreezeCopyHandler,
  type GridFocus,
  type RenderedGrid,
  type SetFrameCachedHandler,
  type SetFrameSourceHandler,
  type TakeOwnershipHandler,
} from "./FrameGrid";
import { hasFrameTabDrag, readFrameTabDrag } from "./FrameViewTabs";
import {
  useActiveFormulaEditorCommands,
  useActiveFormulaEditorPresence,
} from "./ActiveFormulaEditor";
import { CellAwareFormulaBar } from "./CellAwareFormulaBar";
import type { CellFormulaRequest } from "./CellFormulaController";
import { InferredSeriesMenuAction } from "./InferredSeriesMenuAction";
import {
  applyOperation,
  exportFrameCsv,
  importCliSource,
  importDatabaseSource,
  importExcelRange,
  inspectExcelWorkbook,
  newWindow,
  pickDataFile,
  saveDocumentAsDialog,
  setHistoryMenuState,
  type ExcelWorkbookInfo,
} from "./lib/api";
import { reconcileSelection } from "./lib/reconcileSelection";
import { formulaToken } from "./lib/formulaReferences";
import { enterPosition, tabPosition, type GridDirection } from "./lib/gridNavigation";
import { applicationShortcut, hasNativeMenu } from "./lib/applicationShortcuts";
import {
  selectedCanvasView,
  withCanvasView,
} from "./lib/canvasNavigation";
import { outlineFrame, frameNames } from "./lib/dataSources";
import {
  CANVAS_OUTLINE_ZOOM,
  DEFAULT_CANVAS_ZOOM,
  canvasPoint,
  nudgeCanvasZoom,
} from "./lib/canvasZoom";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ComputedFrame,
  DataObject,
  DataType,
  DocumentView,
  FormulaFunction,
  Operation,
  Selection,
  FrameObject,
  ContainerObject,
} from "./lib/types";

export type JoinState = { primaryFrameId: string; x: number; y: number } | null;
/**
 * Which panel is open beside the canvas, if any.
 *
 * They share one slot, so they are one piece of state rather than a boolean
 * each: opening Project while Data is open is a switch and not a stack, and
 * the canvas only ever has to know whether something is there.
 */
export type LeftPanel = "data" | "project" | null;
export type InspectorSection = "selection" | "format" | "wrangle";
/** Tab labels. `.inspector-nav button` capitalizes them either way; written
 *  capitalized because these are also the tabs' accessible names and their
 *  tooltips, and neither of those is styled by the stylesheet. */
const inspectorSectionLabels: Record<InspectorSection, string> = {
  selection: "Selection",
  format: "Format",
  wrangle: "Wrangle",
};
function importPosition(viewport: HTMLDivElement | null): { x: number; y: number } {
  return {
    x: (viewport?.scrollLeft ?? 0) + 110,
    y: (viewport?.scrollTop ?? 0) + 100,
  };
}


export default function App() {
  const formulaEditorActive = useActiveFormulaEditorPresence();
  const {
    commit: commitActiveFormulaEditor,
    disengage: disengageActiveFormulaEditor,
    getActive: getActiveFormulaEditor,
    insertReference: insertActiveFormulaReference,
    clear: clearActiveFormulaEditor,
  } = useActiveFormulaEditorCommands();
  const [document, setDocument] = useState<DocumentView | null>(null);
  const [documentPath, setDocumentPath] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [gridFocus, setGridFocus] = useState<GridFocus | null>(null);
  const [cellFormulaRequest, setCellFormulaRequest] =
    useState<CellFormulaRequest | null>(null);
  const cellFormulaToken = useRef(0);
  const [insertList, setInsertList] = useState<{ containerId: string } | null>(null);
  /**
   * Which block line ⌘J last asked for the cursor, and a token that counts the
   * asks. The token is what makes a second ⌘J at the same line mean anything:
   * the target may not have changed, but the request has.
   *
   * A null `blockId` means the newest block, which is the only way to name a
   * block that did not exist when the key was pressed.
   */
  const [scratchFocus, setScratchFocus] = useState<{
    blockId: string | null;
    token: number;
  } | null>(null);
  const [scratchworkDrawerOpen, setScratchworkDrawerOpen] = useState(false);
  /** Where ⌘J was pressed from, so pressing it again can go back there. */
  const scratchReturn = useRef<{
    left: number;
    top: number;
    selection: Selection | null;
  } | null>(null);
  const [join, setJoin] = useState<JoinState>(null);
  const [datasetLibrary, setDatasetLibrary] = useState(false);
  const [excelImport, setExcelImport] = useState<{
    workbook: ExcelWorkbookInfo;
    position: { x: number; y: number };
  } | null>(null);
  const [leftPanel, setLeftPanel] = useState<LeftPanel>(null);
  const [newDocumentOpen, setNewDocumentOpen] = useState(false);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [preferencesPage, setPreferencesPage] = useState<"settings" | "shortcuts">("settings");
  const [sequenceFill, setSequenceFill] = useState<SequenceFillState | null>(null);
  const [runningCalculation, setRunningCalculation] =
    useState<RunningCalculationState | null>(null);
  const [recurrence, setRecurrence] = useState<RecurrenceState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [inspectorSection, setInspectorSection] =
    useState<InspectorSection>("selection");
  const [error, setError] = useState<string | null>(null);
  /** For work that finishes quietly and is worth reporting anyway. */
  const [notice, setNotice] = useState<string | null>(null);
  const [dataRefreshRevision, setDataRefreshRevision] = useState(0);
  const [useThousandsSeparators, setUseThousandsSeparators] =
    useThousandsSeparatorsPreference();
  /** Set while the import question is on screen, holding where the card goes. */
  const [importAsking, setImportAsking] = useState<{ x: number; y: number } | null>(
    null
  );
  /** The frame an import choice will stack beneath, when this began at its menu. */
  const [appendImport, setAppendImport] = useState<{
    frameId: string;
    x: number;
    y: number;
  } | null>(null);
  const { interfaceScale, setInterfaceScale, interfaceScaleError } =
    useInterfaceScalePreference();
  const { mcpSettings, mcpSettingsError, changeMcpEnabled } = useMcpSettings();
  const canvasRef = useRef<HTMLDivElement>(null);
  const documentOpened = document !== null;

  const { canvasZoom, canvasZoomRef, zoomCanvas, viewportSize } = useCanvasViewport({
    canvasRef,
    documentOpened,
  });
  // What each card actually has on screen, by frame. A ref rather than
  // state: this changes on every scroll of a paged frame, and nothing here
  // renders from it — the keyboard and clipboard handlers read it when a
  // key arrives.
  const renderedRows = useRef(new Map<string, RenderedGrid>());
  const publishRenderedRows = useCallback(
    (frameId: string, grid: RenderedGrid | null) => {
      if (grid) renderedRows.current.set(frameId, grid);
      else renderedRows.current.delete(frameId);
    },
    []
  );

  const {
    importMode,
    setImportMode,
    askOnImport,
    setAskOnImport,
    runImport,
    runAppendImport,
    handleOpenDocument,
  } = useImportFlow({
    setDocument,
    setSelection,
    setContextMenu,
    setError,
    setInspectorSection,
    setGridFocus,
    setDatasetLibrary,
  });

  useDocumentBootstrap({
    setDocument,
    setDocumentPath,
    setSelection,
    setContextMenu,
    setError,
    setDatasetLibrary,
  });

  useEffect(() => {
    const closeContextMenu = () => {
      setContextMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeContextMenu();
    };
    window.addEventListener("pointerdown", closeContextMenu);
    window.addEventListener("blur", closeContextMenu);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeContextMenu);
      window.removeEventListener("blur", closeContextMenu);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [setContextMenu]);

  const run = useCallback(
    async (operation: Operation, options?: { inlineError?: boolean }) => {
      try {
        setDocument(await applyOperation(operation));
        setError(null);
        return null;
      } catch (reason) {
        const message = String(reason).replace(/^Error:\s*/, "");
        if (!options?.inlineError) setError(message);
        return message;
      }
    },
    []
  );

  const {
    contextObject,
    contextFrame,
    contextColumn,
    contextIsMaterialized,
    contextGrid,
    contextKind,
    contextGenerator,
    contextEntryKey,
    contextCrosstabValues,
    openContextMenu,
    deleteFromContext,
  } = useContextMenu({
    contextMenu,
    setContextMenu,
    document,
    gridFocus,
    renderedRows,
    canvasRef,
    canvasZoomRef,
    setSelection,
    run,
  });

  const {
    addCalculatedColumnRequest,
    transformColumnRequest,
    filterColumnRequest,
    hidePipelineColumnRequest,
    rearrangeColumnsRequest,
    clearAddCalculatedColumnRequest,
    clearTransformColumnRequest,
    clearFilterColumnRequest,
    clearHidePipelineColumnRequest,
    clearRearrangeColumnsRequest,
    requestAddCalculatedColumn,
    requestColumnTransformation,
    requestColumnFill,
    requestColumnFilter,
    requestCalculatedColumnEdit,
    requestHidePipelineColumn,
    requestRearrangeColumns,
    transformColumnToken,
    setTransformColumnRequest,
  } = usePipelineColumnRequests({
    setContextMenu,
    setSelection,
    setInspectorSection,
  });

  const deleteContextColumn = () => {
    if (!document || !contextFrame || !contextColumn) return;
    const computed = document.computedFrames[contextFrame.id];
    if (computed?.editing.rows) {
      deleteFromContext({
        type: "deleteColumn",
        frameId: contextFrame.id,
        columnId: contextColumn.id,
      });
      return;
    }
    // A computed or source-backed grid cannot delete its input data. Its
    // equivalent gesture is the same one the chain already exposes: leave
    // this column out of the final Select. Put that request through the open
    // editor so its local draft and the saved pipeline change together.
    requestHidePipelineColumn(contextFrame.id, contextColumn.id, contextMenu?.viewId);
  };

  const {
    freeze,
    refreshConnector,
    changeFrameSource,
    refreshingSnapshots,
    refreshStale,
    takeOwnership,
    packageThisDocument,
    compactData,
    freezeCopy,
    setFrameCached,
    navigateHistory,
  } = useDocumentLifecycle({
    setDocument,
    setError,
    setNotice,
    setSelection,
    setContextMenu,
    setDataRefreshRevision,
  });

  const {
    pendingConnectorRefresh,
    requestConnectorRefresh,
    confirmPendingConnectorRefresh,
    cancelPendingConnectorRefresh,
  } = useConnectorRefreshApproval({ document, refreshConnector });

  // Moving the canvas to a card rather than opening anything: the sidebar
  // is an index, and an index takes you to the thing. A frame sitting on a
  // background tab is brought forward, since scrolling to a card that is
  // showing something else is not arriving anywhere.
  const jumpToObject = useCallback(
    (objectId: string) => {
      if (!document) return;
      const view =
        document.views.find((candidate) => candidate.objectId === objectId) ??
        document.views.find((candidate) =>
          candidate.tabObjectIds?.includes(objectId)
        );
      setSelection({ objectId, viewId: view?.id });
      setInspectorSection("wrangle");
      if (!view) return;
      if (view.objectId !== objectId) {
        void run({ type: "setActiveTab", viewId: view.id, objectId });
      }
      // The card's position is in canvas units and the scroll is in screen
      // pixels, so the jump is only right at 100% unless it is scaled.
      const zoom = canvasZoomRef.current;
      canvasRef.current?.scrollTo({
        left: Math.max(0, view.x * zoom - 120),
        top: Math.max(0, view.y * zoom - 80),
        behavior: "smooth",
      });
    },
    [canvasZoomRef, document, run]
  );

  // Just inside the top-left corner of what is on screen. The offsets are
  // screen distances, so they shrink into canvas units as the canvas zooms
  // out — otherwise a card inserted at 40% lands a long way in from the
  // corner you asked for.
  //
  // Only refs are read, so this never goes stale and never needs rebuilding.
  const insertPosition = useCallback(
    () => ({
      x: ((canvasRef.current?.scrollLeft ?? 0) + 110) / canvasZoomRef.current,
      y: ((canvasRef.current?.scrollTop ?? 0) + 100) / canvasZoomRef.current,
    }),
    [canvasZoomRef]
  );

  const fitViewToWindow = useFitViewToWindow(canvasRef, canvasZoomRef, run);

  const {
    scratchTargetId,
    scratchworkBlock,
    scratchworkBarReferences,
    summonScratchpad,
    appendScratchworkFromBar,
    toggleScratchworkDrawer,
  } = useScratchwork({
    document,
    setDocument,
    scratchFocus,
    setScratchFocus,
    scratchworkDrawerOpen,
    setScratchworkDrawerOpen,
    scratchReturn,
    canvasRef,
    selection,
    setSelection,
    run,
    insertPosition,
    jumpToObject,
    getActiveFormulaEditor,
    commitActiveFormulaEditor,
  });

  // Counted from the view rather than asked of the backend: every computed
  // frame already carries the answer for itself.
  /** Every object that lives inside a container, so the canvas draws each
   * of them once — on its container's card rather than on its own. */
  const containedIds = useMemo(
    () =>
      new Set(
        (document?.objects ?? []).flatMap((object) =>
          object.kind === "container" ? object.memberIds : []
        )
      ),
    [document]
  );

  const navigateCanvas = useCanvasNavigation({
    document, selection, containedIds, canvasRef, canvasZoomRef, setSelection, setGridFocus,
  });

  const containers = useMemo(
    () =>
      (document?.objects ?? []).filter(
        (object): object is ContainerObject => object.kind === "container"
      ),
    [document]
  );

  const staleSnapshotCount = useMemo(
    () =>
      Object.values(document?.computedFrames ?? {}).filter(
        (computed) => computed.materialization?.stale
      ).length,
    [document]
  );

  const handleSaveAsDocument = useCallback(async () => {
    try {
      const saved = await saveDocumentAsDialog();
      if (!saved) return;
      setDocument(saved.document);
      setDocumentPath(saved.path);
      setError(null);
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, []);

  // The grid focus always mirrors the cell selection; when another interaction
  // moves the selection elsewhere (headers, other objects, canvas), leave the
  // navigate/edit modes and fall back to canvas mode.
  useEffect(() => {
    setGridFocus((current) => {
      if (!current) return current;
      if (
        !selection ||
        selection.objectId !== current.objectId ||
        selection.rowId !== current.rowId ||
        selection.columnId !== current.columnId
      )
        return null;
      return current;
    });
    setCellFormulaRequest((current) =>
      current &&
      selection &&
      current.cellId ===
        `${selection.objectId}:${selection.rowId}:${selection.columnId}`
        ? current
        : null
    );
  }, [selection]);

  // Navigate-mode key handling: movement, range selection/fill, clearing,
  // and editor entry. Clipboard events are handled separately below.
  const handleNavigateKey = useGridKeyboardNavigation({
    document,
    gridFocus,
    renderedRows,
    cellFormulaToken,
    transformColumnToken,
    setCellFormulaRequest,
    setTransformColumnRequest,
    clearActiveFormulaEditor,
    setGridFocus,
    setSelection,
    setInspectorSection,
    run,
  });

  const {
    copyIncludesHeaders,
    setCopyHeadersDefault,
    copySelection,
    copyColumnReference,
    handleGridCopy,
    handleGridCut,
    handleGridPaste,
  } = useGridClipboard({
    document,
    gridFocus,
    renderedRows,
    run,
    setError,
    setFrameCached,
  });

  // One visual step from the active cell after an editor commit (Enter/Tab family).
  const stepGridFocus = useCallback(
    (direction: GridDirection) => {
      if (!document || !gridFocus) return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const position = context
        ? visualGridPosition(context, gridFocus.rowId, gridFocus.columnId)
        : null;
      const nextPosition =
        context && position
          ? direction === "left" || direction === "right"
            ? tabPosition(position, direction === "left", gridBoundsFor(context))
            : enterPosition(position, direction === "up", gridBoundsFor(context))
          : null;
      const target = context && nextPosition ? gridCellAt(context, nextPosition) : null;
      const next: GridFocus = target
        ? {
            ...gridFocus,
            rowId: target.row.id,
            columnId: target.column.id,
            mode: "navigate",
            editSeed: null,
            anchor: null,
          }
        : { ...gridFocus, mode: "navigate", editSeed: null, anchor: null };
      setGridFocus(next);
      setSelection({
        objectId: next.objectId,
        viewId: next.viewId,
        rowId: next.rowId,
        columnId: next.columnId,
      });
    },
    [document, gridFocus]
  );

  // Window-level dispatcher, routed by focus mode. Edit-mode keys live on the
  // cell editor itself, and keys are never hijacked from other text inputs
  // (formula editors, name fields, draft rows).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const modifier = event.metaKey || event.ctrlKey;
      const shortcut = applicationShortcut(event);
      // ⌘S, on an application with nothing to save. Every edit is on disk
      // before the key comes back up, so the only document this can mean
      // anything for is the scratch canvas that has no file to be on — and
      // for that one it means precisely what the person pressing it wants.
      // The menu deliberately claims no ⌘S, so there is no accelerator here
      // to race, on any platform.
      if (shortcut === "save") {
        event.preventDefault();
        if (!documentPath) void handleSaveAsDocument();
        return;
      }
      // The platform menu owns all of these when there is one: it takes the
      // key equivalent before the webview is offered it, so a handler here as
      // well would fire twice wherever it does not. They stay bound for the
      // browser dev server, which has no menu bar to inherit them from.
      if (!hasNativeMenu() && shortcut) {
        event.preventDefault();
        if (shortcut === "scratchpad") void summonScratchpad();
        else if (shortcut === "add-block") void run({
          type: "addBlock",
          name: nextObjectName(document?.objects ?? [], "Block"),
          ...insertPosition(),
        });
        else if (shortcut === "add-text")
          void run({ type: "addText", ...insertPosition() });
        else if (shortcut === "add-frame") void run({
          type: "addFrame",
          name: "Frame 1",
          grid: [["Column 1", "Column 2"], ["", ""], ["", ""]],
          ...insertPosition(),
        });
        else if (shortcut === "add-container") void run({
          type: "addContainer",
          name: nextContainerName(document?.objects ?? []),
          ...insertPosition(),
        });
        else if (shortcut.startsWith("inspector-"))
          setInspectorSection(shortcut.replace("inspector-", "") as InspectorSection);
        else if (shortcut === "arrange") {
          void run({ type: "tidyLayout" });
        } else if (shortcut === "fit" || shortcut === "collapse") {
          const selectedView = selectedCanvasView(document, selection);
          if (selectedView) {
            if (shortcut === "fit") void fitViewToWindow(selectedView);
            else void run({
              type: "setViewCollapsed",
              viewId: selectedView.id,
              collapsed: !selectedView.collapsed,
            });
          }
        } else if (shortcut === "library") {
          setDatasetLibrary(true);
        } else if (shortcut === "shortcuts") {
          setPreferencesPage("shortcuts");
          setPreferencesOpen(true);
        } else if (shortcut === "settings") {
          setPreferencesPage("settings");
          setPreferencesOpen(true);
        } else if (shortcut === "zoom-in") {
          zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, 1));
        } else if (shortcut === "zoom-out") {
          zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, -1));
        } else if (shortcut === "zoom-reset") {
          zoomCanvas(DEFAULT_CANVAS_ZOOM);
        } else if (shortcut === "new") {
          setNewDocumentOpen(true);
        } else if (shortcut === "new-window") {
          void newWindow().catch((reason) => setError(String(reason)));
        } else if (shortcut === "open") void handleOpenDocument();
        else if (shortcut === "save-as") void handleSaveAsDocument();
        else void navigateHistory(shortcut === "redo" ? "redo" : "undo");
        return;
      }
      if (isTextEntryTarget(event.target)) return;
      if (contextMenu || preferencesOpen) return;
      if (gridFocus?.mode === "navigate") {
        handleNavigateKey(event);
        return;
      }
      if (
        selection &&
        event.target instanceof HTMLElement &&
        !event.target.closest("button, a, input, textarea, select, [contenteditable]")
      ) {
        const direction = ARROW_DIRECTIONS[event.key];
        const cycle =
          event.key === "Tab" ? (event.shiftKey ? "previous" : "next") : null;
        if ((direction && !modifier && !event.shiftKey) || cycle) {
          if (navigateCanvas(direction ?? cycle!)) event.preventDefault();
          return;
        }
      }
      if (event.key === "Escape" && !gridFocus && selection) setSelection(null);
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("copy", handleGridCopy);
    window.addEventListener("cut", handleGridCut);
    window.addEventListener("paste", handleGridPaste);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("copy", handleGridCopy);
      window.removeEventListener("cut", handleGridCut);
      window.removeEventListener("paste", handleGridPaste);
    };
  }, [
    canvasZoomRef,
    contextMenu,
    document,
    documentPath,
    fitViewToWindow,
    gridFocus,
    handleGridCopy,
    handleGridCut,
    handleGridPaste,
    handleNavigateKey,
    handleOpenDocument,
    handleSaveAsDocument,
    insertPosition,
    navigateHistory,
    navigateCanvas,
    preferencesOpen,
    run,
    selection,
    summonScratchpad,
    zoomCanvas,
  ]);

  const selectedCommandView = selectedCanvasView(document, selection);
  useApplicationMenu(hasNativeMenu(), {
    "new-window": () => void newWindow().catch((reason) => setError(String(reason))),
    "new-document": () => setNewDocumentOpen(true),
    "open-document": () => void handleOpenDocument(),
    "save-document-as": () => void handleSaveAsDocument(),
    "package-document": () => void packageThisDocument(),
    "compact-data": () => void compactData(),
    preferences: () => { setPreferencesPage("settings"); setPreferencesOpen(true); },
    "keyboard-shortcuts": () => { setPreferencesPage("shortcuts"); setPreferencesOpen(true); },
    undo: () => void navigateHistory("undo"),
    redo: () => void navigateHistory("redo"),
    "data-library": () => setDatasetLibrary(true),
    "toggle-sources": () => setLeftPanel((panel) => panel === "data" ? null : "data"),
    "tidy-layout": () => void run({ type: "tidyLayout" }),
    "fit-view": () => withCanvasView(selectedCommandView, fitViewToWindow),
    "collapse-view": () => withCanvasView(selectedCommandView, (view) => void run({ type: "setViewCollapsed", viewId: view.id, collapsed: !view.collapsed })),
    "inspector-selection": () => setInspectorSection("selection"),
    "inspector-format": () => setInspectorSection("format"),
    "inspector-wrangle": () => setInspectorSection("wrangle"),
    "add-block": () => void addBlock(),
    "add-text": () => void addText(),
    "add-frame": () => void addEmptyFrame(),
    "add-container": () => void addContainer(),
    scratchpad: () => void summonScratchpad(),
    "zoom-in": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, 1)),
    "zoom-out": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, -1)),
    "zoom-reset": () => zoomCanvas(DEFAULT_CANVAS_ZOOM),
  }, setError);

  // Undo and Redo grey out with the document's history. Nothing else tells the
  // menu, so every view that arrives pushes it — including the first, which is
  // what leaves both disabled on a document opened fresh. A menu that will not
  // take the news is not worth an error banner over.
  useEffect(() => {
    if (!hasNativeMenu() || !document) return;
    void setHistoryMenuState(document.canUndo, document.canRedo).catch(() => {});
  }, [document?.canUndo, document?.canRedo]);

  /** Pressing the rail button for the panel already open closes it. */
  const toggleLeftPanel = (panel: Exclude<LeftPanel, null>) =>
    setLeftPanel((current) => (current === panel ? null : panel));

  /**
   * A new formula block, named `Block 1`, `Block 2`, … until somebody says
   * otherwise.
   *
   * This is the canvas's one way of making somewhere to put a number now.
   * There used to be three — a value, a result, and a list, each its own card
   * — and a page of scratch arithmetic turned into a page of cards. A block
   * holds all three as lines, so `rate = 0.08` and `` monthly = `Loan`/12 ``
   * sit together in the order they were worked out, which is how the working
   * was written down in the first place.
   */
  const addBlock = (position?: { x: number; y: number }) =>
    run({
      type: "addBlock",
      name: nextObjectName(document?.objects ?? [], "Block"),
      ...(position ?? insertPosition()),
    });

  /** A card of prose: markdown, with `{{…}}` holes that print live values. */
  const addText = (position?: { x: number; y: number }) =>
    run({
      type: "addText",
      ...(position ?? insertPosition()),
    });

  /**
   * A new frame is an empty 2×2 you can type or paste into, not a dialog
   * asking for data up front.
   *
   * The dialog was a text box for pasting, which put a modal between the
   * user and the thing they wanted; the grid is already a better text box.
   * Pasting into it replaces it outright — see `handleGridPaste` — so the
   * old flow survives as "make one, then paste".
   */
  const addEmptyFrame = (position?: { x: number; y: number }) =>
    run({
      type: "addFrame",
      name: "Frame 1",
      grid: [
        ["Column 1", "Column 2"],
        ["", ""],
        ["", ""],
      ],
      ...(position ?? insertPosition()),
    });

  const addContainer = (position?: { x: number; y: number }) =>
    run({
      type: "addContainer",
      name: nextContainerName(document?.objects ?? []),
      ...(position ?? insertPosition()),
    });

  useEffect(() => {
    if (!document) return;
    setSelection((current) => {
      if (!current) return null;
      const view = current.viewId
        ? document.views.find((candidate) => candidate.id === current.viewId)
        : undefined;
      // Follows the selected tab whatever kind it is, so selecting a plot
      // tab does not snap the selection back to the frame it draws.
      const activeTab = view ? activeTabObject(view, document) : undefined;
      if (activeTab && activeTab.id !== current.objectId) {
        return { objectId: activeTab.id, viewId: view!.id };
      }
      return reconcileSelection(document, current);
    });
  }, [document]);

  if (!document) {
    return (
      <main className="loading-screen">
        <div className="mark">F</div>
        <p>{error ?? "Opening your canvas…"}</p>
        {error && (
          <button className="secondary-action" onClick={() => window.location.reload()}>
            Retry
          </button>
        )}
      </main>
    );
  }

  // The canvas is as large as the cards on it plus most of a screen in each
  // direction. The slack is the point: without it the lowest card stops
  // against the end of the scroll range, pinned to the bottom of the window,
  // and cannot be brought to the middle of the screen to be worked on.
  //
  // The slack is measured in screen pixels and the canvas in canvas units, so
  // it grows as the canvas zooms out — a screenful of room has to stay a
  // screenful of room at every magnification.
  const canvasExtent = {
    width: Math.max(
      1800,
      Math.round(
        document.views.reduce((right, view) => Math.max(right, view.x + view.width), 0) +
          Math.max(320, viewportSize.width * 0.6) / canvasZoom
      )
    ),
    height: Math.max(
      1200,
      Math.round(
        document.views.reduce(
          (bottom, view) =>
            Math.max(bottom, view.y + (view.collapsed ? 29 : view.height)),
          0
        ) + Math.max(320, viewportSize.height * 0.7) / canvasZoom
      )
    ),
  };

  // Only worth computing when the cards are actually going to draw as
  // outlines: at reading zoom every card renders its own contents and none of
  // this is looked at.
  const showOutlines = canvasZoom < CANVAS_OUTLINE_ZOOM;
  const outlineNames = showOutlines ? frameNames(document) : null;

  // Where ⌘J last asked for the cursor. Resolved here rather than in the
  // handler because the block it asks for may only have come into existence
  // on the way back from the operation that made it.
  const selectedViewById = selection?.viewId
    ? document.views.find((view) => view.id === selection.viewId)
    : undefined;
  const selectedActiveTab = selectedViewById
    ? activeTabObject(selectedViewById, document)
    : undefined;
  const selectedObjectId = selectedActiveTab?.id ?? selection?.objectId;
  const selectedObject = selectedObjectId
    ? document.objects.find((object) => object.id === selectedObjectId) ?? null
    : null;
  // One answer to "is the inspector there", used by the panel and by the
  // canvas that makes room for it. Two answers is how you get a 360px strip
  // of nothing: the canvas insetting for a panel that decided not to draw.
  const showInspector = Boolean(
    selectedObject && selection && selectedObject.kind !== "block"
  );
  const selectedView =
    selectedViewById ??
    (selectedObject
      ? document.views.find((view) => view.id === selection?.viewId) ??
        document.views.find((view) =>
          tabObjects(view, document).some((tab) => tab.id === selectedObject.id)
        )
      : undefined);
  // A tab is closable when its card has another one to fall back to. Closing
  // it deletes the object it is, which the core refuses if anything
  // downstream reads it, so the button can be optimistic here.
  const closableTabIds = new Set(
    document.views.flatMap((view) => {
      const tabs = tabObjects(view, document);
      return tabs.length > 1 ? tabs.map((tab) => tab.id) : [];
    })
  );
  const selectedGridContext = gridFocus
    ? resolveGridContext(document, gridFocus, renderedRows.current)
    : null;
  const selectedCellFormulaReferences = selectedGridContext
    ? [
        ...selectedGridContext.frame.columns
          .filter((column) => column.id !== gridFocus?.columnId)
          .map((column) => ({
            id: column.id,
            objectId: selectedGridContext.frame.id,
            frameId: selectedGridContext.frame.id,
            label: column.name,
            token: formulaToken(column.name),
            kind: "column" as const,
            detail: `${column.dataType} column in ${selectedGridContext.frame.name}`,
          })),
        ...scalarFormulaReferences(
          document.objects,
          document.formulaFunctions,
          document.computedFrames
        ).filter(
          (reference) =>
            reference.kind !== "frame" && reference.kind !== "column"
        ),
      ]
    : [];
  const handleFormulaPointerDown = canvasFormulaPointerHandler({
    document,
    getActive: getActiveFormulaEditor,
    insertReference: insertActiveFormulaReference,
    clear: clearActiveFormulaEditor,
    disengage: disengageActiveFormulaEditor,
    onNotice: setNotice,
    onRecurrence: setRecurrence,
  });

  return (
    <NumberDisplayContext.Provider value={useThousandsSeparators}>
    <div
      className={`app-shell${formulaEditorActive ? " formula-pick-active" : ""}`}
      onContextMenu={openContextMenu}
      onPointerDownCapture={handleFormulaPointerDown}
    >
      <CellAwareFormulaBar
        context={selectedGridContext}
        focus={gridFocus}
        onCommit={appendScratchworkFromBar}
        references={scratchworkBarReferences}
        cellReferences={selectedCellFormulaReferences}
        cellFormulaRequest={cellFormulaRequest}
        onOperation={run}
        onEditCalculated={requestCalculatedColumnEdit}
        onTransformColumn={requestColumnFill}
        onReadOnly={(nextSelection, reason) => {
          setSelection(nextSelection);
          setInspectorSection("selection");
          setNotice(reason);
        }}
        expanded={scratchworkDrawerOpen}
        onToggle={() => void toggleScratchworkDrawer()}
        onCellFormulaSaved={() => {
          setCellFormulaRequest(null);
          stepGridFocus("down");
        }}
      />
      <ReferenceHighlights />
      {scratchworkDrawerOpen && scratchworkBlock && (
        <section
          id="scratchwork-drawer"
          className="scratchwork-drawer"
          aria-label="Scratchwork"
        >
          <BlockCard
            block={scratchworkBlock}
            computed={document.computedBlocks[scratchworkBlock.id]}
            focusToken={1}
            objects={document.objects}
            computedFrames={document.computedFrames}
            formulaFunctions={document.formulaFunctions}
            onOperation={run}
            onFreeze={freeze}
          />
        </section>
      )}
      <LeftRail
        leftPanel={leftPanel}
        setLeftPanel={setLeftPanel}
        toggleLeftPanel={toggleLeftPanel}
        onOpenLibrary={() => setDatasetLibrary(true)}
        addBlock={addBlock}
        addText={addText}
        addEmptyFrame={addEmptyFrame}
        addContainer={addContainer}
        viewCount={document.views.length}
        onOperation={run}
      />

      {leftPanel === "data" && (
        <DataSidebar
          document={document}
          selectedObjectId={selection?.objectId}
          onJump={jumpToObject}
          onImport={() => setDatasetLibrary(true)}
          onRefreshConnector={requestConnectorRefresh}
          onSourceChanged={changeFrameSource}
          onClose={() => setLeftPanel(null)}
        />
      )}

      {leftPanel === "project" && (
        <ProjectPanel
          document={document}
          path={documentPath}
          onClose={() => setLeftPanel(null)}
          onOperation={run}
          onSaveAs={handleSaveAsDocument}
          onPackage={packageThisDocument}
          onCompact={compactData}
        />
      )}

      <main
        className={`canvas-viewport ${showInspector ? "with-inspector" : ""} ${
          leftPanel ? "with-panel" : ""
        }`}
        ref={canvasRef}
        // Dragging the bare canvas moves the canvas. A press that does not
        // travel is still a click, and still clears the selection — which is
        // what the press used to do on its own, and only when it landed on
        // the viewport rather than on the dotted grid filling it.
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          if ((event.target as HTMLElement).closest(".canvas-object")) return;
          const element = event.currentTarget;
          const start = {
            x: event.clientX,
            y: event.clientY,
            left: element.scrollLeft,
            top: element.scrollTop,
          };
          let panning = false;
          const move = (moveEvent: PointerEvent) => {
            const dx = moveEvent.clientX - start.x;
            const dy = moveEvent.clientY - start.y;
            // A few pixels of slop, so a click with an unsteady hand is a
            // click and not a one-pixel pan that eats the deselect.
            if (!panning && Math.hypot(dx, dy) < 3) return;
            panning = true;
            element.scrollLeft = start.left - dx;
            element.scrollTop = start.top - dy;
          };
          const end = () => {
            window.removeEventListener("pointermove", move);
            window.removeEventListener("pointerup", end);
            window.removeEventListener("pointercancel", end);
            element.classList.remove("panning");
            if (!panning) setSelection(null);
          };
          element.classList.add("panning");
          window.addEventListener("pointermove", move);
          window.addEventListener("pointerup", end);
          window.addEventListener("pointercancel", end);
        }}
        onDragOver={(event) => {
          if (
            hasFrameTabDrag(event) &&
            !(event.target as HTMLElement).closest(".canvas-object")
          ) {
            event.preventDefault();
            event.dataTransfer.dropEffect = "move";
          }
        }}
        onDrop={(event) => {
          if ((event.target as HTMLElement).closest(".canvas-object")) return;
          const payload = readFrameTabDrag(event);
          if (!payload) return;
          event.preventDefault();
          const bounds = event.currentTarget.getBoundingClientRect();
          const dropped = canvasPoint(
            { x: event.clientX, y: event.clientY },
            {
              left: bounds.left,
              top: bounds.top,
              scrollLeft: event.currentTarget.scrollLeft,
              scrollTop: event.currentTarget.scrollTop,
            },
            canvasZoomRef.current
          );
          // The card lands under the cursor rather than starting at it: the
          // grab was somewhere in the middle of a tab, not on its corner.
          const x = Math.max(0, dropped.x - 100);
          const y = Math.max(0, dropped.y - 16);
          setSelection(null);
          void run({
            type: "detachTab",
            viewId: payload.sourceViewId,
            objectId: payload.objectId,
            x,
            y,
          });
        }}
        onDoubleClick={(event) => {
          if (event.target !== event.currentTarget) return;
          const bounds = event.currentTarget.getBoundingClientRect();
          void addBlock(
            canvasPoint(
              { x: event.clientX, y: event.clientY },
              {
                left: bounds.left,
                top: bounds.top,
                scrollLeft: event.currentTarget.scrollLeft,
                scrollTop: event.currentTarget.scrollTop,
              },
              canvasZoomRef.current
            )
          );
        }}
      >
        {/* The zoom is one transform on the whole canvas rather than a
            recalculation of everything on it: the cords, the dot grid and the
            cards all scale together, and a card's own scroll geometry is left
            in the layout pixels its virtualiser measures in. */}
        <div
          className="canvas-grid"
          aria-label="Free-form data canvas"
          style={{
            width: canvasExtent.width,
            height: canvasExtent.height,
            transform: canvasZoom === 1 ? undefined : `scale(${canvasZoom})`,
            transformOrigin: "0 0",
          }}
        >
          <div className="canvas-heading">
            <span>ANALYSIS CANVAS</span>
            <h1>{document.name}</h1>
            <p>
              {document.objects.length === 0
                ? "Nothing here yet. Press ⌘J to start writing, open a document or a sample from the Data library, import a file, or add a frame."
                : `${document.objects.length} object${
                    document.objects.length === 1 ? "" : "s"
                  } · ${document.views.length} window${
                    document.views.length === 1 ? "" : "s"
                  }`}
            </p>
          </div>

          <LineageCords
            document={document}
            selection={selection}
            width={canvasExtent.width}
            height={canvasExtent.height}
          />

          {document.views.map((view) => {
            const object = document.objects.find(
              (candidate) => candidate.id === view.objectId
            );
            if (!object) return null;
            // Something inside a container is drawn by that container's
            // card. Drawing its own as well would put it in two places.
            if (containedIds.has(object.id)) return null;
            // The frame this card is about: itself when a frame tab is
            // selected, and the plot's own source when a plot tab is. Both
            // the plot body and the tab strip's "add" menu need it, and it
            // is the same frame either way.
            const cardFrame = activeTabFrame(view, document);
            return (
              <CanvasObject
                key={view.id}
                view={view}
                object={object}
                objects={document.objects}
                computed={
                  object.kind === "frame"
                    ? document.computedFrames[object.id]
                    : undefined
                }
                tabs={tabObjects(view, document)}
                computedFrames={document.computedFrames}
                computedResults={document.computedResults}
                computedBlocks={document.computedBlocks}
                computedTexts={document.computedTexts}
                scratchFocusToken={
                  object.id === scratchTargetId ? scratchFocus?.token : undefined
                }
                scratchworkInDrawer={
                  scratchworkDrawerOpen && object.id === scratchworkBlock?.id
                }
                formulaFunctions={document.formulaFunctions}
                sourceFrame={cardFrame}
                sourceComputed={
                  cardFrame ? document.computedFrames[cardFrame.id] : undefined
                }
                closableTabIds={closableTabIds}
                zoom={canvasZoom}
                outline={
                  outlineNames && object.kind === "frame"
                    ? outlineFrame(
                        object,
                        document.computedFrames[object.id],
                        outlineNames,
                        chainFilterCount(document.computedFrames[object.id])
                      )
                    : undefined
                }
                selection={selection}
                gridFocus={gridFocus}
                onSelect={setSelection}
                onFitToWindow={(nextView) => void fitViewToWindow(nextView)}
                onGridFocus={setGridFocus}
                onGridStep={stepGridFocus}
                onRenderedRows={publishRenderedRows}
                onOperation={run}
                onRearrangeColumns={(frameId, columnIds) =>
                  requestRearrangeColumns(frameId, columnIds, view.id)
                }
                onFilterColumn={(frame, column) =>
                  requestColumnFilter(frame, column, view.id)
                }
                onTransformColumn={(frame, column, formula) =>
                  requestColumnFill(frame, column, formula, undefined, view.id)
                }
                onEditCalculatedColumn={(frame, column, rowIndex) =>
                  requestCalculatedColumnEdit(frame, column, rowIndex, view.id)
                }
                onFreeze={freeze}
                onAddList={(containerId) => setInsertList({ containerId })}
                dataRefreshRevision={dataRefreshRevision}
              />
            );
          })}
        </div>
      </main>

      {/* Anchored to the canvas corner rather than to a bar across the top.
          Both of these are statements about what is on the canvas, both are
          absent most of the time, and a band kept permanently at the top of
          the window to hold two things that are usually not there is the
          trade this corner exists to avoid. */}
      <CanvasStatus
        withInspector={showInspector}
        context={selectedGridContext}
        focus={gridFocus}
        documentPath={documentPath}
        staleCount={staleSnapshotCount}
        refreshing={refreshingSnapshots}
        zoom={canvasZoom}
        onOperation={run}
        onSave={() => void handleSaveAsDocument()}
        onRefresh={() => void refreshStale()}
        onZoom={zoomCanvas}
      />

      {/* Not for a block. A block is edited entirely on its own card, so the
          panel had nothing to put in itself — and a third of the window
          opening to show a heading is worse than not opening. */}
      {showInspector && selection && selectedObject && (
        <Inspector
          documentId={document.id}
          object={selectedObject}
          objects={document.objects}
          formulaFunctions={document.formulaFunctions}
          selection={selection}
          computed={
            selectedObject.kind === "frame"
              ? document.computedFrames[selectedObject.id]
              : undefined
          }
          suggestedPosition={
            selectedView
              ? {
                  x: selectedView.x + 80,
                  y:
                    selectedView.y +
                    selectedView.height +
                    70 +
                    document.objects.filter(
                      (candidate) =>
                        candidate.kind === "frame" &&
                        candidate.derivation?.sourceFrameId === selectedObject.id
                    ).length *
                      300,
                }
              : { x: 900, y: 100 }
          }
          onClose={() => setSelection(null)}
          section={inspectorSection}
          onSectionChange={setInspectorSection}
          addCalculatedColumnRequest={scopedPipelineRequest(
            addCalculatedColumnRequest,
            selectedObject.id
          )}
          onAddCalculatedColumnRequestHandled={clearAddCalculatedColumnRequest}
          transformColumnRequest={scopedPipelineRequest(
            transformColumnRequest,
            selectedObject.id
          )}
          onTransformColumnRequestHandled={clearTransformColumnRequest}
          filterColumnRequest={scopedPipelineRequest(
            filterColumnRequest,
            selectedObject.id
          )}
          onFilterColumnRequestHandled={clearFilterColumnRequest}
          hidePipelineColumnRequest={scopedPipelineRequest(
            hidePipelineColumnRequest,
            selectedObject.id
          )}
          onHidePipelineColumnRequestHandled={clearHidePipelineColumnRequest}
          rearrangeColumnsRequest={scopedPipelineRequest(
            rearrangeColumnsRequest,
            selectedObject.id
          )}
          onRearrangeColumnsRequestHandled={clearRearrangeColumnsRequest}
          onOperation={run}
          onSourceChanged={changeFrameSource}
          onSetCached={setFrameCached}
          onTakeOwnership={takeOwnership}
          onFreezeCopy={freezeCopy}
          onJoin={() =>
            setJoin({
              primaryFrameId: selectedObject.id,
              x: selectedView ? selectedView.x + selectedView.width + 100 : 900,
              y: selectedView?.y ?? 100,
            })
          }
          onTransformColumn={(column, formula, focus) =>
            requestColumnTransformation(
              selectedObject as FrameObject,
              column,
              formula,
              focus,
              selectedView?.id
            )
          }
        />
      )}

      {pendingConnectorRefresh && (
        <ConnectorRefreshConfirmDialog
          frameName={pendingConnectorRefresh.frameName}
          connector={pendingConnectorRefresh.connector}
          onConfirm={confirmPendingConnectorRefresh}
          onCancel={cancelPendingConnectorRefresh}
        />
      )}

      {insertList && (
        <InsertListDialog
          state={insertList}
          onClose={() => setInsertList(null)}
          onCreate={(operation) => {
            setInsertList(null);
            run(operation);
          }}
          onPickFile={pickDataFile}
        />
      )}

      {join && (
        <JoinDialog
          state={join}
          document={document}
          onClose={() => setJoin(null)}
          onOperation={run}
          onCreated={() => {
            setJoin(null);
          }}
        />
      )}

      <ColumnAuthoringDialogs
        document={document}
        sequence={sequenceFill}
        running={runningCalculation}
        recurrence={recurrence}
        onCloseSequence={() => setSequenceFill(null)}
        onCloseRunning={() => setRunningCalculation(null)}
        onCloseRecurrence={() => setRecurrence(null)}
        onTransform={(frame, column, formula, viewId, orderByColumnId) =>
          requestColumnTransformation(
            frame,
            column,
            formula,
            false,
            viewId,
            orderByColumnId
          )
        }
      />

      {(importAsking || appendImport) && (
        <ImportChoiceDialog
          mode={importMode}
          onModeChange={setImportMode}
          askOnImport={askOnImport}
          onAskOnImportChange={setAskOnImport}
          onCancel={() => {
            setImportAsking(null);
            setAppendImport(null);
          }}
          onChoose={(mode) => {
            const position = importAsking;
            const appendTarget = appendImport;
            setImportAsking(null);
            setAppendImport(null);
            if (appendTarget) void runAppendImport(appendTarget, mode);
            else if (position) void runImport(position, mode);
          }}
        />
      )}

      {preferencesOpen && preferencesPage === "settings" && (
        <PreferencesDialog
          interfaceScale={interfaceScale}
          interfaceScaleError={interfaceScaleError}
          onInterfaceScale={setInterfaceScale}
          importMode={importMode}
          onImportModeChange={setImportMode}
          askOnImport={askOnImport}
          onAskOnImportChange={setAskOnImport}
          copyIncludesHeaders={copyIncludesHeaders}
          onCopyIncludesHeaders={setCopyHeadersDefault}
          useThousandsSeparators={useThousandsSeparators}
          onUseThousandsSeparators={setUseThousandsSeparators}
          mcpSettings={mcpSettings}
          mcpSettingsError={mcpSettingsError}
          documentPath={documentPath}
          onMcpEnabledChange={changeMcpEnabled}
          onKeyboardShortcuts={() => setPreferencesPage("shortcuts")}
          onClose={() => setPreferencesOpen(false)}
        />
      )}
      {preferencesOpen && preferencesPage === "shortcuts" && (
        <KeyboardShortcutsDialog onClose={() => setPreferencesOpen(false)} />
      )}

      {newDocumentOpen && (
        <NewDocumentDialog
          onClose={() => setNewDocumentOpen(false)}
          onOpened={(opened) => {
            setDocument(opened.document);
            setDocumentPath(opened.path);
            setSelection(null);
            setGridFocus(null);
            setContextMenu(null);
            setNewDocumentOpen(false);
            setError(null);
          }}
        />
      )}

      {datasetLibrary && (
        <DatasetDialog
          document={document}
          onSourceChanged={changeFrameSource}
          onClose={() => setDatasetLibrary(false)}
          onImportFile={async () => {
            const viewport = canvasRef.current;
            const position = importPosition(viewport);
            // The question comes before the file picker rather than after:
            // it is about what this document becomes, not about the file,
            // and answering it first means the picker is the last step.
            if (askOnImport) {
              setDatasetLibrary(false);
              setImportAsking(position);
              return true;
            }
            const imported = await runImport(position, importMode);
            if (imported) setDatasetLibrary(false);
            return imported;
          }}
          onImportExcelFile={async () => {
            const workbook = await inspectExcelWorkbook();
            if (!workbook) return false;
            const viewport = canvasRef.current;
            setExcelImport({
              workbook,
              position: importPosition(viewport),
            });
            setDatasetLibrary(false);
            return true;
          }}
          onImportCliSource={async (source) => {
            const viewport = canvasRef.current;
            const next = await importCliSource(importPosition(viewport), source);
            setDocument(next);
            setDatasetLibrary(false);
            setNotice(`Connected ${source.sourceLabel}.`);
          }}
          onImportDatabaseSource={async (source) => {
            const viewport = canvasRef.current;
            const next = await importDatabaseSource(importPosition(viewport), source);
            setDocument(next);
            setDatasetLibrary(false);
            setNotice(`Connected ${source.sourceName}.`);
          }}
          onOpened={(opened) => {
            setDocument(opened.document);
            setDocumentPath(opened.path);
            setSelection(null);
            setContextMenu(null);
            setDatasetLibrary(false);
            setError(null);
          }}
        />
      )}

      {excelImport && (
        <ExcelImportDialog
          workbook={excelImport.workbook}
          onClose={() => setExcelImport(null)}
          onImport={async (selection: ExcelRangeSelection, another: boolean) => {
            const next = await importExcelRange(
              excelImport.workbook,
              selection,
              excelImport.position
            );
            setDocument(next);
            setSelection(null);
            setContextMenu(null);
            setError(null);
            setNotice(`Imported ${selection.name} from ${selection.sheetName}!${selection.cellRange}.`);
            if (another) {
              setExcelImport((current) =>
                current
                  ? {
                      ...current,
                      position: {
                        x: current.position.x + 42,
                        y: current.position.y + 42,
                      },
                    }
                  : null
              );
            } else {
              setExcelImport(null);
            }
          }}
        />
      )}

      {notice && !error && (
        <div className="notice-toast" role="status">
          <Check size={16} />
          <span>{notice}</span>
          <button onClick={() => setNotice(null)}>
            <X size={15} />
          </button>
        </div>
      )}

      {error && (
        <div className="error-toast" role="alert">
          <CircleAlert size={18} />
          <span>{error}</span>
          <button onClick={() => setError(null)}>
            <X size={15} />
          </button>
        </div>
      )}

      {contextMenu && (
        <ContextMenuSurface x={contextMenu.screenX} y={contextMenu.screenY}>
          <div className="context-menu-heading">
            <span>{contextKind}</span>
            {(contextObject || contextFrame) && (
              <strong>
                {contextColumn
                  ? `${contextFrame?.name} / ${contextColumn.name}`
                  : (contextFrame ?? contextObject)?.name}
              </strong>
            )}
          </div>
          {/* Copy comes first and applies to the selection, not to whatever
              the pointer happens to be over — right-clicking inside a range
              is how people reach for it, and the range is what they mean.
              The preference changes what this one ordinary Copy does; its
              inverse remains explicit without turning the menu into a
              preferences panel. */}
          {gridFocus && contextFrame && contextFrame.id === gridFocus.objectId && (
            <>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void copySelection(copyIncludesHeaders);
                }}
              >
                <Copy size={14} />
                <span>Copy</span>
                <kbd>⌘C</kbd>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void copySelection(!copyIncludesHeaders);
                }}
              >
                <Copy size={14} />
                <span>
                  {copyIncludesHeaders ? "Copy without headers" : "Copy with headers"}
                </span>
              </button>
              <span className="menu-separator" />
            </>
          )}
          {contextFrame && (
            <GridEditingMenu
              column={contextColumn}
              computed={document.computedFrames[contextFrame.id]}
              rowId={contextMenu.rowId}
              viewId={contextMenu.viewId}
              gridContext={contextGrid}
              gridFocus={gridFocus}
              frameId={contextFrame.id}
              onClose={() => setContextMenu(null)}
              onSelect={setSelection}
              onGridFocus={setGridFocus}
              onSetCells={(updates) => {
                void run({
                  type: "setCells",
                  frameId: contextFrame.id,
                  cells: updates,
                });
              }}
            />
          )}
          {contextFrame &&
            contextMenu.rowId &&
            !document.computedFrames[contextFrame.id]?.editing.cells && (
              <details className="context-menu-submenu">
                <summary>
                  <Database size={14} />
                  <span>Data source</span>
                  <ChevronRight className="submenu-chevron" size={14} />
                </summary>
                <div>
                  <button
                    onClick={() => {
                      const reason =
                        document.computedFrames[contextFrame.id]?.editing.reason;
                      setContextMenu(null);
                      setSelection({
                        objectId: contextFrame.id,
                        viewId: contextMenu.viewId,
                        rowId: contextMenu.rowId,
                        columnId: contextColumn?.id,
                      });
                      setInspectorSection("selection");
                      setNotice(reason ?? "Make an owned copy to edit this value.");
                    }}
                  >
                    <Database size={14} />
                    <span>Make these rows editable…</span>
                  </button>
                </div>
              </details>
            )}
          {contextObject &&
            (contextObject.kind === "value" ||
              contextObject.kind === "series" ||
              contextObject.kind === "container") && (
              <>
                {containers
                  .filter(
                    (candidate) =>
                      candidate.id !== contextObject.id &&
                      !candidate.memberIds.includes(contextObject.id)
                  )
                  .map((candidate) => (
                    <button
                      key={candidate.id}
                      onClick={() => {
                        setContextMenu(null);
                        run({
                          type: "moveIntoContainer",
                          objectId: contextObject.id,
                          containerId: candidate.id,
                        });
                      }}
                    >
                      <FolderInput size={14} />
                      <span>Keep under {candidate.name}</span>
                    </button>
                  ))}
                {containedIds.has(contextObject.id) && (
                  <button
                    onClick={() => {
                      setContextMenu(null);
                      run({
                        type: "moveIntoContainer",
                        objectId: contextObject.id,
                        containerId: null,
                      });
                    }}
                  >
                    <FolderOutput size={14} />
                    <span>Take out onto the canvas</span>
                  </button>
                )}
                <span className="menu-separator" />
              </>
            )}
          {!contextObject ? (
            <>
              {/* A block, a frame, a container. There used to be three more
                  — a value, a result, a list — and every one of them made a
                  card that held one number. Those are lines of a block now,
                  which is the same three things in a tenth of the room. */}
              <button
                onClick={() => {
                  setContextMenu(null);
                  void addBlock({
                    x: contextMenu.canvasX,
                    y: contextMenu.canvasY,
                  });
                }}
              >
                <SquareFunction size={14} />
                <span>Add formula block here</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void addText({
                    x: contextMenu.canvasX,
                    y: contextMenu.canvasY,
                  });
                }}
              >
                <Type size={14} />
                <span>Add text here</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void addEmptyFrame({
                    x: contextMenu.canvasX,
                    y: contextMenu.canvasY,
                  });
                }}
              >
                <FrameIcon size={14} />
                <span>Add frame here</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void addContainer({
                    x: contextMenu.canvasX,
                    y: contextMenu.canvasY,
                  });
                }}
              >
                <FolderPlus size={14} />
                <span>Add container here</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void run({
                    type: "addGeneratorFrame",
                    name: "Generator",
                    formula: "sequence(1, 11)",
                    columnName: null,
                    x: contextMenu.canvasX,
                    y: contextMenu.canvasY,
                  });
                }}
              >
                <ListOrdered size={14} />
                <span>Add generator here</span>
              </button>
            </>
          ) : contextFrame ? (
            <>
              <ColumnContextAuthoringActions
                frame={contextFrame}
                column={contextColumn}
                grid={contextGrid}
                rowId={contextMenu.rowId}
                viewId={contextMenu.viewId}
                onTransform={(formula, focus) =>
                  contextColumn &&
                  requestColumnTransformation(
                    contextFrame,
                    contextColumn,
                    formula,
                    focus,
                    contextMenu.viewId
                  )
                }
                onEdit={() =>
                  contextColumn &&
                  requestCalculatedColumnEdit(
                    contextFrame,
                    contextColumn,
                    contextMenu.rowIndex,
                    contextMenu.viewId
                  )
                }
                onRunning={(state) => {
                  setContextMenu(null);
                  setRunningCalculation(state);
                }}
                onRecurrence={(state) => {
                  setContextMenu(null);
                  setRecurrence(state);
                }}
                onSequence={(state) => {
                  setContextMenu(null);
                  setSequenceFill(state);
                }}
                compact={contextMenu.rowId !== undefined}
              />
              <button
                onClick={() =>
                  requestAddCalculatedColumn(
                    contextFrame.id,
                    contextColumn?.id,
                    contextMenu.rowIndex,
                    contextMenu.viewId
                  )
                }
              >
                <FunctionSquare size={14} />
                <span>
                  {contextMenu.rowIndex === undefined
                    ? "Add calculated column"
                    : "Formula here"}
                </span>
              </button>
              {contextGenerator && contextColumn && (
                <InferredSeriesMenuAction
                  frame={contextFrame}
                  column={contextColumn}
                  inference={contextGenerator}
                  viewId={contextMenu.viewId}
                  x={contextMenu.canvasX}
                  y={contextMenu.canvasY}
                  onClose={() => setContextMenu(null)}
                  onFill={setSequenceFill}
                  onOperation={run}
                />
              )}
              {contextEntryKey && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    void run({
                      type: "addEntryColumn",
                      frameId: contextFrame.id,
                      name: nextEntryColumnName(contextFrame),
                      dataType: "number",
                      keyColumnIds: contextEntryKey,
                    });
                  }}
                >
                  <KeyRound size={14} />
                  <span>Add entry column (keyed)</span>
                </button>
              )}
              {contextColumn && contextCrosstabValues && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    void run({
                      type: "setFrameDisplayCrosstab",
                      frameId: contextFrame.id,
                      crosstab: {
                        namesColumnId: contextColumn.id,
                        valuesColumnId: contextCrosstabValues,
                      },
                    });
                  }}
                >
                  <FrameIcon size={14} />
                  <span>Spread across columns</span>
                </button>
              )}
              {contextFrame.display?.crosstab && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    void run({
                      type: "setFrameDisplayCrosstab",
                      frameId: contextFrame.id,
                      crosstab: null,
                    });
                  }}
                >
                  <FrameIcon size={14} />
                  <span>Back to rows</span>
                </button>
              )}
              {contextColumn &&
                !document.computedFrames[contextFrame.id]?.editing.rows && (
                  <label className="context-menu-field">
                    <span>Convert column type</span>
                    <select
                      value={contextColumn.dataType}
                      onChange={(event) =>
                        requestColumnTransformation(
                          contextFrame,
                          contextColumn,
                          `${formulaToken(contextColumn.name)}.cast("${event.target.value}")`,
                          false,
                          contextMenu.viewId
                        )
                      }
                    >
                      <option value="string">Text</option>
                      <option value="categorical" disabled>
                        Categorical
                      </option>
                      <option value="integer">Integer</option>
                      <option value="number">Number</option>
                      <option value="currency" disabled>
                        Currency
                      </option>
                      <option value="percentage" disabled>
                        Percentage
                      </option>
                      <option value="boolean">Boolean</option>
                      <option value="date">Date</option>
                    </select>
                  </label>
                )}
              {/* Reading a column from another frame needs that frame to
                  hold a snapshot, which is one action away — so this is one
                  action, not two, and it says which one it is doing. */}
              {contextColumn && (contextFrame.derivation || contextIsMaterialized) && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    void copyColumnReference(
                      contextFrame,
                      contextColumn,
                      contextIsMaterialized
                    );
                  }}
                >
                  <Braces size={14} />
                  <span>
                    {contextIsMaterialized
                      ? "Copy reference to this column"
                      : "Materialize and copy reference"}
                  </span>
                </button>
              )}
              <ContextMenuGroup
                collapsed={contextMenu.rowId !== undefined}
                label="Frame actions"
                Icon={Frame}
              >
              <button
                onClick={() => {
                  setContextMenu(null);
                  setJoin({
                    primaryFrameId: contextFrame.id,
                    x: contextMenu.canvasX + 60,
                    y: contextMenu.canvasY + 40,
                  });
                }}
              >
                <GitMerge size={14} />
                <span>Join another frame</span>
              </button>
              <button
                onClick={() => {
                  const target = {
                    frameId: contextFrame.id,
                    x: contextMenu.canvasX + 54,
                    y: contextMenu.canvasY + 54,
                  };
                  setContextMenu(null);
                  // An append source has the same static-versus-refreshable
                  // choice as an ordinary import. Keeping the prompt before
                  // the picker avoids quietly creating a different kind of
                  // data because this import began at a frame menu.
                  if (askOnImport) {
                    setAppendImport(target);
                    return;
                  }
                  void runAppendImport(target, importMode);
                }}
              >
                <FolderOpen size={14} />
                <span>Import and append…</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  run({
                    type: "addLinkedFrame",
                    sourceFrameId: contextFrame.id,
                    name: `${contextFrame.name} frame`,
                    x: contextMenu.canvasX + 28,
                    y: contextMenu.canvasY + 28,
                  });
                }}
              >
                <GitBranch size={14} />
                <span>Create frame from this</span>
              </button>
              {contextFrame.derivation && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    setSelection({ objectId: contextFrame.id });
                    setInspectorSection("wrangle");
                  }}
                >
                  <FunctionSquare size={14} />
                  <span>
                    {contextColumn
                      ? `Filter or sort by ${contextColumn.name}`
                      : "Edit transformations"}
                  </span>
                </button>
              )}
              {/* A plot of this frame can live beside it as a tab or stand
                  on its own; both are one operation apart, so offer both
                  rather than making the user move it afterwards. */}
              {contextMenu.viewId && (
                <button
                  onClick={() => {
                    setContextMenu(null);
                    run({
                      type: "addPlot",
                      name: `${contextFrame.name} plot`,
                      sourceFrameId: contextFrame.id,
                      spec: defaultPlotSpec(contextFrame),
                      x: 0,
                      y: 0,
                      viewId: contextMenu.viewId,
                    });
                  }}
                >
                  <BarChart3 size={14} />
                  <span>Plot in this card</span>
                </button>
              )}
              <button
                onClick={() => {
                  setContextMenu(null);
                  run({
                    type: "addPlot",
                    name: `${contextFrame.name} plot`,
                    sourceFrameId: contextFrame.id,
                    spec: defaultPlotSpec(contextFrame),
                    x: contextMenu.canvasX + 28,
                    y: contextMenu.canvasY + 28,
                  });
                }}
              >
                <BarChart3 size={14} />
                <span>Plot in a new window</span>
              </button>
              <button
                onClick={() => {
                  setContextMenu(null);
                  void exportFrameCsv(contextFrame.id).catch((reason) =>
                    setError(String(reason).replace(/^Error:\s*/, ""))
                  );
                }}
              >
                <ArrowDownToLine size={14} />
                <span>Export CSV</span>
              </button>
              {/* A read-only grid can still choose which outputs it shows.
                  This is exactly unchecking the column in a final Select;
                  owned rows keep the structural delete offered below. */}
              {!document.computedFrames[contextFrame.id]?.editing.rows &&
                contextColumn && (
                  <button
                    className="destructive"
                    onClick={deleteContextColumn}
                  >
                    <Trash2 size={14} />
                    <span>Delete column</span>
                  </button>
                )}
              {document.computedFrames[contextFrame.id]?.editing.rows && (
                <>
                    <button
                      onClick={() => {
                        setContextMenu(null);
                        run({ type: "addRow", frameId: contextFrame.id, values: {} });
                      }}
                    >
                      <Plus size={14} />
                      <span>Add empty row</span>
                    </button>
                    <button
                      onClick={() => {
                        setContextMenu(null);
                        run({
                          type: "addColumn",
                          frameId: contextFrame.id,
                          name: nextColumnName(contextFrame),
                          dataType: "string",
                          afterColumnId:
                            contextColumn?.id ??
                            contextFrame.columns.at(-1)?.id ??
                            null,
                        });
                      }}
                    >
                      <Plus size={14} />
                      <span>
                        {contextColumn ? "Insert column here" : "Add column"}
                      </span>
                    </button>
                    {contextColumn && (
                      <label className="context-menu-field">
                        <span>Column type</span>
                        <select
                          value={contextColumn.dataType}
                          onChange={(event) => {
                            setContextMenu(null);
                            run({
                              type: "setColumnType",
                              frameId: contextFrame.id,
                              columnId: contextColumn.id,
                              dataType: event.target.value as DataType,
                            });
                          }}
                        >
                          <option value="string">Text</option>
                          <option value="categorical">Categorical</option>
                          <option value="integer">Integer</option>
                          <option value="number">Number</option>
                          <option value="currency">Currency</option>
                          <option value="percentage">Percentage</option>
                          <option value="boolean">Boolean</option>
                          <option value="date">Date</option>
                        </select>
                      </label>
                    )}
                    {contextMenu.rowId && (
                      <button
                        className="destructive"
                        onClick={() =>
                          deleteFromContext({
                            type: "deleteRow",
                            frameId: contextFrame.id,
                            rowId: contextMenu.rowId!,
                          })
                        }
                      >
                        <Trash2 size={14} />
                        <span>Delete row</span>
                      </button>
                    )}
                    {contextColumn && (
                      <button
                        className="destructive"
                        onClick={deleteContextColumn}
                      >
                        <Trash2 size={14} />
                        <span>Delete column</span>
                      </button>
                    )}
                </>
              )}
              {/* The one control for a relationship that has two shapes. A
                  derived frame is either a tab on the card it reads from or
                  a card of its own with a cord back to it — never both, and
                  never neither — so it is a switch between them rather than
                  two commands that each only apply half the time. There is
                  exactly one card it would sensibly join, so nothing has to
                  be aimed at and nothing can be missed. */}
              {(() => {
                const own = viewHolding(document, contextFrame.id);
                const parentId = contextFrame.derivation?.sourceFrameId;
                const parent = parentId
                  ? document.objects.find((object) => object.id === parentId)
                  : undefined;
                const parentView = parentId
                  ? viewHolding(document, parentId)
                  : undefined;
                if (!own || !parent || !parentView) return null;
                const tabbed = parentView.id === own.id;
                return (
                  <label className="context-menu-check">
                    <input
                      type="checkbox"
                      checked={tabbed}
                      onChange={(event) => {
                        setContextMenu(null);
                        void run(
                          event.target.checked
                            ? {
                                type: "moveTab",
                                sourceViewId: own.id,
                                targetViewId: parentView.id,
                                objectId: contextFrame.id,
                                targetIndex: parentView.tabObjectIds?.length ?? 1,
                              }
                            : {
                                type: "detachTab",
                                viewId: own.id,
                                objectId: contextFrame.id,
                                x: contextMenu.canvasX,
                                y: contextMenu.canvasY,
                              }
                        );
                      }}
                    />
                    <span>Show as a tab on {parent.name}</span>
                  </label>
                );
              })()}
              <button
                className="destructive"
                onClick={() =>
                  deleteFromContext({
                    type: "deleteObject",
                    objectId: contextFrame.id,
                  })
                }
              >
                <Trash2 size={14} />
                <span>Delete frame</span>
              </button>
              </ContextMenuGroup>
            </>
          ) : (
            <button
              className="destructive"
              onClick={() =>
                deleteFromContext({
                  type: "deleteObject",
                  objectId: contextObject.id,
                })
              }
            >
              <Trash2 size={14} />
              <span>Delete {contextObject.kind}</span>
            </button>
          )}
        </ContextMenuSurface>
      )}

    </div>
    </NumberDisplayContext.Provider>
  );
}


type InspectorProps = {
  documentId: string;
  object: DataObject;
  objects: DataObject[];
  formulaFunctions: FormulaFunction[];
  selection: Selection;
  computed?: ComputedFrame;
  suggestedPosition: { x: number; y: number };
  onClose: () => void;
  section: InspectorSection;
  onSectionChange: Dispatch<SetStateAction<InspectorSection>>;
  addCalculatedColumnRequest?: AddCalculatedColumnEditorRequest;
  onAddCalculatedColumnRequestHandled: () => void;
  transformColumnRequest?: TransformColumnEditorRequest;
  onTransformColumnRequestHandled: () => void;
  filterColumnRequest?: FilterColumnEditorRequest;
  onFilterColumnRequestHandled: () => void;
  hidePipelineColumnRequest?: HidePipelineColumnEditorRequest;
  onHidePipelineColumnRequestHandled: () => void;
  rearrangeColumnsRequest?: RearrangeColumnsEditorRequest;
  onRearrangeColumnsRequestHandled: () => void;
  onOperation: OperationHandler;
  onSourceChanged: SetFrameSourceHandler;
  onSetCached: SetFrameCachedHandler;
  onTakeOwnership: TakeOwnershipHandler;
  onFreezeCopy: FreezeCopyHandler;
  onJoin: () => void;
  onTransformColumn: (column: Column, formula: string, focus?: boolean) => void;
};

function Inspector({
  documentId,
  object,
  objects,
  formulaFunctions,
  selection,
  computed,
  suggestedPosition,
  onClose,
  section,
  onSectionChange,
  addCalculatedColumnRequest,
  onAddCalculatedColumnRequestHandled,
  transformColumnRequest,
  onTransformColumnRequestHandled,
  filterColumnRequest,
  onFilterColumnRequestHandled,
  hidePipelineColumnRequest,
  onHidePipelineColumnRequestHandled,
  rearrangeColumnsRequest,
  onRearrangeColumnsRequestHandled,
  onOperation,
  onSourceChanged,
  onSetCached,
  onTakeOwnership,
  onFreezeCopy,
  onJoin,
  onTransformColumn,
}: InspectorProps) {
  return (
    <aside className="inspector">
      <div className="inspector-header">
        <div>
          <span className="eyebrow">INSPECTOR</span>
          <h2>{object.name || "Unnamed object"}</h2>
        </div>
        <button className="icon-button" onClick={onClose}>
          <X size={17} />
        </button>
      </div>
      {object.kind === "frame" && (
        <nav className="inspector-nav" aria-label="Inspector sections">
          {(
            [
              "selection",
              "format",
              "wrangle",
            ] as InspectorSection[]
          ).map((candidate) => (
            <button
              key={candidate}
              className={section === candidate ? "active" : ""}
              aria-label={inspectorSectionLabels[candidate]}
              aria-pressed={section === candidate}
              onClick={() => onSectionChange(candidate)}
              title={`${inspectorSectionLabels[candidate]} (⌘${candidate === "selection" ? "1" : candidate === "format" ? "2" : "3"})`}
            >
              {inspectorSectionLabels[candidate]}
            </button>
          ))}
        </nav>
      )}
      {object.kind === "value" && (
        <ValueInspector value={object} onOperation={onOperation} />
      )}
      {/* A list is edited on its card, where the whole of it is visible. The
          inspector says the two things the card cannot: what it is called in
          a formula, and how to use it. */}
      {object.kind === "series" && (
        <section className="inspector-section">
          <h3>List</h3>
          <p className="inspector-note">
            {object.values.length}{" "}
            {object.values.length === 1 ? "value" : "values"} · {object.dataType}
          </p>
          <p className="inspector-note">
            Write <code>`{object.name}`</code> in a formula to pass it to
            something that takes a list, like <code>.is_in()</code>.
          </p>
        </section>
      )}
      {object.kind === "frame" && (
        <FrameInspector
          documentId={documentId}
          frame={object}
          objects={objects}
          formulaFunctions={formulaFunctions}
          selection={selection}
          computed={computed!}
          suggestedPosition={suggestedPosition}
          section={section}
          addCalculatedColumnRequest={addCalculatedColumnRequest}
          onAddCalculatedColumnRequestHandled={
            onAddCalculatedColumnRequestHandled
          }
          transformColumnRequest={transformColumnRequest}
          onTransformColumnRequestHandled={onTransformColumnRequestHandled}
          filterColumnRequest={filterColumnRequest}
          onFilterColumnRequestHandled={onFilterColumnRequestHandled}
          hidePipelineColumnRequest={hidePipelineColumnRequest}
          onHidePipelineColumnRequestHandled={
            onHidePipelineColumnRequestHandled
          }
          rearrangeColumnsRequest={rearrangeColumnsRequest}
          onRearrangeColumnsRequestHandled={
            onRearrangeColumnsRequestHandled
          }
          onOperation={onOperation}
          onSourceChanged={onSourceChanged}
          onSetCached={onSetCached}
          onTakeOwnership={onTakeOwnership}
          onFreezeCopy={onFreezeCopy}
          onJoin={onJoin}
          onTransformColumn={onTransformColumn}
        />
      )}
      {object.kind === "plot" &&
        (() => {
          const frame = objects.find(
            (candidate): candidate is FrameObject =>
              candidate.kind === "frame" && candidate.id === object.sourceFrameId
          );
          return frame ? (
            <PlotInspector plot={object} frame={frame} onOperation={onOperation} />
          ) : null;
        })()}
    </aside>
  );
}


function nextObjectName(objects: DataObject[], stem: string): string {
  const taken = new Set(objects.map((object) => object.name));
  if (!taken.has(stem)) return stem;
  let suffix = 2;
  while (taken.has(`${stem} ${suffix}`)) suffix += 1;
  return `${stem} ${suffix}`;
}

/** A container name nothing has taken yet. */
function nextContainerName(objects: DataObject[]): string {
  const taken = new Set(objects.map((object) => object.name));
  if (!taken.has("Container")) return "Container";
  let suffix = 2;
  while (taken.has(`Container ${suffix}`)) suffix += 1;
  return `Container ${suffix}`;
}



/**
 * The next free name for an entry column: names are formula addresses, so
 * a second "Entry" must not shadow the first.
 */
function nextEntryColumnName(frame: FrameObject): string {
  const names = new Set(frame.columns.map((column) => column.name));
  if (!names.has("Entry")) return "Entry";
  let index = 2;
  while (names.has(`Entry ${index}`)) index += 1;
  return `Entry ${index}`;
}
