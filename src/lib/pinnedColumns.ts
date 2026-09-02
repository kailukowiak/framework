import type { CSSProperties } from "react";

/**
 * Where the frozen cells of a records grid sit, in layout pixels from the
 * left edge of the table.
 *
 * Index 0 is the row-number gutter, which is frozen alongside the columns —
 * a frozen key column that scrolls away from its own row number is worse
 * than no freezing at all. Index `n + 1` is data column `n`. The array is
 * empty when nothing is frozen, or while the grid has not been measured
 * yet: half a freeze, drawn at the wrong offsets, would sit on top of the
 * columns it is meant to be beside.
 *
 * The stored count is clamped here rather than corrected in the document.
 * Dropping or reordering columns can leave a frame asking for more frozen
 * columns than it now has, and the honest reading of that is "freeze what
 * is there" — putting the columns back puts the whole freeze back.
 */
export function pinnedColumnOffsets(
  columnWidths: number[] | null,
  pinnedColumns: number,
  columnCount: number
): number[] {
  const pinned = Math.min(Math.max(Math.trunc(pinnedColumns) || 0, 0), columnCount);
  if (pinned === 0 || !columnWidths || columnWidths.length < pinned + 1) return [];
  const offsets = [0];
  let left = 0;
  for (let index = 0; index < pinned; index += 1) {
    left += columnWidths[index];
    offsets.push(left);
  }
  return offsets;
}

/**
 * The sticky offset for one cell, as a custom property the stylesheet reads,
 * or nothing at all for a cell that scrolls normally. Returned as a style
 * fragment so callers spread it over the cell's own styling rather than
 * choosing between the two.
 */
export function pinnedCellStyle(index: number, offsets: number[]): CSSProperties {
  const left = offsets[index];
  return left === undefined
    ? {}
    : ({ "--pinned-left": `${left}px` } as CSSProperties);
}

/**
 * The last frozen cell carries a divider, which is the only thing that says
 * where the frozen region ends once the grid has scrolled under it.
 */
export function pinnedCellClass(index: number, offsets: number[]): string {
  if (offsets[index] === undefined) return "";
  return index === offsets.length - 1
    ? "pinned-column pinned-column-last"
    : "pinned-column";
}

/**
 * What "pin through this column" means for one column: the count that freezes
 * everything up to and including it, or `0` when it is already frozen — the
 * same menu entry unfreezes, because a second press on a frozen column is
 * asking to undo the first.
 */
export function pinnedThrough(
  frame: {
    columns: Array<{ id: string }>;
    display?: { pinnedColumns?: number } | null;
  },
  column: { id: string }
): number {
  const index = frame.columns.findIndex((candidate) => candidate.id === column.id);
  if (index < 0) return 0;
  const through = index + 1;
  return (frame.display?.pinnedColumns ?? 0) >= through ? 0 : through;
}
