import { useHistoryMenuState } from "./hooks/useHistoryMenuState";
import { CircleAlert, Check, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useReducer, useState } from "react";
import { CanvasContextMenu } from "./CanvasContextMenu";
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
import { FindPaletteHost } from "./FindPalette";
import { HelpBrowser } from "./HelpBrowser";
import {
  QuickCommands,
  quickCommandItems,
  type SelectionActions,
  type SelectionTarget,
} from "./QuickCommands";
import { UpdateDialog } from "./UpdateDialog";
import { useApplicationMenu } from "./useApplicationMenu";
import { useFitViewToWindow } from "./useFitViewToWindow";
import { useCanvasNavigation } from "./useCanvasNavigation";
import { useThousandsSeparatorsPreference } from "./hooks/useThousandsSeparatorsPreference";
import { useInterfaceScalePreference } from "./hooks/useInterfaceScalePreference";
import { useMcpSettings } from "./hooks/useMcpSettings";
import { useModifierHints } from "./hooks/useModifierHints";
import { useCanvasObjectCreation } from "./hooks/useCanvasObjectCreation";
import {
  usePipelineColumnRequests,
  scopedPipelineRequest,
} from "./hooks/usePipelineColumnRequests";
import { useContextMenu } from "./hooks/useContextMenu";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useScratchwork } from "./hooks/useScratchwork";
import { useScratchworkWindowBridge } from "./hooks/useScratchworkWindowBridge";
import { useImportFlow } from "./hooks/useImportFlow";
import { useCanvasClipboard } from "./hooks/useCanvasClipboard";
import { useGridClipboard } from "./hooks/useGridClipboard";
import { useCanvasViewport } from "./hooks/useCanvasViewport";
import { useDocumentLifecycle } from "./hooks/useDocumentLifecycle";
import { useConnectorRefreshApproval } from "./hooks/useConnectorRefreshApproval";
import { ConnectorRefreshConfirmDialog } from "./ConnectorRefreshConfirmDialog";
import { useGridKeyboardNavigation } from "./hooks/useGridKeyboardNavigation";
import { useDocumentBootstrap } from "./hooks/useDocumentBootstrap";
import { useOpenedDocument } from "./hooks/useOpenedDocument";
import { CollapsedInspector, Inspector } from "./Inspector";
import { JoinDialog } from "./JoinDialog";
import { DataSidebar } from "./DataSidebar";
import { LeftRail } from "./LeftRail";
import { CARD_SIZES, frameCardSize, placeNewCard } from "./lib/cardPlacement";
import {
  INITIAL_INSPECTOR_PANEL,
  inspectorAvailable,
  inspectorPanelReducer,
  inspectorShown,
} from "./lib/inspectorPanel";
import {
  focusGridClipboardTarget,
  keyboardBelongsToCanvas,
} from "./lib/gridClipboardTarget";
import { CanvasObject, LineageCords } from "./CanvasObject";
import { NewDocumentDialog } from "./NewDocumentDialog";
import { DatasetDialog } from "./DatasetDialog";
import { ExcelImportDialog, type ExcelRangeSelection } from "./ExcelImportDialog";
import { InsertListDialog } from "./InsertListDialog";
import { VectorCombinePrompt, type VectorCombineState } from "./VectorCombinePrompt";
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
  gridRangeForFocus,
  isTextEntryTarget,
  pipelineSortKeys,
  resolveGridContext,
  tabObjects,
  visualGridPosition,
  type ContextMenuState,
  type GridContext,
  type GridFocus,
  type RenderedGrid,
} from "./FrameGrid";
import { hasFrameTabDrag, readFrameTabDrag } from "./FrameViewTabs";
import { revealScroll } from "./lib/canvasReveal";
import { useInsertPosition } from "./hooks/useInsertPosition";
import { useSplashSlow } from "./hooks/useSplashSlow";
import { hasVectorDrag, readVectorDrag } from "./lib/vectorDrag";
import { combineVectorWithFrame } from "./lib/vectorCombine";
import {
  useActiveFormulaEditorCommands,
  useActiveFormulaEditorPresence,
} from "./ActiveFormulaEditor";
import { CellAwareFormulaBar } from "./CellAwareFormulaBar";
import type { CellFormulaRequest } from "./CellFormulaController";
import { SafeModeBanner } from "./SafeModeBanner";
import {
  applyOperation,
  importCliSource,
  importDatabaseSource,
  importExcelRange,
  inspectExcelWorkbook,
  listRecentDocuments,
  newWindow,
  openDocument,
  pickDataFile,
  saveDocumentAsDialog,
  type ExcelWorkbookInfo,
  type RecentDocument,
} from "./lib/api";
import { reconcileSelection } from "./lib/reconcileSelection";
import {
  columnRangeKey,
  emptyColumnRangeMemo,
  stableColumnRange,
} from "./lib/selectedColumnRange";
import { formulaToken } from "./lib/formulaReferences";
import { enterPosition, tabPosition, type GridDirection } from "./lib/gridNavigation";
import {
  applicationShortcut,
  hasNativeMenu,
  receivesMenuCommands,
  shortcutCommandId,
} from "./lib/applicationShortcuts";
import { reportIgnoredFailure } from "./lib/errorReporting";
import { selectedCanvasView, withCanvasView } from "./lib/canvasNavigation";
import { pinnedThrough } from "./lib/pinnedColumns";
import { outlineFrame, frameNames } from "./lib/dataSources";
import {
  CANVAS_OUTLINE_ZOOM,
  DEFAULT_CANVAS_ZOOM,
  canvasPoint,
  nudgeCanvasZoom,
} from "./lib/canvasZoom";
import type { JoinState } from "./lib/joinState";
import type {
  DocumentView,
  Operation,
  Selection,
  FrameObject,
  ContainerObject,
} from "./lib/types";

/**
 * Which panel is open beside the canvas, if any.
 *
 * They share one slot, so they are one piece of state rather than a boolean
 * each: opening Project while Data is open is a switch and not a stack, and
 * the canvas only ever has to know whether something is there.
 */
export type LeftPanel = "data" | "project" | null;
/**
 * The selected object as Quick Commands names it: the card, the column when
 * one is picked, and the few facts its verbs need to read correctly — whether
 * the card is collapsed, how the column sorts today, how far a pin would
 * reach, and whether the frame owns its rows (which decides whether a column
 * is deleted or hidden). A plain description rather than the objects
 * themselves, so `quickCommandItems` stays testable without a document.
 */
function quickCommandSelection(
  document: DocumentView | null,
  selection: Selection | null
): SelectionTarget | null {
  if (!document || !selection) return null;
  const object = document.objects.find(
    (candidate) => candidate.id === selection.objectId
  );
  if (!object) return null;
  const frame = object.kind === "frame" ? object : null;
  const column =
    frame?.columns.find((candidate) => candidate.id === selection.columnId) ?? null;
  const sortKey =
    frame && column
      ? pipelineSortKeys(document.computedFrames[frame.id]).find(
          (key) => key.columnId === column.id
        )
      : undefined;
  return {
    name: column ? `${object.name} · ${column.name}` : object.name,
    objectId: object.id,
    kind: object.kind,
    collapsed: selectedCanvasView(document, selection)?.collapsed ?? false,
    frameId: frame?.id,
    column:
      frame && column
        ? {
            id: column.id,
            name: column.name,
            descending: sortKey ? sortKey.descending : null,
            pinThrough: pinnedThrough(frame, column),
            ownsRows: Boolean(document.computedFrames[frame.id]?.editing.rows),
          }
        : undefined,
  };
}

/** Ids of every column the grid selection covers, in frame order. */
function selectedGridColumnIds(context: GridContext, focus: GridFocus): string[] {
  const range = gridRangeForFocus(context, focus);
  if (!range) return [];
  const [from, to] =
    context.orientation === "fieldsAsRows"
      ? [range.top, range.bottom]
      : [range.left, range.right];
  return context.frame.columns.slice(from, to + 1).map((column) => column.id);
}

export default function App() {
  const localFormulaEditorActive = useActiveFormulaEditorPresence();
  const formulaCommands = useActiveFormulaEditorCommands();
  const {
    commit: commitActiveFormulaEditor,
    getActive: getActiveFormulaEditor,
    clear: clearActiveFormulaEditor,
  } = formulaCommands;
  const [document, setDocument] = useState<DocumentView | null>(null);
  const [documentPath, setDocumentPath] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [gridFocus, setGridFocus] = useState<GridFocus | null>(null);
  const [cellFormulaRequest, setCellFormulaRequest] =
    useState<CellFormulaRequest | null>(null);
  const cellFormulaToken = useRef(0);
  const [insertList, setInsertList] = useState<{ containerId: string } | null>(null);
  const [vectorCombine, setVectorCombine] = useState<VectorCombineState | null>(null);
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
  // The ids that existed before a join was created. The joined frame is the
  // one not among them once the document comes back, and it is what the
  // person just made, so it is what gets selected, sized to its columns, and
  // scrolled to -- not the source frame the inspector was showing.
  const [pendingJoinSelect, setPendingJoinSelect] = useState<Set<string> | null>(null);
  const [datasetLibrary, setDatasetLibrary] = useState(false);
  const [excelImport, setExcelImport] = useState<{
    workbook: ExcelWorkbookInfo;
    position: { x: number; y: number };
  } | null>(null);
  const [leftPanel, setLeftPanel] = useState<LeftPanel>(null);
  const [newDocumentOpen, setNewDocumentOpen] = useState(false);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [preferencesPage, setPreferencesPage] = useState<"settings" | "shortcuts">(
    "settings"
  );
  const [helpScope, setHelpScope] = useState<"formulas" | "guide" | null>(null);
  const [lastHelpScope, setLastHelpScope] = useState<"formulas" | "guide">("guide");
  const [quickCommandsOpen, setQuickCommandsOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  /** What Find opens on, when something handed it a query. */
  const [findQuery, setFindQuery] = useState("");
  /** The device's recents, read when the palette first needs to list them. */
  const [recents, setRecents] = useState<RecentDocument[]>([]);
  const [sequenceFill, setSequenceFill] = useState<SequenceFillState | null>(null);
  const [runningCalculation, setRunningCalculation] =
    useState<RunningCalculationState | null>(null);
  const [recurrence, setRecurrence] = useState<RecurrenceState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  // One reducer rather than a hidden flag beside the section, so the
  // dispatch stays the stable section setter every hook already takes, and
  // asking for a section is what un-hides the panel (see inspectorPanel.ts).
  const [inspectorPanel, setInspectorSection] = useReducer(inspectorPanelReducer, INITIAL_INSPECTOR_PANEL);
  const inspectorSection = inspectorPanel.section;
  const [error, setError] = useState<string | null>(null);
  const splashSlow = useSplashSlow(document === null && !error);
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
  // Hold ⌘ and every control that has a shortcut wears it.
  useModifierHints();
  const canvasRef = useRef<HTMLDivElement>(null);

  const { canvasZoom, canvasZoomRef, zoomCanvas, viewportSize } = useCanvasViewport({
    canvasRef,
    documentOpened: document !== null,
  });

  // What a document open clears in this window, and the canvas jump that goes
  // with it — see `useOpenedDocument` for why each line is here.
  const resetForOpenedDocument = useCallback(
    (opened: { document: DocumentView; path: string | null }) => {
      setDocument(opened.document);
      setDocumentPath(opened.path);
      setSelection(null);
      setGridFocus(null);
      setContextMenu(null);
      setError(null);
      setScratchFocus(null);
      setScratchworkDrawerOpen(false);
      clearActiveFormulaEditor();
    },
    [clearActiveFormulaEditor]
  );
  const adoptOpenedDocument = useOpenedDocument({
    document,
    canvasRef,
    canvasZoomRef,
    reset: resetForOpenedDocument,
  });
  // What each card actually has on screen, by frame. A ref rather than
  // state: this changes on every scroll of a paged frame, and the keyboard
  // and clipboard handlers read it when a key arrives rather than rendering
  // from it.
  const renderedRows = useRef(new Map<string, RenderedGrid>());
  // Three things *are* rendered from it, though — the formula bar's cell
  // label, the status bar's aggregate and the inspector's column range —
  // and a ref write reaches none of them. That is how a sort that moved the
  // selected cell to row 4 left the bar still saying "row 1": the card
  // published its new order and nothing on this side re-read it. So the
  // focused card's rows are mirrored into state. Only that card's: every
  // other frame keeps the silent ref write it had, which is what keeps
  // scrolling a million-row import off the render path.
  const [focusedGrids, setFocusedGrids] = useState(new Map<string, RenderedGrid>());
  const focusedFrameIdRef = useRef<string | null>(null);
  const publishRenderedRows = useCallback(
    (frameId: string, grid: RenderedGrid | null) => {
      if (grid) renderedRows.current.set(frameId, grid);
      else renderedRows.current.delete(frameId);
      if (frameId === focusedFrameIdRef.current)
        setFocusedGrids(new Map(renderedRows.current));
    },
    []
  );
  const columnRangeRef = useRef(emptyColumnRangeMemo);
  const focusedFrameId = gridFocus?.objectId ?? null;
  useEffect(() => {
    focusedFrameIdRef.current = focusedFrameId;
    // The newly focused card published its rows before it was focused, so
    // the mirror has to be taken now rather than waiting for its next one.
    setFocusedGrids(new Map(renderedRows.current));
  }, [focusedFrameId]);

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
    setNotice,
    setInspectorSection,
    setGridFocus,
    setDatasetLibrary,
  });

  useDocumentBootstrap({
    setDocument,
    setDocumentPath,
    setError,
    setDatasetLibrary,
    onDocumentOpened: adoptOpenedDocument,
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
    setGridFocus,
    run,
  });

  const {
    addCalculatedColumnRequest,
    transformColumnRequest,
    filterColumnRequest,
    hidePipelineColumnRequest,
    rearrangeColumnsRequest,
    applyVectorRequest,
    pairVectorRequest,
    clearAddCalculatedColumnRequest,
    clearTransformColumnRequest,
    clearFilterColumnRequest,
    clearHidePipelineColumnRequest,
    clearRearrangeColumnsRequest,
    clearApplyVectorRequest,
    clearPairVectorRequest,
    requestAddCalculatedColumn,
    requestColumnTransformation,
    requestColumnFill,
    requestColumnFilter,
    requestCalculatedColumnEdit,
    requestHidePipelineColumn,
    requestRearrangeColumns,
    requestApplyVector,
    requestPairVector,
    transformColumnToken,
    setTransformColumnRequest,
  } = usePipelineColumnRequests({
    setContextMenu,
    setSelection,
    setInspectorSection,
  });

  const chooseVectorLayout = useCallback(
    async (mode: "hstack" | "vstack") => {
      if (!document || !vectorCombine) return;
      const choice = vectorCombine;
      setVectorCombine(null);
      if (mode === "hstack") {
        requestPairVector(
          choice.frameId,
          choice.vector.name,
          choice.vector.formula,
          choice.vector.length,
          choice.viewId
        );
        return;
      }
      try {
        const applyAndShow = async (operation: Operation) => {
          const next = await applyOperation(operation);
          setDocument(next);
          return next;
        };
        await combineVectorWithFrame(document, choice, applyAndShow);
        setSelection({ objectId: choice.frameId, viewId: choice.viewId });
        setInspectorSection("wrangle");
        setError(null);
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
      }
    },
    [document, requestPairVector, vectorCombine]
  );

  /**
   * Dropping a column by whichever gesture its frame allows. Named by frame
   * and column rather than read off the context menu, so the right-click item
   * and Quick Commands reach the same decision rather than each making it.
   */
  const removeColumn = (frameId: string, columnId: string, viewId?: string) => {
    if (!document) return;
    const computed = document.computedFrames[frameId];
    if (computed?.editing.rows) {
      deleteFromContext({ type: "deleteColumn", frameId, columnId });
      return;
    }
    // A computed or source-backed grid cannot delete its input data. Its
    // equivalent gesture is the same one the chain already exposes: leave
    // this column out of the final Select. Put that request through the open
    // editor so its local draft and the saved pipeline change together.
    requestHidePipelineColumn(frameId, columnId, viewId);
  };

  const deleteContextColumn = () => {
    if (!contextFrame || !contextColumn) return;
    removeColumn(contextFrame.id, contextColumn.id, contextMenu?.viewId);
  };

  const {
    freeze,
    refreshConnector,
    changeFrameSource,
    refreshingSnapshots,
    refreshStale,
    takeOwnership,
    updateOriginalFile,
    exportFrameFile,
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
  useEffect(() => {
    if (!pendingJoinSelect || !document) return;
    const created = document.objects.find(
      (object): object is FrameObject =>
        object.kind === "frame" && !pendingJoinSelect.has(object.id)
    );
    if (!created) return;
    setPendingJoinSelect(null);
    const view = document.views.find((candidate) => candidate.objectId === created.id);
    if (view) {
      const size = frameCardSize(created.columns.length, created.rows.length || 6);
      void run({ type: "resizeView", viewId: view.id, ...size });
    }
    setSelection({ objectId: created.id, viewId: view?.id });
    setInspectorSection("wrangle");
    const canvas = canvasRef.current;
    if (view && canvas) {
      const scroll = revealScroll(canvas, view, canvasZoomRef.current);
      if (scroll) canvas.scrollTo({ ...scroll, behavior: "smooth" });
    }
  }, [canvasZoomRef, document, pendingJoinSelect, run]);

  const jumpToObject = useCallback(
    (objectId: string) => {
      if (!document) return;
      const view =
        document.views.find((candidate) => candidate.objectId === objectId) ??
        document.views.find((candidate) => candidate.tabObjectIds?.includes(objectId));
      setSelection({ objectId, viewId: view?.id });
      setInspectorSection("wrangle");
      if (!view) return;
      if (view.objectId !== objectId) {
        void run({ type: "setActiveTab", viewId: view.id, objectId });
      }
      // Only if it is not already in view, and then centred: a card in plain
      // sight stays put, and one that has to be fetched arrives in the
      // middle rather than in a corner of an otherwise empty screen.
      const canvas = canvasRef.current;
      if (!canvas) return;
      const scroll = revealScroll(canvas, view, canvasZoomRef.current);
      if (scroll) canvas.scrollTo({ ...scroll, behavior: "smooth" });
    },
    [canvasZoomRef, document, run]
  );

  const insertPosition = useInsertPosition(document, canvasRef, canvasZoomRef);

  const fitViewToWindow = useFitViewToWindow(canvasRef, canvasZoomRef, run);

  const {
    scratchTargetId,
    scratchworkBlock,
    scratchworkBarReferences,
    scratchworkWindowOpen,
    summonScratchpad,
    openScratchworkPopout,
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
    clearActiveFormulaEditor,
    setError,
  });
  const {
    active: formulaEditorActive,
    getActive: getFormulaEditor,
    insertReference: insertFormulaReference,
    replaceSelection: replaceFormulaSelection,
    cancel: cancelFormulaEditor,
    clear: clearFormulaEditor,
    disengage: disengageFormulaEditor,
  } = useScratchworkWindowBridge({
    local: formulaCommands,
    localFocused: localFormulaEditorActive,
    block: scratchworkBlock,
    references: scratchworkBarReferences,
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
    document,
    selection,
    containedIds,
    canvasRef,
    canvasZoomRef,
    setSelection,
    setGridFocus,
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

  // Paste with nothing selected lands the clipboard on the canvas as a new
  // frame: the empty-frame paste without the empty frame.
  const { handleCanvasPaste } = useCanvasClipboard({
    document,
    gridFocus,
    run,
    insertPosition,
  });

  // These are application actions, not menu actions: the native menu, the
  // menu-less shell, and Quick Commands all enter through the same table.
  // The update check still runs once on open; its offer owns the modal layer.
  const updates = useUpdateCheck();
  const selectedCommandView = selectedCanvasView(document, selection);
  const menuHandlers: Record<string, () => void> = {
    "new-window": () => void newWindow().catch((reason) => setError(String(reason))),
    "new-document": () => setNewDocumentOpen(true),
    "open-document": () => void handleOpenDocument(),
    "save-document-as": () => void handleSaveAsDocument(),
    "package-document": () => void packageThisDocument(),
    "compact-data": () => void compactData(),
    preferences: () => {
      setPreferencesPage("settings");
      setPreferencesOpen(true);
    },
    "keyboard-shortcuts": () => {
      setPreferencesPage("shortcuts");
      setPreferencesOpen(true);
    },
    find: () => {
      setFindQuery("");
      setFindOpen(true);
    },
    "quick-commands": () => {
      setHelpScope(null);
      setFindOpen(false);
      setQuickCommandsOpen(true);
    },
    reference: () => {
      setQuickCommandsOpen(false);
      setHelpScope(getFormulaEditor() ? "formulas" : lastHelpScope);
    },
    "check-for-updates": () => updates.check(),
    undo: () => void navigateHistory("undo"),
    redo: () => void navigateHistory("redo"),
    "data-library": () => setDatasetLibrary(true),
    "toggle-sources": () => setLeftPanel((panel) => (panel === "data" ? null : "data")),
    "tidy-layout": () => void run({ type: "tidyLayout" }),
    "fit-view": () => withCanvasView(selectedCommandView, fitViewToWindow),
    "collapse-view": () =>
      withCanvasView(
        selectedCommandView,
        (view) =>
          void run({
            type: "setViewCollapsed",
            viewId: view.id,
            collapsed: !view.collapsed,
          })
      ),
    "inspector-toggle": () => setInspectorSection({ panel: "toggle" }),
    "inspector-selection": () => setInspectorSection("selection"),
    "inspector-format": () => setInspectorSection("format"),
    "inspector-wrangle": () => setInspectorSection("wrangle"),
    "add-variable": () => void addVariable(),
    "add-block": () => void addBlock(),
    "add-text": () => void addText(),
    "add-matrix": () => void addCalculationMatrix(),
    "canvas-only": () => setLeftPanel(null),
    "add-frame": () => void addEmptyFrame(),
    "add-container": () => void addContainer(),
    scratchpad: () => void summonScratchpad(),
    "open-scratchwork-window": () => void openScratchworkPopout(),
    "zoom-in": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, 1)),
    "zoom-out": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, -1)),
    "zoom-reset": () => zoomCanvas(DEFAULT_CANVAS_ZOOM),
  };
  // What ⌘⇧P offers about the selection. Every handler is the one the
  // right-click menu, the column header or the menu itself already calls —
  // the palette is another way in, not a second implementation.
  const selectionActions: SelectionActions = {
    rename: (objectId) => {
      jumpToObject(objectId);
      // Renaming happens in the card's own name field, the way it does from
      // the canvas; the palette only puts the cursor in it.
      requestAnimationFrame(() => {
        const card = `[data-object-id="${CSS.escape(objectId)}"]`;
        window.document
          .querySelector<HTMLInputElement>(
            `${card} input.frame-name, ${card} input.object-name-input`
          )
          ?.select();
      });
    },
    remove: (objectId) => deleteFromContext({ type: "deleteObject", objectId }),
    fitToWindow: () => menuHandlers["fit-view"](),
    toggleCollapsed: () => menuHandlers["collapse-view"](),
    createFrameFrom: (frameId) => {
      const frame = document?.objects.find((object) => object.id === frameId);
      const view = document?.views.find((candidate) => candidate.objectId === frameId);
      void run({
        type: "addLinkedFrame",
        sourceFrameId: frameId,
        name: `${frame?.name ?? "Frame"} frame`,
        x: (view?.x ?? 0) + 28,
        y: (view?.y ?? 0) + 28,
      });
    },
    sortColumn: (frameId, columnId, descending) =>
      void run({
        type: "setFrameDisplaySort",
        frameId,
        keys: [{ columnId, descending }],
      }),
    clearColumnSort: (frameId, columnId) =>
      void run({
        type: "setFrameDisplaySort",
        frameId,
        keys: pipelineSortKeys(document?.computedFrames[frameId]).filter(
          (key) => key.columnId !== columnId
        ),
      }),
    pinColumns: (frameId, pinnedColumns) =>
      void run({ type: "setFrameDisplayPinnedColumns", frameId, pinnedColumns }),
    removeColumn: (frameId, columnId) =>
      removeColumn(frameId, columnId, selection?.viewId),
  };

  /** Opening a recent from the palette lands the same way the library does. */
  const openRecentDocument = (path: string) => {
    void openDocument(path)
      .then(adoptOpenedDocument)
      .catch((reason) => setError(String(reason).replace(/^Error:\s*/, "")));
  };

  // The recents are a device-level list, read when the palette opens rather
  // than kept in step with a document nothing here is waiting on.
  useEffect(() => {
    if (!quickCommandsOpen) return;
    let disposed = false;
    void listRecentDocuments()
      .then((items) => {
        if (!disposed) setRecents(items);
      })
      .catch(reportIgnoredFailure("recent documents for Quick Commands"));
    return () => {
      disposed = true;
    };
  }, [quickCommandsOpen]);

  const menuHandlersRef = useRef(menuHandlers);
  menuHandlersRef.current = menuHandlers;
  useApplicationMenu(receivesMenuCommands(), menuHandlers, setError);

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
        const command = shortcutCommandId(shortcut);
        if (command) menuHandlersRef.current[command]?.();
        return;
      }
      if (isTextEntryTarget(event.target)) return;
      if (contextMenu || preferencesOpen) return;
      // Canvas-level Escape for the formula session. The editor surfaces
      // handle their own Escape while they hold the keyboard; this is the
      // fallback for a session whose surface lost focus — without it, Escape
      // pressed over the canvas ended nothing and the session had no exit.
      if (event.key === "Escape" && getFormulaEditor()) {
        cancelFormulaEditor();
        return;
      }
      if (gridFocus?.mode === "navigate") {
        handleNavigateKey(event);
        return;
      }
      if (selection && keyboardBelongsToCanvas(event.target)) {
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
    window.addEventListener("paste", handleCanvasPaste);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("copy", handleGridCopy);
      window.removeEventListener("cut", handleGridCut);
      window.removeEventListener("paste", handleGridPaste);
      window.removeEventListener("paste", handleCanvasPaste);
    };
  }, [
    cancelFormulaEditor,
    canvasZoomRef,
    contextMenu,
    document,
    documentPath,
    fitViewToWindow,
    getFormulaEditor,
    gridFocus,
    handleCanvasPaste,
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

  useHistoryMenuState(document?.canUndo, document?.canRedo);

  /** Pressing the rail button for the panel already open closes it. */
  const toggleLeftPanel = (panel: Exclude<LeftPanel, null>) =>
    setLeftPanel((current) => (current === panel ? null : panel));

  const {
    addBlock,
    addVariable,
    addCalculationMatrix,
    addText,
    addEmptyFrame,
    addContainer,
  } = useCanvasObjectCreation({ run, document, insertPosition });

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
        {!error && splashSlow && (
          <p>
            Still opening. If macOS is asking for permission to read your Documents
            folder, allow it to continue.
          </p>
        )}
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
        document.views.reduce(
          (right, view) => Math.max(right, view.x + view.width),
          0
        ) +
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
        ) +
          Math.max(320, viewportSize.height * 0.7) / canvasZoom
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
  // One answer for the panel and for the canvas that makes room for it.
  const showInspector = inspectorShown(inspectorPanel, selectedObject, selection);
  const showCollapsedInspector =
    inspectorPanel.hidden && inspectorAvailable(selectedObject, selection);
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
    ? resolveGridContext(document, gridFocus, focusedGrids)
    : null;
  // Every column the grid selection covers, in frame order, so the inspector
  // can format a range of columns as one gesture rather than the active one.
  // Held across a render where the card has no rows to resolve against —
  // see `stableColumnRange` for why that is not a narrowing.
  const columnRange = stableColumnRange(
    columnRangeRef.current,
    columnRangeKey(gridFocus),
    selectedGridContext && gridFocus
      ? selectedGridColumnIds(selectedGridContext, gridFocus)
      : []
  );
  columnRangeRef.current = columnRange;
  const selectedColumnIds = columnRange.ids;
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
          (reference) => reference.kind !== "frame" && reference.kind !== "column"
        ),
      ]
    : [];
  const handleFormulaPointerDown = canvasFormulaPointerHandler({
    document,
    getActive: getFormulaEditor,
    insertReference: insertFormulaReference,
    clear: clearFormulaEditor,
    disengage: disengageFormulaEditor,
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
        <SafeModeBanner
          document={document}
          onDocument={setDocument}
          onError={setError}
        />
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
          onOpenQuickCommands={menuHandlers["quick-commands"]}
          addBlock={addBlock}
          addVariable={addVariable}
          addText={addText}
          addCalculationMatrix={addCalculationMatrix}
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
          className={`canvas-viewport ${leftPanel ? "with-panel" : ""}`}
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
              if (!panning) {
                setSelection(null);
                // The click moved the focus to the body, where the native
                // Paste command has nothing to deliver to. Re-arm the target
                // so ⌘V on the bare canvas still makes a frame.
                focusGridClipboardTarget();
              }
            };
            element.classList.add("panning");
            window.addEventListener("pointermove", move);
            window.addEventListener("pointerup", end);
            window.addEventListener("pointercancel", end);
          }}
          onDragOver={(event) => {
            if (
              hasVectorDrag(event.dataTransfer) &&
              !(event.target as HTMLElement).closest(".canvas-object")
            ) {
              event.preventDefault();
              event.dataTransfer.dropEffect = "link";
              return;
            }
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
            const vector = readVectorDrag(event.dataTransfer);
            if (vector) {
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
              setSelection(null);
              void run({
                type: "addGeneratorFrame",
                name: `${vector.name} table`,
                formula: vector.formula,
                columnName: vector.name,
                x: Math.max(0, dropped.x - 100),
                y: Math.max(0, dropped.y - 16),
              });
              return;
            }
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
                  computedValues={document.computedValues ?? {}}
                  computedBlocks={document.computedBlocks}
                  computedTexts={document.computedTexts}
                  computedCalculationMatrices={document.computedCalculationMatrices}
                  scratchFocusToken={
                    object.id === scratchTargetId ? scratchFocus?.token : undefined
                  }
                  scratchworkElsewhere={
                    (scratchworkDrawerOpen || scratchworkWindowOpen) &&
                    object.id === scratchworkBlock?.id
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
                  onApplyVector={(frame, columnIds, vector, expectedLength) =>
                    requestApplyVector(
                      frame.id,
                      columnIds,
                      vector,
                      expectedLength,
                      view.id
                    )
                  }
                  onPairVector={(frame, name, vector, expectedLength) => {
                    const computed = document.computedFrames[frame.id];
                    if (
                      frame.columns.length === 1 &&
                      computed?.generatorRule &&
                      (computed.steps?.length ?? 0) === 0
                    ) {
                      setVectorCombine({
                        frameId: frame.id,
                        viewId: view.id,
                        vector: { name, formula: vector, length: expectedLength },
                      });
                      return;
                    }
                    requestPairVector(frame.id, name, vector, expectedLength, view.id);
                  }}
                  onJoinColumns={(
                    primaryFrameId,
                    primaryColumnId,
                    lookupFrameId,
                    lookupOutputColumnIds
                  ) => {
                    // Where a new card lands: in free space inside the
                    // viewport, not past the primary frame's right edge,
                    // which was off screen and behind the inspector.
                    setJoin({
                      primaryFrameId,
                      lookupFrameId,
                      primaryKeyId: primaryColumnId,
                      lookupOutputColumnIds,
                      ...insertPosition(CARD_SIZES.frame),
                    });
                  }}
                  onFilterColumn={(frame, column) =>
                    requestColumnFilter(frame, column, view.id)
                  }
                  onTransformColumn={(frame, column, formula) =>
                    requestColumnFill(frame, column, formula, undefined, view.id)
                  }
                  onEditCalculatedColumn={(frame, column, rowIndex) =>
                    requestCalculatedColumnEdit(frame, column, rowIndex, view.id)
                  }
                  onTakeOwnership={takeOwnership}
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
          withCollapsedInspector={showCollapsedInspector}
          document={document}
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
        {inspectorAvailable(selectedObject, selection) && selection && selectedObject && (
          <Inspector
            hidden={!showInspector}
            documentId={document.id}
            object={selectedObject}
            objects={document.objects}
            scenarios={document.scenarios ?? []}
            formulaFunctions={document.formulaFunctions}
            selection={selection}
            selectedColumnIds={selectedColumnIds}
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
            onHide={() => setInspectorSection({ panel: "hide" })}
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
            applyVectorRequest={scopedPipelineRequest(
              applyVectorRequest,
              selectedObject.id
            )}
            onApplyVectorRequestHandled={clearApplyVectorRequest}
            pairVectorRequest={scopedPipelineRequest(
              pairVectorRequest,
              selectedObject.id
            )}
            onPairVectorRequestHandled={clearPairVectorRequest}
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
        {showCollapsedInspector && (
          <CollapsedInspector onShow={() => setInspectorSection({ panel: "toggle" })} />
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

        {vectorCombine && (
          <VectorCombinePrompt
            state={vectorCombine}
            onChoose={(mode) => void chooseVectorLayout(mode)}
            onClose={() => setVectorCombine(null)}
          />
        )}

        {join && (
          <JoinDialog
            state={join}
            document={document}
            onClose={() => setJoin(null)}
            onOperation={run}
            onCreated={() => {
              setPendingJoinSelect(
                new Set(document.objects.map((object) => object.id))
              );
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

        {findOpen && (
          <FindPaletteHost
            document={document}
            initialQuery={findQuery}
            jumpToObject={jumpToObject}
            setSelection={setSelection}
            setGridFocus={setGridFocus}
            onClose={() => setFindOpen(false)}
          />
        )}

        <QuickCommands
          open={quickCommandsOpen}
          commands={quickCommandItems(
            menuHandlers,
            {
              canUndo: document.canUndo,
              canRedo: document.canRedo,
              hasSelectedView: Boolean(selectedCommandView),
              selection: quickCommandSelection(document, selection),
              recents: recents.map(({ path, title }) => ({ path, title })),
            },
            { selection: selectionActions, openRecent: openRecentDocument }
          )}
          onSearchDocument={(query) => {
            setFindQuery(query);
            setFindOpen(true);
          }}
          onClose={() => setQuickCommandsOpen(false)}
        />

        {helpScope && (
          <HelpBrowser
            scope={helpScope}
            formulaFunctions={document.formulaFunctions}
            canInsert={Boolean(getFormulaEditor())}
            onScopeChange={(scope) => {
              setLastHelpScope(scope);
              setHelpScope(scope);
            }}
            onInsert={(formula) => {
              replaceFormulaSelection(formula);
              setHelpScope(null);
            }}
            onClose={() => setHelpScope(null)}
          />
        )}

        {updates.status.kind !== "idle" && (
          <UpdateDialog
            status={updates.status}
            progress={updates.progress}
            onInstall={updates.install}
            onSkip={updates.skip}
            onDismiss={updates.dismiss}
          />
        )}

        {newDocumentOpen && (
          <NewDocumentDialog
            onClose={() => setNewDocumentOpen(false)}
            onOpened={(opened) => {
              adoptOpenedDocument(opened);
              setNewDocumentOpen(false);
            }}
          />
        )}

        {datasetLibrary && updates.status.kind === "idle" && (
          <DatasetDialog
            document={document}
            onSourceChanged={changeFrameSource}
            onClose={() => setDatasetLibrary(false)}
            onImportFile={async () => {
              const position = insertPosition(CARD_SIZES.frame);
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
              setExcelImport({
                workbook,
                position: insertPosition(CARD_SIZES.frame),
              });
              setDatasetLibrary(false);
              return true;
            }}
            onImportCliSource={async (source) => {
              const next = await importCliSource(
                insertPosition(CARD_SIZES.frame),
                source
              );
              setDocument(next);
              setDatasetLibrary(false);
              setNotice(`Connected ${source.sourceLabel}.`);
            }}
            onImportDatabaseSource={async (source) => {
              const next = await importDatabaseSource(
                insertPosition(CARD_SIZES.frame),
                source
              );
              setDocument(next);
              setDatasetLibrary(false);
              setNotice(`Connected ${source.sourceName}.`);
            }}
            onOpened={(opened) => {
              adoptOpenedDocument(opened);
              setDatasetLibrary(false);
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
              setNotice(
                `Imported ${selection.name} from ${selection.sheetName}!${selection.cellRange}.`
              );
              if (another) {
                // The frame just imported now occupies `current.position`, so
                // re-anchoring there and running it back through the shared
                // helper (against `next.views`, which already includes that
                // new card) finds the next free spot instead of the flat +42
                // nudge this used to apply blindly, which could still land a
                // second or third range on top of an unrelated card.
                setExcelImport((current) =>
                  current
                    ? {
                        ...current,
                        position: placeNewCard(
                          next.views,
                          current.position,
                          CARD_SIZES.frame
                        ),
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
          <CanvasContextMenu
            updateOriginalFile={updateOriginalFile}
            exportFrameFile={exportFrameFile}
            contextMenu={contextMenu}
            contextKind={contextKind}
            contextObject={contextObject}
            contextFrame={contextFrame}
            contextColumn={contextColumn}
            contextGrid={contextGrid}
            contextGenerator={contextGenerator}
            contextEntryKey={contextEntryKey}
            contextCrosstabValues={contextCrosstabValues}
            contextIsMaterialized={contextIsMaterialized}
            gridFocus={gridFocus}
            document={document}
            containers={containers}
            containedIds={containedIds}
            copyIncludesHeaders={copyIncludesHeaders}
            copySelection={copySelection}
            copyColumnReference={copyColumnReference}
            addBlock={addBlock}
            addText={addText}
            addCalculationMatrix={addCalculationMatrix}
            addEmptyFrame={addEmptyFrame}
            addContainer={addContainer}
            askOnImport={askOnImport}
            importMode={importMode}
            runAppendImport={runAppendImport}
            deleteContextColumn={deleteContextColumn}
            deleteFromContext={deleteFromContext}
            requestAddCalculatedColumn={requestAddCalculatedColumn}
            requestCalculatedColumnEdit={requestCalculatedColumnEdit}
            requestColumnTransformation={requestColumnTransformation}
            run={run}
            setAppendImport={setAppendImport}
            setContextMenu={setContextMenu}
            setError={setError}
            setGridFocus={setGridFocus}
            setInspectorSection={setInspectorSection}
            setJoin={setJoin}
            setNotice={setNotice}
            setRecurrence={setRecurrence}
            setRunningCalculation={setRunningCalculation}
            setSelection={setSelection}
            setSequenceFill={setSequenceFill}
          />
        )}
      </div>
    </NumberDisplayContext.Provider>
  );
}
