import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { isEditableGridColumn, type GridFocusMode } from "../FrameGrid";
import { dragFillCells } from "../lib/dragFill";
import type { OperationHandler } from "../lib/handlers";
import type { GridPosition, GridRange } from "../lib/gridNavigation";
import type { Column, ComputedFrame, FrameObject, Row } from "../lib/types";

/** The fill square belongs only to literal, document-owned data. A drag
 * commits the values in its preview as one undoable operation. */
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
function rowIndexAt(x: number, y: number, table: Element | null): number | null {
  const row = document
    .elementFromPoint(x, y)
    ?.closest<HTMLElement>("tr[data-row-index]");
  if (!row || row.closest("table") !== table) return null;
  const index = Number(row.dataset.rowIndex);
  return Number.isInteger(index) ? index : null;
}

export function useFillHandleDrag(
  frame: FrameObject,
  selectionRange: GridRange | null,
  focusPosition: GridPosition | null,
  gridFocus: { mode: GridFocusMode } | null,
  options: { computed: ComputedFrame; rows: Row[]; rowOffset?: number; onOperation: OperationHandler }
): FillHandleDrag {
  const [preview, setPreview] = useState<FillHandleDrag["preview"]>(null);
  const candidate = fillHandleTarget(
    frame,
    selectionRange,
    focusPosition,
    gridFocus?.mode === "edit"
  );

  const offset = options.rowOffset ?? 0;
  const target = candidate && isEditableGridColumn(options.computed, candidate.column)
    && frame.rows.length > 0 && (selectionRange?.top ?? candidate.rowIndex) >= offset
    && candidate.rowIndex < offset + options.rows.length ? candidate : null;
  const cleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => cleanup.current?.(), []);

  const beginFillDrag = (event: ReactPointerEvent) => {
    if (event.button !== 0 || !target) return;
    // The handle owns this press: the cell under it would otherwise move the
    // selection the handle is describing.
    event.preventDefault();
    event.stopPropagation();
    cleanup.current?.();
    const { column, rowIndex: top } = target;
    const first = selectionRange?.top ?? top;
    const table = event.currentTarget.closest("table");
    try {
      event.currentTarget.setPointerCapture?.(event.pointerId);
    } catch {
      // Capture is a nicety; the window listeners below do the real work.
    }
    let reached = top;
    const move = (moveEvent: PointerEvent) => {
      const row = rowIndexAt(moveEvent.clientX, moveEvent.clientY, table);
      // Crossing a row is the only event worth a render; a drag stays inside
      // one row for most of the pointer moves it emits.
      if (row === null || row === reached) return;
      reached = row;
      setPreview(row > top ? { columnId: column.id, top, bottom: row }
        : row < first ? { columnId: column.id, top: row - 1, bottom: first - 1 } : null);
    };
    const end = (endEvent: PointerEvent) => {
      cleanup.current?.();
      setPreview(null);
      if (endEvent.type === "pointercancel") return;
      const row = rowIndexAt(endEvent.clientX, endEvent.clientY, table);
      if (row === null || (row >= first && row <= top)) return;
      const cells = dragFillCells(column, options.rows, first - offset, top - offset, row - offset);
      if (cells.length) void options.onOperation({ type: "setCells", frameId: frame.id, cells });
    };
    const cancel = (key: KeyboardEvent) => {
      if (key.key !== "Escape") return;
      cleanup.current?.();
      setPreview(null);
    };
    cleanup.current = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      window.removeEventListener("keydown", cancel);
      cleanup.current = null;
    };
    window.addEventListener("keydown", cancel);
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
