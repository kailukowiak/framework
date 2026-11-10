/**
 * How wide a column wants to be, read from the table the browser has laid
 * out. Fitting is a measurement, not a calculation: font, number format,
 * accounting symbols and the header's sort and filter buttons all decide the
 * width, and the engine knows none of them.
 *
 * It fits what is rendered. A virtualised or paged frame has only a window of
 * rows in the DOM, so a column is fitted to the rows currently loaded plus
 * its header, the way a spreadsheet fits to what it has drawn; a longer value
 * further down is clipped (with an ellipsis) until the person fits again.
 *
 * `scrollWidth` is a layout width, unaffected by the canvas zoom, so unlike
 * `measureGridColumnWidths` this needs no scale correction. It is zero in
 * jsdom, which is why the hook takes its measurer as a parameter.
 */
export type ColumnFitMeasurer = (
  table: HTMLTableElement,
  columnId: string
) => number | null;

/** Padding on each side of a cell's text, plus the cell's own border. */
const CELL_CHROME = 21;

export const measureColumnFit: ColumnFitMeasurer = (table, columnId) => {
  const selector = `[data-column-id="${CSS.escape(columnId)}"]`;
  let widest = 0;
  const header = table.querySelector<HTMLElement>(`th.column-header${selector}`);
  // The header row is a flex line of sort, filter and the name; its natural
  // width is the sum of its items, each measured unclipped.
  if (header?.firstElementChild)
    widest = Array.from(header.firstElementChild.children).reduce(
      (sum, item) => sum + (item as HTMLElement).scrollWidth,
      0
    );
  for (const cell of table.querySelectorAll<HTMLElement>(`tbody td${selector}`)) {
    const content = cell.firstElementChild as HTMLElement | null;
    widest = Math.max(widest, (content?.scrollWidth ?? 0) + CELL_CHROME);
  }
  return widest > 0 ? widest : null;
};
