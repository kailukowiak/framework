import {
  useCallback,
  useEffect,
  useRef,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import {
  ARROW_DIRECTIONS,
  gridCellAt,
  resolveGridContext,
  type GridFocus,
  type RenderedGrid,
} from "../FrameGrid";
import { focusGridClipboardTarget } from "../lib/gridClipboardTarget";
import type { GridDirection } from "../lib/gridNavigation";
import type { OperationHandler } from "../lib/handlers";
import type { DocumentView, FrameObject, Selection } from "../lib/types";

/**
 * Keys for a card that is selected on the canvas but not entered: Return goes
 * in, Delete takes the card away.
 *
 * Return lands where the person last was in that card, so Escape-then-Return
 * is a round trip rather than a reset to the corner; a card never entered, or
 * whose remembered cell has since gone, opens on its first cell. Delete is the
 * ordinary `deleteObject`, one undo step. Both ignore a selection that names
 * a row or a column, since that is a cell selection and the grid's keys own
 * it, and the caller has already ruled out text fields.
 *
 * Arrow keys and Tab move the selection between cards (`navigateCanvas`);
 * they are taken whenever that gesture was meant, whether or not there was a
 * neighbour to land on, so the browser's own scrolling and focus walk never
 * race the canvas's.
 *
 * Returns whether the key was taken.
 */
export function useCardSelectionKeys({
  document,
  selection,
  gridFocus,
  renderedRows,
  setGridFocus,
  setSelection,
  run,
  navigateCanvas,
}: {
  document: DocumentView | null;
  selection: Selection | null;
  gridFocus: GridFocus | null;
  renderedRows: MutableRefObject<Map<string, RenderedGrid>>;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  run: OperationHandler;
  navigateCanvas: (direction: GridDirection | "next" | "previous") => boolean;
}) {
  const lastCell = useRef(new Map<string, { rowId: string; columnId: string }>());
  useEffect(() => {
    if (gridFocus)
      lastCell.current.set(gridFocus.objectId, {
        rowId: gridFocus.rowId,
        columnId: gridFocus.columnId,
      });
  }, [gridFocus]);

  const handleCardKey = useCallback(
    (event: KeyboardEvent): boolean => {
      if (!document || !selection || gridFocus) return false;
      if (selection.rowId !== undefined || selection.columnId !== undefined)
        return false;
      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey)
        return false;
      const object = document.objects.find(
        (candidate) => candidate.id === selection.objectId
      );
      if (!object) return false;

      if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        void run({ type: "deleteObject", objectId: object.id }).then((failure) => {
          if (!failure) setSelection(null);
        });
        return true;
      }
      if (event.key !== "Enter" || object.kind !== "frame") return false;
      return enterCard(event, object, {
        document,
        selection,
        renderedRows,
        lastCell: lastCell.current,
        setGridFocus,
        setSelection,
      });
    },
    [document, gridFocus, renderedRows, run, selection, setGridFocus, setSelection]
  );

  return useCallback(
    (event: KeyboardEvent): boolean => {
      if (handleCardKey(event)) return true;
      const direction = ARROW_DIRECTIONS[event.key];
      const cycle = event.key === "Tab" ? (event.shiftKey ? "previous" : "next") : null;
      const modifier = event.metaKey || event.ctrlKey;
      if (!((direction && !modifier && !event.shiftKey) || cycle)) return false;
      if (navigateCanvas(direction ?? cycle!)) event.preventDefault();
      return true;
    },
    [handleCardKey, navigateCanvas]
  );
}

function enterCard(
  event: KeyboardEvent,
  object: FrameObject,
  {
    document,
    selection,
    renderedRows,
    lastCell,
    setGridFocus,
    setSelection,
  }: {
    document: DocumentView;
    selection: Selection;
    renderedRows: MutableRefObject<Map<string, RenderedGrid>>;
    lastCell: Map<string, { rowId: string; columnId: string }>;
    setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
    setSelection: Dispatch<SetStateAction<Selection | null>>;
  }
): boolean {
  const view =
    document.views.find((candidate) => candidate.id === selection.viewId) ??
    document.views.find(
      (candidate) =>
        candidate.objectId === object.id || candidate.tabObjectIds?.includes(object.id)
    );
  if (!view) return false;
  const probe: GridFocus = {
    viewId: view.id,
    objectId: object.id,
    rowId: "",
    columnId: "",
    mode: "navigate",
    editSeed: null,
    anchor: null,
    span: null,
  };
  const context = resolveGridContext(document, probe, renderedRows.current);
  if (!context) return false;
  const remembered = lastCell.get(object.id);
  const target =
    remembered &&
    context.displayedRows.some((row) => row.id === remembered.rowId) &&
    context.frame.columns.some((column) => column.id === remembered.columnId)
      ? remembered
      : (() => {
          const first = gridCellAt(context, { row: 0, col: 0 });
          return first ? { rowId: first.row.id, columnId: first.column.id } : null;
        })();
  if (!target) return false;
  event.preventDefault();
  focusGridClipboardTarget();
  setGridFocus({ ...probe, ...target });
  setSelection({ objectId: object.id, viewId: view.id, ...target });
  return true;
}
