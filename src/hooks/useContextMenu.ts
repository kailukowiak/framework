import { useCallback, type Dispatch, type RefObject, type SetStateAction } from "react";
import {
  resolveGridContext,
  type ContextMenuState,
  type GridFocus,
  type RenderedGrid,
} from "../FrameGrid";
import { canvasPoint } from "../lib/canvasZoom";
import { inferContextGenerator } from "../lib/contextGeneratorInference";
import type { DocumentView, FrameObject, Operation, Selection } from "../lib/types";

/**
 * Everything the right-click menu needs to know about what it was opened
 * on, derived once from `contextMenu` rather than re-resolved by every
 * menu item: which object and frame it named, the column and cell under
 * the pointer if any, and the handful of facts (is it materialized, does
 * it have a unique key, what would spreading fill) that decide which menu
 * items make sense to offer.
 *
 * Takes `contextMenu`/`setContextMenu` as input rather than owning that
 * state itself: several effects elsewhere in App need to clear the menu
 * (on Escape, on a document swap, ...) and are declared earlier in the
 * component than this hook can safely be called, since it needs `run` for
 * deleteFromContext. Keeping the state a plain useState at the top of App
 * lets those early effects see it without caring when this hook runs.
 *
 * Deleting a column is not included here even though it starts from a
 * context-menu click: on a computed or source-backed frame it hands off to
 * a pipeline "hide this column" request, which lives in
 * usePipelineColumnRequests — composing the two is left to the caller.
 */
export function useContextMenu({
  contextMenu,
  setContextMenu,
  document,
  gridFocus,
  renderedRows,
  canvasRef,
  canvasZoomRef,
  setSelection,
  run,
}: {
  contextMenu: ContextMenuState | null;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  document: DocumentView | null;
  gridFocus: GridFocus | null;
  renderedRows: RefObject<Map<string, RenderedGrid>>;
  canvasRef: RefObject<HTMLDivElement | null>;
  canvasZoomRef: RefObject<number>;
  setSelection: (value: Selection | null) => void;
  run: (operation: Operation) => Promise<string | null>;
}) {
  const contextObject =
    document && contextMenu?.objectId
      ? (document.objects.find((object) => object.id === contextMenu.objectId) ?? null)
      : null;
  const contextFrame = !document
    ? null
    : contextMenu?.frameId
      ? (document.objects.find(
          (object): object is FrameObject =>
            object.kind === "frame" && object.id === contextMenu.frameId
        ) ?? null)
      : contextObject?.kind === "frame"
        ? contextObject
        : null;
  const contextColumn =
    contextFrame?.columns.find((column) => column.id === contextMenu?.columnId) ?? null;
  /** Whether the frame under the pointer already holds a snapshot — which is
   * what decides whether another frame may read from it. */
  const contextIsMaterialized = Boolean(
    document && contextFrame && document.computedFrames[contextFrame.id]?.materialization
  );
  const contextGrid =
    document && contextFrame && gridFocus?.objectId === contextFrame.id
      ? resolveGridContext(document, gridFocus, renderedRows.current)
      : null;
  const contextKind = contextFrame
    ? contextMenu?.rowId && contextColumn
      ? "Cell"
      : contextMenu?.rowId
        ? "Row"
        : contextColumn
          ? "Column"
          : "Frame"
    : contextObject
      ? contextObject.kind[0].toUpperCase() + contextObject.kind.slice(1)
      : "Canvas";

  // The Excel gesture read the Excel way: a selected run declares the
  // series' start and step, while this frame's row count declares its end.
  // The resulting frame.len()-bound calculation fills this column in place;
  // other columns already contribute the rectangular rows and therefore
  // remain blank/null wherever they have no value.
  const contextGenerator = inferContextGenerator(
    contextFrame,
    contextColumn,
    gridFocus,
    contextGrid,
    contextMenu?.rowIndex
  );
  // An entry column is offered where typing is refused and a unique key
  // says what a row *is* — the two facts that make keyed storage the only
  // honest place for a person's numbers on a computed frame.
  const contextEntryKey =
    document &&
    contextFrame &&
    !document.computedFrames[contextFrame.id]?.editing.cells &&
    !document.computedFrames[contextFrame.id]?.paged &&
    contextFrame.uniqueKeys.length > 0
      ? contextFrame.uniqueKeys[0].columnIds
      : null;
  // Spreading needs a column to fill the cells from: an entry column when
  // there is one, else the rightmost number column.
  const contextCrosstabValues = (() => {
    if (!document || !contextFrame || !contextColumn) return null;
    if (document.computedFrames[contextFrame.id]?.paged) return null;
    if (contextFrame.display?.crosstab) return null;
    const entry = contextFrame.entryColumns?.find(
      (candidate) => candidate.columnId !== contextColumn.id
    );
    if (entry) return entry.columnId;
    const numeric = [...contextFrame.columns]
      .reverse()
      .find(
        (column) =>
          column.id !== contextColumn.id &&
          ["number", "integer", "currency", "percentage"].includes(column.dataType)
      );
    return numeric?.id ?? null;
  })();

  const openContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      const target = event.target as HTMLElement;
      const frameId = target.closest<HTMLElement>("[data-frame-id]")?.dataset.frameId;
      const columnId = target.closest<HTMLElement>("[data-column-id]")?.dataset.columnId;
      const rowId = target.closest<HTMLElement>("[data-row-id]")?.dataset.rowId;
      const rowIndexText = target.closest<HTMLElement>("[data-row-index]")?.dataset
        .rowIndex;
      const rowIndex = rowIndexText === undefined ? undefined : Number(rowIndexText);
      const objectId = target.closest<HTMLElement>("[data-object-id]")?.dataset.objectId;
      const viewId = target.closest<HTMLElement>("[data-view-id]")?.dataset.viewId;
      const viewport = canvasRef.current;
      const bounds = viewport?.getBoundingClientRect();
      const { x: canvasX, y: canvasY } = canvasPoint(
        { x: event.clientX, y: event.clientY },
        {
          left: bounds?.left ?? 0,
          top: bounds?.top ?? 0,
          scrollLeft: viewport?.scrollLeft ?? 0,
          scrollTop: viewport?.scrollTop ?? 0,
        },
        canvasZoomRef.current
      );
      setContextMenu({
        screenX: event.clientX,
        screenY: event.clientY,
        canvasX,
        canvasY,
        frameId,
        columnId,
        rowId,
        rowIndex: Number.isFinite(rowIndex) ? rowIndex : undefined,
        objectId,
        viewId,
      });
      if (objectId) setSelection({ objectId, viewId, columnId, rowId });
    },
    [canvasRef, canvasZoomRef, setContextMenu, setSelection]
  );

  const deleteFromContext = useCallback(
    (operation: Operation) => {
      setContextMenu(null);
      void run(operation).then((failure) => {
        if (!failure) setSelection(null);
      });
    },
    [run, setContextMenu, setSelection]
  );

  return {
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
  };
}
