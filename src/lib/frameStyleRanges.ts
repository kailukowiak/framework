import type { FrameCellStyle, FrameStyle } from "./types";

/**
 * A rectangle format, indexed for lookup.
 *
 * The engine stores a range as two id lists, which is the right shape to
 * persist and the wrong one to ask "is this cell inside?" of: resolution
 * runs once per rendered cell, so an `includes` over a few hundred row ids
 * per range per cell is quadratic in the thing people select most, a whole
 * screen of rows. Sets make the question constant-time.
 */
type IndexedRange = {
  rows: ReadonlySet<string>;
  columns: ReadonlySet<string>;
  style: FrameCellStyle;
};

// Keyed by the styles array itself. The document hands the same array back
// for as long as nothing about the frame's formatting changed, so the index
// is built once per edit rather than once per cell. A WeakMap lets an old
// array (and its Sets) go when the next edit replaces it.
const indexes = new WeakMap<readonly FrameStyle[], IndexedRange[]>();

function rangeIndex(styles: readonly FrameStyle[]): IndexedRange[] {
  let index = indexes.get(styles);
  if (!index) {
    index = [];
    for (const entry of styles) {
      if (entry.target.kind !== "range") continue;
      index.push({
        rows: new Set(entry.target.rowIds),
        columns: new Set(entry.target.columnIds),
        style: entry.style,
      });
    }
    indexes.set(styles, index);
  }
  return index;
}

/**
 * The range formats that reach one cell, in the order they were made, so
 * a later range wins property by property where two overlap.
 *
 * A range never reaches the row-number gutter or a column header: both
 * ask without the other coordinate, and a rectangle of cells is not a
 * statement about them.
 */
export function rangeStylesAt(
  styles: readonly FrameStyle[],
  rowId: string | undefined,
  columnId: string | undefined
): FrameCellStyle[] {
  if (!rowId || !columnId || styles.length === 0) return [];
  const matched: FrameCellStyle[] = [];
  for (const range of rangeIndex(styles))
    if (range.rows.has(rowId) && range.columns.has(columnId)) matched.push(range.style);
  return matched;
}

/**
 * Whether any direct format touches the rectangle, for enabling
 * "Clear formatting": a cell format inside it, or a range that overlaps it.
 * Row, column and frame formats are not the rectangle's to clear.
 */
export function rectangleHasFormatting(
  styles: readonly FrameStyle[],
  rowIds: readonly string[],
  columnIds: readonly string[]
): boolean {
  const rows = new Set(rowIds);
  const columns = new Set(columnIds);
  return styles.some(({ target }) => {
    if (target.kind === "cell") return rows.has(target.rowId) && columns.has(target.columnId);
    if (target.kind === "range")
      return (
        target.rowIds.some((id) => rows.has(id)) &&
        target.columnIds.some((id) => columns.has(id))
      );
    return false;
  });
}
