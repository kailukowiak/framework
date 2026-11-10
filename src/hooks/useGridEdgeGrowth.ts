import {
  useCallback,
  useEffect,
  useRef,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from "react";
import {
  gridBoundsFor,
  gridCellAt,
  resolveGridContext,
  visualGridPosition,
  type GridFocus,
  type RenderedGrid,
} from "../FrameGrid";
import {
  edgeGrowthFor,
  focusGhostColumn,
  focusGhostRow,
  rowsAreFileBacked,
  type EdgeKey,
} from "../lib/gridEdgeGrowth";
import { focusGridClipboardTarget } from "../lib/gridClipboardTarget";
import { appendRowOperation } from "../lib/gridStructure";
import { enterPosition, tabPosition, type GridDirection } from "../lib/gridNavigation";
import type { OperationHandler } from "../lib/handlers";
import type { DocumentView, Selection } from "../lib/types";

/** Fired by anything that finishes a cell edit somewhere the grid cannot see. */
export const GRID_RETURN_EVENT = "framework:grid-return";

/**
 * Tells the grid that a value was just committed to the focused cell from
 * outside it — the formula bar folding `=1200/12` into a plain number — so
 * focus comes back to the grid and moves down, as any committed cell edit
 * does. An event rather than a prop because the committing editor lives in
 * the inspector and the focused grid in the canvas, and neither owns the
 * other.
 */
export function announceGridReturn() {
  window.dispatchEvent(new Event(GRID_RETURN_EVENT));
}

/**
 * Keeps the grid continuous at its edges: Return on the last row, Tab on the
 * last cell, and Right-arrow off the last column all carry on into the
 * space where data is made (see `edgeGrowthFor`), and the step that follows
 * a committed edit takes the same road as the keystroke would.
 *
 * `stepGridFocus` is what App hands each card as `onGridStep`; the navigate
 * handler uses `growAtEdge` before it clamps. They share one decision so the
 * two ways of leaving a cell — typing then Return, or just pressing Return —
 * cannot disagree about where the bottom of the grid is.
 */
export function useGridEdgeGrowth({
  document,
  gridFocus,
  renderedRows,
  setGridFocus,
  setSelection,
  run,
}: {
  document: DocumentView | null;
  gridFocus: GridFocus | null;
  renderedRows: MutableRefObject<Map<string, RenderedGrid>>;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  run: OperationHandler;
}) {
  // A blank row was asked for with Tab. Its id is not known until the engine
  // answers, so the request waits for a different last row to appear.
  const landing = useRef<{ fromRowId: string; lastRowId: string | null } | null>(null);

  useEffect(() => {
    const pending = landing.current;
    if (!pending || !document || !gridFocus) return;
    // The cursor moved on (or somewhere else entirely) while the row was
    // being made; it is not ours to take back.
    if (gridFocus.rowId !== pending.fromRowId) {
      landing.current = null;
      return;
    }
    const context = resolveGridContext(document, gridFocus, renderedRows.current);
    // A frame that owns its rows has the new one in the document itself;
    // reading it there does not wait on the card's next render to publish
    // its rows. Only rows read out of a file arrive in pages and have to be
    // asked for.
    const last = context
      ? rowsAreFileBacked(context.frame)
        ? context.displayedRows.at(-1)
        : context.frame.rows.at(-1)
      : undefined;
    const first = context?.frame.columns[0];
    if (!context || !last || !first || last.id === pending.lastRowId) return;
    landing.current = null;
    setGridFocus({
      ...gridFocus,
      rowId: last.id,
      columnId: first.id,
      mode: "navigate",
      editSeed: null,
      editRefused: false,
      anchor: null,
      span: null,
    });
    setSelection({
      objectId: gridFocus.objectId,
      viewId: gridFocus.viewId,
      rowId: last.id,
      columnId: first.id,
    });
  }, [document, gridFocus, renderedRows, setGridFocus, setSelection]);

  const growAtEdge = useCallback(
    (key: EdgeKey): boolean => {
      if (!document || !gridFocus) return false;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const position = context
        ? visualGridPosition(context, gridFocus.rowId, gridFocus.columnId)
        : null;
      if (!context || !position) return false;
      const growth = edgeGrowthFor(context, position, key);
      if (!growth) return false;
      const settled: GridFocus = {
        ...gridFocus,
        mode: "navigate",
        editSeed: null,
        editRefused: false,
        anchor: null,
        span: null,
      };
      if (growth === "ghost-row") {
        if (!focusGhostRow(context.frame.id, gridFocus.viewId, gridFocus.columnId))
          return false;
        setGridFocus(settled);
        return true;
      }
      if (growth === "ghost-column") {
        if (!focusGhostColumn(context.frame.id, gridFocus.viewId, gridFocus.rowId))
          return false;
        setGridFocus(settled);
        return true;
      }
      landing.current = {
        fromRowId: gridFocus.rowId,
        lastRowId: context.displayedRows.at(-1)?.id ?? null,
      };
      setGridFocus(settled);
      void run(appendRowOperation(context.frame.id)).then(
        (failure) => {
          if (failure) landing.current = null;
        }
      );
      return true;
    },
    [document, gridFocus, renderedRows, run, setGridFocus]
  );

  // One visual step from the active cell after an editor commit (Enter/Tab family).
  const stepGridFocus = useCallback(
    (direction: GridDirection) => {
      if (!document || !gridFocus) return;
      if (direction === "down" && growAtEdge("enter")) return;
      if (direction === "right" && growAtEdge("tab")) return;
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
    [document, gridFocus, growAtEdge, renderedRows, setGridFocus, setSelection]
  );

  return { growAtEdge, stepGridFocus };
}

/**
 * Hands the keyboard back to the grid when a value was committed to the
 * focused cell from outside it (the formula bar folding `=1200/12` into a
 * number): focus returns and steps down, as any committed cell edit does.
 * Separate from `useGridEdgeGrowth` because that hook is also used by the
 * key handler, and two listeners for one announcement would step twice.
 */
export function useGridReturnListener(stepGridFocus: (direction: GridDirection) => void) {
  useEffect(() => {
    const onReturn = () => {
      focusGridClipboardTarget();
      stepGridFocus("down");
    };
    window.addEventListener(GRID_RETURN_EVENT, onReturn);
    return () => window.removeEventListener(GRID_RETURN_EVENT, onReturn);
  }, [stepGridFocus]);
}
