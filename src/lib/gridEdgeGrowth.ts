import type { GridContext } from "../FrameGrid";
import type { FrameObject } from "./types";
import type { GridPosition } from "./gridNavigation";

/**
 * What a keystroke that would run off the edge of the grid does instead.
 *
 * A spreadsheet has no edge: Return at the bottom of a column and Tab at the
 * end of a row both carry on into empty space that becomes data the moment
 * it is typed into. FrameWork's grid is a bounded table with a "ghost" row
 * and a "ghost" column hung off it, so these are the places the same
 * continuity has to be built by hand.
 *
 * - `ghost-row`: Return on the last row lands in the new-row line, in the
 *   same column, so typing carries straight on. No empty row is written
 *   until something is typed.
 * - `append-row`: Tab on the very last cell is the Excel table gesture and
 *   does write: a blank row arrives and the cursor lands on its first cell.
 * - `ghost-column`: Right-arrow off the last column steps onto the column
 *   that typing grows.
 */
export type EdgeGrowth = "ghost-row" | "append-row" | "ghost-column";
export type EdgeKey = "enter" | "tab" | "arrowRight";

/**
 * Which growth (if any) a key at this position asks for.
 *
 * Only the records-as-rows orientation grows: a fields-as-rows card draws a
 * column as a row, so "the last row" there is a different edge and would
 * add a column where the person expects a record. A paged frame whose
 * window stops short of the final row has not reached its edge either — the
 * key moves within the loaded window as it always did.
 */
export function edgeGrowthFor(
  context: GridContext,
  position: GridPosition,
  key: EdgeKey
): EdgeGrowth | null {
  if (context.orientation === "fieldsAsRows") return null;
  const lastColumn = context.frame.columns.length - 1;
  const atWindowEnd = position.row >= context.displayedRows.length - 1;
  const atFrameEnd =
    atWindowEnd &&
    context.rowOffset + context.displayedRows.length >= context.totalRows;
  const rows = Boolean(context.computed?.editing.rows);
  if (key === "enter") return atFrameEnd && rows ? "ghost-row" : null;
  if (key === "tab")
    return atFrameEnd && rows && position.col >= lastColumn ? "append-row" : null;
  return position.col >= lastColumn && context.computed?.editing.columns
    ? "ghost-column"
    : null;
}

/**
 * The input a ghost line exposes for one frame, found by the data attributes
 * the card puts on it. Looked up in the document rather than held in a ref
 * because the ghost row is drawn by a component that has no reason to know a
 * keyboard handler exists; the attribute is the whole contract between them.
 * With two views of one frame open the one under `viewId` wins.
 */
function ghostInput(
  kind: "row" | "column",
  frameId: string,
  viewId: string,
  match: (element: HTMLElement) => boolean
): HTMLElement | null {
  const candidates = [
    ...window.document.querySelectorAll<HTMLElement>(`[data-ghost="${kind}"]`),
  ].filter((element) => element.dataset.frameId === frameId && match(element));
  return (
    candidates.find((element) => element.closest(`[data-view-id="${viewId}"]`)) ??
    candidates[0] ??
    null
  );
}

/** Focuses the new-row line at `columnId`, or its first input if that column has none. */
export function focusGhostRow(
  frameId: string,
  viewId: string,
  columnId: string
): boolean {
  const same = ghostInput(
    "row",
    frameId,
    viewId,
    (el) => el.dataset.ghostColumnId === columnId
  );
  const target = same ?? ghostInput("row", frameId, viewId, () => true);
  if (!target) return false;
  target.focus();
  return window.document.activeElement === target;
}

/** Focuses the column-growing cell on `rowId`. */
export function focusGhostColumn(
  frameId: string,
  viewId: string,
  rowId: string
): boolean {
  const target = ghostInput(
    "column",
    frameId,
    viewId,
    (el) => el.dataset.rowId === rowId
  );
  if (!target) return false;
  target.focus();
  return window.document.activeElement === target;
}

/**
 * Whether the frame's rows live in a file, an artifact or a connector, with
 * the document holding only patches over them. Those can take an appended
 * row but not one inserted at a position, so the insert chord appends there.
 * Asked of the frame's own source fields rather than of `computed.paged`,
 * because a hand-entered frame with a row-preserving Wrangle chain is paged
 * too and still owns every row.
 */
export function rowsAreFileBacked(frame: FrameObject): boolean {
  return Boolean(frame.sourceFile || frame.artifact || frame.connector);
}

/**
 * The structure chords: insert and delete a row or column from the keyboard.
 *
 * Two natural spellings were rejected. ⌘− and ⌘= are canvas zoom here (the
 * View menu binds them, and in a webview they would be browser zoom if the
 * menu did not), so Excel's ⌃− / ⌃⇧= cannot be used without fighting a
 * shortcut that already has a meaning. Adding ⌥ clears both: ⌘⌥= inserts and
 * ⌘⌥− deletes, which is also Google Sheets' spelling, so it is a chord some
 * people already own. The menu's only ⌘⌥ accelerators are letters, and the
 * application shortcut map answers ⌥ with its insert-object letters alone.
 * `code` is matched, not `key`: ⌥ turns `=` into `≠` and `-` into an
 * en dash on a Mac keyboard.
 */
export function structureChord(event: KeyboardEvent): "insert" | "delete" | null {
  if (!(event.metaKey || event.ctrlKey) || !event.altKey) return null;
  if (event.code === "Equal" || event.code === "NumpadAdd") return "insert";
  if (event.code === "Minus" || event.code === "NumpadSubtract") return "delete";
  return null;
}
