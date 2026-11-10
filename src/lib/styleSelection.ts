import type { GridContext, GridFocus } from "../FrameGrid";
import { gridRangeForFocus } from "../FrameGrid";
import type { FrameCellStyle, FrameStyle, Operation } from "./types";

/**
 * What a format gesture should address when the grid selection is more than
 * the active cell, or null when the existing single-target rule (cell, row,
 * column, frame) already says it.
 *
 * - `range`: a dragged rectangle of cells. One `range` target covering the
 *   selected rows by the selected columns.
 * - `columns` / `rows`: several whole columns or rows. These keep their
 *   whole-axis targets, one per column or row, because a column format must
 *   reach rows that are not loaded and must follow rows added later.
 *
 * Rows come from the loaded window, which is the only place a rectangle can
 * exist on a paged frame: the range is resolved against `displayedRows`, so a
 * drag cannot name an unloaded row. Whole-column selections do not have that
 * limit, since they never enumerate rows.
 */
export type StyleSelection =
  | { kind: "range"; rowIds: string[]; columnIds: string[] }
  | { kind: "columns"; columnIds: string[] }
  | { kind: "rows"; rowIds: string[] };

export function selectedStyleSelection(
  context: GridContext | null,
  focus: GridFocus | null
): StyleSelection | null {
  if (!context || !focus) return null;
  // With fields as rows, "row" is a column of the frame and the rectangle is
  // transposed; formatting there keeps the active cell's target.
  if (context.orientation === "fieldsAsRows") return null;
  const range = gridRangeForFocus(context, focus);
  if (!range) return null;
  const columnIds = context.frame.columns
    .slice(range.left, range.right + 1)
    .map((column) => column.id);
  const rowIds = context.displayedRows
    .slice(range.top, range.bottom + 1)
    .map((row) => row.id);
  if (focus.span === "column")
    return columnIds.length > 1 ? { kind: "columns", columnIds } : null;
  if (focus.span === "row") return rowIds.length > 1 ? { kind: "rows", rowIds } : null;
  if (rowIds.length * columnIds.length <= 1) return null;
  return { kind: "range", rowIds, columnIds };
}

/** A patch that carries at least one property. */
export function patchHasValue(patch: Partial<FrameCellStyle>): boolean {
  return Object.values(patch).some((value) => value !== null && value !== undefined);
}

/**
 * The operation for one format gesture over a multi-cell selection.
 *
 * A range is a patch: it sets only what it carries, and clears those
 * properties from the cell formats beneath it. So a bold button sends
 * `{ bold }` alone and leaves a fill someone chose cell by cell untouched.
 * A patch of nothing but nulls is not sent — an empty range style means
 * "clear formats", and a colour picker's "no colour" must not silently wipe
 * everything else; "Clear formatting" is the gesture for that.
 *
 * `patch` null is that explicit clear.
 */
export function styleSelectionOperation(
  frameId: string,
  selection: StyleSelection,
  patch: Partial<FrameCellStyle> | null,
  exactStyleOf: (target: FrameStyle["target"]) => FrameCellStyle,
  empty: FrameCellStyle
): Operation | null {
  if (selection.kind === "range") {
    if (patch && !patchHasValue(patch)) return null;
    const style = patch ? { ...empty, ...stripNulls(patch) } : empty;
    return {
      type: "setFrameStyle",
      frameId,
      target: { kind: "range", rowIds: selection.rowIds, columnIds: selection.columnIds },
      style,
    };
  }
  const targets: FrameStyle["target"][] =
    selection.kind === "columns"
      ? selection.columnIds.map((columnId) => ({ kind: "column", columnId }))
      : selection.rowIds.map((rowId) => ({ kind: "row", rowId }));
  const operations: Operation[] = targets.map((target) => ({
    type: "setFrameStyle",
    frameId,
    target,
    style: patch ? { ...exactStyleOf(target), ...patch } : empty,
  }));
  return { type: "batch", operations };
}

function stripNulls(patch: Partial<FrameCellStyle>): Partial<FrameCellStyle> {
  return Object.fromEntries(
    Object.entries(patch).filter(([, value]) => value !== null && value !== undefined)
  );
}
