import { AUTOMATIC_COLUMN_WIDTH, clampColumnWidth, usesFixedColumns } from "./columnLayout";

/**
 * Reads how wide a column's header cell is laid out. `offsetWidth`, not
 * `getBoundingClientRect`: the canvas can be zoomed, and a bounding box is
 * scaled by it while a layout width — which is what the engine stores and a
 * `<col>` is given — is not. It is zero in jsdom, which is why the resize
 * hook takes it as a parameter.
 */
export type ColumnWidthReader = (table: HTMLTableElement, columnId: string) => number | null;

export const readHeaderWidth: ColumnWidthReader = (table, columnId) => {
  const header = Array.from(table.querySelectorAll<HTMLElement>("th.column-header")).find(
    (candidate) => candidate.dataset.columnId === columnId
  );
  return header?.offsetWidth || null;
};

/**
 * The widths every column has right now, for a table that is still sizing
 * itself, or an empty object when it already is not.
 *
 * Why a resize needs this: an automatic table asks its content how wide each
 * column is, and a fixed one cannot. The moment one width is stored the
 * table flips to fixed layout and every column without an entry falls back to
 * `AUTOMATIC_COLUMN_WIDTH`, so resizing one column used to make all the
 * others jump to 150. Taking the measurements first and writing them with the
 * resized column pins every column where it already was, in the same
 * operation, so the first resize changes one column and nothing else.
 *
 * A column that cannot be measured keeps the fallback it would have got anyway.
 */
export function automaticColumnBasis(
  frame: Parameters<typeof usesFixedColumns>[0],
  table: HTMLTableElement | null | undefined,
  read: ColumnWidthReader = readHeaderWidth
): Record<string, number> {
  if (!table || usesFixedColumns(frame)) return {};
  return Object.fromEntries(
    frame.columns.map((column) => [
      column.id,
      clampColumnWidth(read(table, column.id) ?? AUTOMATIC_COLUMN_WIDTH),
    ])
  );
}
