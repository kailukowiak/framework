import { useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import type { GridFocusMode } from "../FrameGrid";
import type { GridPosition, GridRange } from "../lib/gridNavigation";
import type { Column, FrameObject } from "../lib/types";

/**
 * Excel's fill handle, reinterpreted. There it drags a value down a range;
 * here the unit is the column, and a column already has a declaration slot,
 * so the drag opens that slot instead of copying cells — the same door `=`
 * in a cell and a double-click on a calculated column already open.
 *
 * ⌘D still fills literally. This gesture never writes a value of its own:
 * it hands the column to Wrangle and leaves the selection where it was.
 */
export type FillHandleTarget = {
  column: Column;
  /** Visual row index of the range's bottom row — the cell the handle sits in. */
  rowIndex: number;
};

export type FillHandleDrag = {
  /** Where the handle belongs, or null when the selection cannot carry one. */
  target: FillHandleTarget | null;
  /** How far the drag reaches right now, or null when it is asking for nothing. */
  preview: { columnId: string; top: number; bottom: number } | null;
  /** What a body cell takes from the handle: the anchor cell, or the preview. */
  cellClass: (columnId: string, rowIndex: number) => string;
  beginFillDrag: (event: ReactPointerEvent) => void;
};

/**
 * One column, any number of rows, and nothing being typed. A selection
 * across several columns has no single declaration to open, so it carries
 * no handle rather than picking a column for you. One cell paints no range
 * at all, and still names a column, so the focused cell stands in for one.
 */
function fillHandleTarget(
  frame: FrameObject,
  selectionRange: GridRange | null,
  focusPosition: GridPosition | null,
  editing: boolean
): FillHandleTarget | null {
  if (editing) return null;
  const range =
    selectionRange ??
    (focusPosition && {
      top: focusPosition.row,
      bottom: focusPosition.row,
      left: focusPosition.col,
      right: focusPosition.col,
    });
  if (!range || range.left !== range.right) return null;
  const column = frame.columns[range.left];
  return column ? { column, rowIndex: range.bottom } : null;
}

/** The row a screen point is over, read off the grid's own row markers. */
function rowIndexAt(x: number, y: number): number | null {
  const row = document
    .elementFromPoint(x, y)
    ?.closest<HTMLElement>("tr[data-row-index]");
  if (!row) return null;
  const index = Number(row.dataset.rowIndex);
  return Number.isInteger(index) ? index : null;
}

export function useFillHandleDrag(
  frame: FrameObject,
  selectionRange: GridRange | null,
  focusPosition: GridPosition | null,
  gridFocus: { mode: GridFocusMode } | null,
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void
): FillHandleDrag {
  const [preview, setPreview] = useState<FillHandleDrag["preview"]>(null);
  const target = fillHandleTarget(
    frame,
    selectionRange,
    focusPosition,
    gridFocus?.mode === "edit"
  );

  const beginFillDrag = (event: ReactPointerEvent) => {
    if (event.button !== 0 || !target) return;
    // The handle owns this press: the cell under it would otherwise move the
    // selection the handle is describing.
    event.preventDefault();
    event.stopPropagation();
    const { column, rowIndex: top } = target;
    try {
      event.currentTarget.setPointerCapture?.(event.pointerId);
    } catch {
      // Capture is a nicety; the window listeners below do the real work.
    }
    let reached = top;
    const move = (moveEvent: PointerEvent) => {
      const row = rowIndexAt(moveEvent.clientX, moveEvent.clientY);
      // Crossing a row is the only event worth a render; a drag stays inside
      // one row for most of the pointer moves it emits.
      if (row === null || row === reached) return;
      reached = row;
      setPreview(row > top ? { columnId: column.id, top, bottom: row } : null);
    };
    const end = (endEvent: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      setPreview(null);
      if (endEvent.type === "pointercancel") return;
      const row = rowIndexAt(endEvent.clientX, endEvent.clientY) ?? reached;
      // Down at least one row is the whole gesture. Up, sideways, or a
      // release on the row it started from asked for nothing.
      if (row > top) onTransformColumn(frame, column, "");
    };
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };

  const cellClass = (columnId: string, rowIndex: number) => {
    if (target?.column.id === columnId && target.rowIndex === rowIndex)
      return "fill-anchor";
    if (!preview || preview.columnId !== columnId) return "";
    return rowIndex > preview.top && rowIndex <= preview.bottom
      ? "fill-preview"
      : "";
  };

  return { target, preview, cellClass, beginFillDrag };
}
