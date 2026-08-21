// Whole-axis selections: clicking a column header, or a row's index.
//
// These cannot be expressed as an anchor and a focus cell the way an
// ordinary drag can. An imported frame holds only the scrolled-to window of
// rows on the client, so "every row of this column" has no far corner to
// point at — the far corner is a row that has not been read yet. A span is
// therefore carried on the focus as an intent, and resolved separately
// against whatever the caller can see: the rendered window when painting
// the highlight, the frame's real row count when copying.
//
// The mapping is the part worth isolating. A span is stated in *frame*
// terms — all rows, or all columns — while a range is in *screen* terms,
// and the two orientations disagree about which screen axis a frame row
// runs along. Getting that backwards is invisible in the common
// orientation and wrong in the other, so it lives here with tests rather
// than being written out at each of the three call sites.

import type { GridBounds, GridRange } from "./gridNavigation";

/**
 * `"column"` means the anchored columns across every row; `"row"` means the
 * anchored rows across every column. `null` is an ordinary rectangle.
 */
export type GridSpan = "row" | "column" | null;

/**
 * Widens `range` to cover the whole of whichever axis `span` names.
 *
 * `transposed` is the fields-as-rows orientation, where a frame row runs
 * across the screen rather than down it — so each span paints the opposite
 * way round from the records-as-rows case.
 */
export function expandRangeForSpan(
  range: GridRange,
  span: GridSpan,
  bounds: GridBounds,
  transposed = false
): GridRange {
  if (!span) return range;
  const frameRowsRunVertically = !transposed;
  const spanCoversFrameRows = span === "column";
  return spanCoversFrameRows === frameRowsRunVertically
    ? { ...range, top: 0, bottom: Math.max(0, bounds.rowCount - 1) }
    : { ...range, left: 0, right: Math.max(0, bounds.columnCount - 1) };
}
