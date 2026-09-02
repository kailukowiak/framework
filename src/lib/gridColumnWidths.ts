/**
 * Where each column of a rendered grid table starts and ends, in layout
 * pixels: the row-number gutter, one entry per column, then the trailing
 * edge. Taken from where each header cell begins rather than how wide it is,
 * so border spacing is included, and divided by the canvas scale so a zoomed
 * card measures the same as an unzoomed one. Null while the table has no
 * layout yet, or when its header does not have the expected cells — a
 * crosstab, say, standing in for the records table.
 */
export function measureGridColumnWidths(
  table: HTMLTableElement,
  expectedCells: number
): number[] | null {
  const cells = Array.from(
    table.querySelectorAll<HTMLElement>("thead tr:first-child > th")
  );
  if (cells.length !== expectedCells || !table.offsetWidth) return null;
  const rect = table.getBoundingClientRect();
  const scale = rect.width / table.offsetWidth;
  if (!(scale > 0)) return null;
  const lefts = cells.map((cell) => cell.getBoundingClientRect().left);
  return lefts.map((left, index) => ((lefts[index + 1] ?? rect.right) - left) / scale);
}
