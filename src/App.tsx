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
import { UpdateDialog } from "./UpdateDialog";
import { useApplicationMenu } from "./useApplicationMenu";
import { useFitViewToWindow } from "./useFitViewToWindow";
import { useCanvasNavigation } from "./useCanvasNavigation";
import { useThousandsSeparatorsPreference } from "./hooks/useThousandsSeparatorsPreference";
import { useInterfaceScalePreference } from "./hooks/useInterfaceScalePreference";
import { useMcpSettings } from "./hooks/useMcpSettings";
import {
  useCanvasObjectCreation,
  nextContainerName,
  nextObjectName,
} from "./hooks/useCanvasObjectCreation";
import {
  usePipelineColumnRequests,
  scopedPipelineRequest,
} from "./hooks/usePipelineColumnRequests";
import { useContextMenu } from "./hooks/useContextMenu";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useScratchwork } from "./hooks/useScratchwork";
import { useImportFlow } from "./hooks/useImportFlow";
import { useCanvasClipboard } from "./hooks/useCanvasClipboard";
import { useGridClipboard } from "./hooks/useGridClipboard";
import { useCanvasViewport } from "./hooks/useCanvasViewport";
import { useDocumentLifecycle } from "./hooks/useDocumentLifecycle";
import { useConnectorRefreshApproval } from "./hooks/useConnectorRefreshApproval";
import { ConnectorRefreshConfirmDialog } from "./ConnectorRefreshConfirmDialog";
import { useGridKeyboardNavigation } from "./hooks/useGridKeyboardNavigation";
import { useDocumentBootstrap } from "./hooks/useDocumentBootstrap";
import { CollapsedInspector, Inspector } from "./Inspector";
import { JoinDialog } from "./JoinDialog";
import { DataSidebar } from "./DataSidebar";
import { LeftRail } from "./LeftRail";
import {
  CARD_SIZES,
  placeNewCard,
  type CardSize,
  type Rect,
} from "./lib/cardPlacement";
import {
  inspectorAvailable,
  inspectorPanelReducer,
  inspectorShortcutAction,
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
  isTextEntryTarget,
  resolveGridContext,
  tabObjects,
  visualGridPosition,
  type ContextMenuState,
  type GridFocus,
  type RenderedGrid,
} from "./FrameGrid";
import { hasFrameTabDrag, readFrameTabDrag } from "./FrameViewTabs";
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
import { reportIgnoredFailure } from "./lib/errorReporting";
import { selectedCanvasView, withCanvasView } from "./lib/canvasNavigation";
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
// The anchor a fresh import prefers — just inside the viewport's corner —
// displaced only if a card is already sitting there. Importing a CSV used
// to land the new frame exactly on top of an existing card (Frame 1, most
// often, since both computed the same fixed point), hiding it outright.
function importPosition(
  viewport: HTMLDivElement | null,
  existingViews: Rect[]
): { x: number; y: number } {
  const anchor = {
    x: (viewport?.scrollLeft ?? 0) + 110,
    y: (viewport?.scrollTop ?? 0) + 100,
  };
  return placeNewCard(existingViews, anchor, CARD_SIZES.frame);
}

export default function App() {
  const formulaEditorActive = useActiveFormulaEditorPresence();
  const {
    commit: commitActiveFormulaEditor,
    disengage: disengageActiveFormulaEditor,
    getActive: getActiveFormulaEditor,
    insertReference: insertActiveFormulaReference,
    replaceSelection: replaceActiveFormulaSelection,
    cancel: cancelActiveFormulaEditor,
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
  const [findOpen, setFindOpen] = useState(false);
  const [sequenceFill, setSequenceFill] = useState<SequenceFillState | null>(null);
  const [runningCalculation, setRunningCalculation] =
    useState<RunningCalculationState | null>(null);
  const [recurrence, setRecurrence] = useState<RecurrenceState | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  // One reducer rather than a hidden flag beside the section, so the
  // dispatch stays the stable section setter every hook already takes, and
  // asking for a section is what un-hides the panel (see inspectorPanel.ts).
  const [inspectorPanel, setInspectorSection] = useReducer(inspectorPanelReducer, {
    hidden: false,
    section: "selection",
  });
  const inspectorSection = inspectorPanel.section;
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

  const { canvasZoom, canvasZoomRef, zoomCanvas, viewportSize } = useCanvasViewport({
    canvasRef,
    documentOpened: document !== null,
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
        document.views.find((candidate) => candidate.tabObjectIds?.includes(objectId));
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
  // That corner is only where a new card *prefers* to land: every caller
  // used to get exactly that point regardless of what already sat there,
  // which is how clicking Matrix right after an Arrange dropped the new
  // card behind Frame 1 instead of beside it. `placeNewCard` checks the
  // preferred spot against the document's current views and steps aside
  // only if something is actually in the way — see `src/lib/cardPlacement.ts`.
  // `size` defaults to a Block's footprint since most no-argument callers
  // (Scratchwork's block, in particular) are creating one.
  const insertPosition = useCallback(
    (size: CardSize = CARD_SIZES.block) => {
      const anchor = {
        x: ((canvasRef.current?.scrollLeft ?? 0) + 110) / canvasZoomRef.current,
        y: ((canvasRef.current?.scrollTop ?? 0) + 100) / canvasZoomRef.current,
      };
      return placeNewCard(document?.views ?? [], anchor, size);
    },
    [canvasZoomRef, document]
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
        else if (shortcut === "add-block")
          void run({
            type: "addBlock",
            name: nextObjectName(document?.objects ?? [], "Block"),
            ...insertPosition(CARD_SIZES.block),
          });
        else if (shortcut === "add-text")
          void run({ type: "addText", ...insertPosition(CARD_SIZES.text) });
        else if (shortcut === "add-frame")
          void run({
            type: "addFrame",
            name: "Frame 1",
            grid: [
              ["Column 1", "Column 2"],
              ["", ""],
              ["", ""],
            ],
            ...insertPosition(CARD_SIZES.frame),
          });
        else if (shortcut === "add-container")
          void run({
            type: "addContainer",
            name: nextContainerName(document?.objects ?? []),
            ...insertPosition(CARD_SIZES.container),
          });
        else if (shortcut.startsWith("inspector-"))
          setInspectorSection(inspectorShortcutAction(shortcut));
        else if (shortcut === "arrange") {
          void run({ type: "tidyLayout" });
        } else if (shortcut === "fit" || shortcut === "collapse") {
          const selectedView = selectedCanvasView(document, selection);
          if (selectedView) {
            if (shortcut === "fit") void fitViewToWindow(selectedView);
            else
              void run({
                type: "setViewCollapsed",
                viewId: selectedView.id,
                collapsed: !selectedView.collapsed,
              });
          }
        } else if (shortcut === "find") {
          setFindOpen(true);
        } else if (shortcut === "library") {
          setDatasetLibrary(true);
        } else if (shortcut === "shortcuts") {
          setPreferencesPage("shortcuts");
          setPreferencesOpen(true);
        } else if (shortcut === "formula-help") {
          setHelpScope("formulas");
        } else if (shortcut === "framework-help") {
          setHelpScope("guide");
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
      // Canvas-level Escape for the formula session. The editor surfaces
      // handle their own Escape while they hold the keyboard; this is the
      // fallback for a session whose surface lost focus — without it, Escape
      // pressed over the canvas ended nothing and the session had no exit.
      if (event.key === "Escape" && getActiveFormulaEditor()) {
        cancelActiveFormulaEditor();
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
    cancelActiveFormulaEditor,
    canvasZoomRef,
    contextMenu,
    document,
    documentPath,
    fitViewToWindow,
    getActiveFormulaEditor,
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

  // Checks once when this window opens, throttled across windows and
  // launches; the menu item asks outright and bypasses that.
  const updates = useUpdateCheck();

  const selectedCommandView = selectedCanvasView(document, selection);
  useApplicationMenu(
    hasNativeMenu(),
    {
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
      find: () => setFindOpen(true),
      "formula-help": () => setHelpScope("formulas"),
      "framework-help": () => setHelpScope("guide"),
      "check-for-updates": () => updates.check(),
      undo: () => void navigateHistory("undo"),
      redo: () => void navigateHistory("redo"),
      "data-library": () => setDatasetLibrary(true),
      "toggle-sources": () =>
        setLeftPanel((panel) => (panel === "data" ? null : "data")),
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
      "add-block": () => void addBlock(),
      "add-text": () => void addText(),
      "add-frame": () => void addEmptyFrame(),
      "add-container": () => void addContainer(),
      scratchpad: () => void summonScratchpad(),
      "zoom-in": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, 1)),
      "zoom-out": () => zoomCanvas(nudgeCanvasZoom(canvasZoomRef.current, -1)),
      "zoom-reset": () => zoomCanvas(DEFAULT_CANVAS_ZOOM),
    },
    setError
  );

  // Undo and Redo grey out with the document's history. Nothing else tells the
  // menu, so every view that arrives pushes it — including the first, which is
  // what leaves both disabled on a document opened fresh. A menu that will not
  // take the news is not worth an error banner over.
  useEffect(() => {
    if (!hasNativeMenu() || !document) return;
    void setHistoryMenuState(document.canUndo, document.canRedo).catch(
      reportIgnoredFailure("history menu state")
    );
  }, [document?.canUndo, document?.canRedo]);

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
          (reference) => reference.kind !== "frame" && reference.kind !== "column"
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
          className={`canvas-viewport ${showInspector ? "with-inspector" : ""} ${
            showCollapsedInspector ? "with-inspector-collapsed" : ""
          } ${leftPanel ? "with-panel" : ""}`}
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
                    const primaryView = document.views.find(
                      (candidate) => candidate.objectId === primaryFrameId
                    );
                    setJoin({
                      primaryFrameId,
                      lookupFrameId,
                      primaryKeyId: primaryColumnId,
                      lookupOutputColumnIds,
                      x: primaryView
                        ? primaryView.x + primaryView.width + 100
                        : view.x + view.width + 100,
                      y: primaryView?.y ?? view.y,
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
        {showInspector && selection && selectedObject && (
          <Inspector
            documentId={document.id}
            object={selectedObject}
            objects={document.objects}
            scenarios={document.scenarios ?? []}
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
            jumpToObject={jumpToObject}
            setSelection={setSelection}
            setGridFocus={setGridFocus}
            onClose={() => setFindOpen(false)}
          />
        )}

        {helpScope && (
          <HelpBrowser
            scope={helpScope}
            formulaFunctions={document.formulaFunctions}
            canInsert={Boolean(getActiveFormulaEditor())}
            onScopeChange={setHelpScope}
            onInsert={(formula) => {
              replaceActiveFormulaSelection(formula);
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
              const position = importPosition(viewport, document.views);
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
                position: importPosition(viewport, document.views),
              });
              setDatasetLibrary(false);
              return true;
            }}
            onImportCliSource={async (source) => {
              const viewport = canvasRef.current;
              const next = await importCliSource(
                importPosition(viewport, document.views),
                source
              );
              setDocument(next);
              setDatasetLibrary(false);
              setNotice(`Connected ${source.sourceLabel}.`);
            }}
            onImportDatabaseSource={async (source) => {
              const viewport = canvasRef.current;
              const next = await importDatabaseSource(
                importPosition(viewport, document.views),
                source
              );
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
