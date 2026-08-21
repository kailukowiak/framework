import { describe, expect, it } from "vitest";
import { expandRangeForSpan } from "./gridSpan";
import type { GridBounds, GridRange } from "./gridNavigation";

const bounds: GridBounds = { rowCount: 40, columnCount: 7 };
/** One cell, at row 3 column 2 — the cell a header click anchors on. */
const cell: GridRange = { top: 3, bottom: 3, left: 2, right: 2 };

describe("expandRangeForSpan", () => {
  it("leaves an ordinary rectangle alone", () => {
    const range: GridRange = { top: 1, bottom: 5, left: 0, right: 3 };
    expect(expandRangeForSpan(range, null, bounds)).toEqual(range);
  });

  it("takes a column selection down every row, keeping the columns", () => {
    expect(expandRangeForSpan(cell, "column", bounds)).toEqual({
      top: 0,
      bottom: 39,
      left: 2,
      right: 2,
    });
  });

  it("takes a row selection across every column, keeping the rows", () => {
    expect(expandRangeForSpan(cell, "row", bounds)).toEqual({
      top: 3,
      bottom: 3,
      left: 0,
      right: 6,
    });
  });

  it("keeps a multi-column span's columns while covering every row", () => {
    const twoColumns: GridRange = { top: 3, bottom: 3, left: 2, right: 4 };
    expect(expandRangeForSpan(twoColumns, "column", bounds)).toEqual({
      top: 0,
      bottom: 39,
      left: 2,
      right: 4,
    });
  });

  // With fields as rows a frame row runs across the screen, so both spans
  // paint the opposite way round. This is the case that silently breaks if
  // the axis mapping is written out by hand at each call site.
  it("swaps both axes when fields are the rows", () => {
    const transposed: GridBounds = { rowCount: 7, columnCount: 40 };
    expect(expandRangeForSpan(cell, "column", transposed, true)).toEqual({
      top: 3,
      bottom: 3,
      left: 0,
      right: 39,
    });
    expect(expandRangeForSpan(cell, "row", transposed, true)).toEqual({
      top: 0,
      bottom: 6,
      left: 2,
      right: 2,
    });
  });

  it("stays in range on an empty grid rather than going negative", () => {
    const empty: GridBounds = { rowCount: 0, columnCount: 0 };
    const origin: GridRange = { top: 0, bottom: 0, left: 0, right: 0 };
    expect(expandRangeForSpan(origin, "column", empty)).toEqual(origin);
    expect(expandRangeForSpan(origin, "row", empty)).toEqual(origin);
  });
});
