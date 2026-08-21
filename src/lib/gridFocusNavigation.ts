import { gridCellAt, type GridContext, type GridFocus } from "../FrameGrid";
import type { GridPosition, GridRange } from "./gridNavigation";
import type { Selection } from "./types";

export type GridFocusDestination = {
  focus: GridFocus;
  selection: Selection;
};

export function movedGridFocus(
  context: GridContext,
  current: GridFocus,
  position: GridPosition,
  extend: boolean
): GridFocusDestination | null {
  const target = gridCellAt(context, position);
  if (!target) return null;
  return {
    focus: {
      ...current,
      rowId: target.row.id,
      columnId: target.column.id,
      mode: "navigate",
      editSeed: null,
      anchor: extend
        ? current.anchor ?? { rowId: current.rowId, columnId: current.columnId }
        : null,
      // Shift+Arrow off a whole-axis selection returns to a rectangle.
      span: null,
    },
    selection: {
      objectId: current.objectId,
      viewId: current.viewId,
      rowId: target.row.id,
      columnId: target.column.id,
    },
  };
}

export function rangedGridFocus(
  context: GridContext,
  current: GridFocus,
  range: GridRange,
  span: GridFocus["span"] = null
): GridFocusDestination | null {
  const anchor = gridCellAt(context, { row: range.top, col: range.left });
  const target = gridCellAt(context, { row: range.bottom, col: range.right });
  if (!anchor || !target) return null;
  return {
    focus: {
      ...current,
      rowId: target.row.id,
      columnId: target.column.id,
      mode: "navigate",
      editSeed: null,
      anchor: { rowId: anchor.row.id, columnId: anchor.column.id },
      span,
    },
    selection: {
      objectId: current.objectId,
      viewId: current.viewId,
      rowId: target.row.id,
      columnId: target.column.id,
    },
  };
}
