import { describe, expect, it } from "vitest";
import {
  clampPosition,
  contiguousDataRange,
  documentEdgePosition,
  enterPosition,
  fillPairs,
  fullColumnRange,
  fullGridRange,
  fullRowRange,
  isPrintableKey,
  jumpPosition,
  normalizeRange,
  pagePosition,
  positionInRange,
  rowEdgePosition,
  sameRange,
  scrollLeftToRevealColumn,
  scrollTopToRevealRow,
  stepPosition,
  tabPosition,
  type GridBounds,
  type GridPosition,
} from "./gridNavigation";
import {
  MAX_VIRTUAL_SCROLL_HEIGHT,
  TABLE_HEADER_HEIGHT,
  TABLE_ROW_HEIGHT,
} from "./frameVirtualization";

const bounds: GridBounds = { rowCount: 5, columnCount: 4 };

describe("clampPosition", () => {
  it("keeps in-bounds positions unchanged", () => {
    expect(clampPosition({ row: 2, col: 3 }, bounds)).toEqual({ row: 2, col: 3 });
  });

  it("clamps negative and overflowing coordinates", () => {
    expect(clampPosition({ row: -3, col: 99 }, bounds)).toEqual({ row: 0, col: 3 });
    expect(clampPosition({ row: 99, col: -1 }, bounds)).toEqual({ row: 4, col: 0 });
  });

  it("collapses to the origin for an empty grid", () => {
    expect(clampPosition({ row: 3, col: 3 }, { rowCount: 0, columnCount: 0 })).toEqual({
      row: 0,
      col: 0,
    });
  });
});

describe("stepPosition", () => {
  it("moves one cell in each direction", () => {
    expect(stepPosition({ row: 2, col: 2 }, "up", bounds)).toEqual({ row: 1, col: 2 });
    expect(stepPosition({ row: 2, col: 2 }, "down", bounds)).toEqual({
      row: 3,
      col: 2,
    });
    expect(stepPosition({ row: 2, col: 2 }, "left", bounds)).toEqual({
      row: 2,
      col: 1,
    });
    expect(stepPosition({ row: 2, col: 2 }, "right", bounds)).toEqual({
      row: 2,
      col: 3,
    });
  });

  it("clamps at every edge instead of wrapping", () => {
    expect(stepPosition({ row: 0, col: 0 }, "up", bounds)).toEqual({ row: 0, col: 0 });
    expect(stepPosition({ row: 0, col: 0 }, "left", bounds)).toEqual({
      row: 0,
      col: 0,
    });
    expect(stepPosition({ row: 4, col: 3 }, "down", bounds)).toEqual({
      row: 4,
      col: 3,
    });
    expect(stepPosition({ row: 4, col: 3 }, "right", bounds)).toEqual({
      row: 4,
      col: 3,
    });
  });
});

describe("tabPosition and enterPosition", () => {
  it("tab moves right and shift+tab moves left", () => {
    expect(tabPosition({ row: 1, col: 1 }, false, bounds)).toEqual({ row: 1, col: 2 });
    expect(tabPosition({ row: 1, col: 1 }, true, bounds)).toEqual({ row: 1, col: 0 });
  });

  it("enter moves down and shift+enter moves up", () => {
    expect(enterPosition({ row: 1, col: 1 }, false, bounds)).toEqual({
      row: 2,
      col: 1,
    });
    expect(enterPosition({ row: 1, col: 1 }, true, bounds)).toEqual({ row: 0, col: 1 });
  });

  it("tab wraps across rows and clamps only at the first/last cell", () => {
    expect(tabPosition({ row: 0, col: 3 }, false, bounds)).toEqual({ row: 1, col: 0 });
    expect(tabPosition({ row: 1, col: 0 }, true, bounds)).toEqual({ row: 0, col: 3 });
    expect(tabPosition({ row: 4, col: 3 }, false, bounds)).toEqual({ row: 4, col: 3 });
    expect(tabPosition({ row: 0, col: 0 }, true, bounds)).toEqual({ row: 0, col: 0 });
    expect(enterPosition({ row: 4, col: 0 }, false, bounds)).toEqual({
      row: 4,
      col: 0,
    });
  });
});

describe("rowEdgePosition", () => {
  it("home goes to the first column, end to the last", () => {
    expect(rowEdgePosition({ row: 3, col: 2 }, "home", bounds)).toEqual({
      row: 3,
      col: 0,
    });
    expect(rowEdgePosition({ row: 3, col: 2 }, "end", bounds)).toEqual({
      row: 3,
      col: 3,
    });
  });
});

describe("document and page movement", () => {
  it("moves to the origin or last used cell", () => {
    const used = new Set(["0:1", "2:3", "4:0"]);
    const isEmpty = ({ row, col }: GridPosition) => !used.has(`${row}:${col}`);
    expect(documentEdgePosition("home", bounds, isEmpty)).toEqual({ row: 0, col: 0 });
    expect(documentEdgePosition("end", bounds, isEmpty)).toEqual({ row: 4, col: 3 });
    expect(documentEdgePosition("end", bounds, () => true)).toEqual({ row: 0, col: 0 });
  });

  it("moves by a viewport-sized page and clamps", () => {
    expect(pagePosition({ row: 3, col: 2 }, "up", 2, bounds)).toEqual({
      row: 1,
      col: 2,
    });
    expect(pagePosition({ row: 3, col: 2 }, "down", 20, bounds)).toEqual({
      row: 4,
      col: 2,
    });
  });
});

describe("jumpPosition", () => {
  // One row, columns marked F(illed) or E(mpty): F F F E E F F E
  const rowPattern = ["F", "F", "F", "E", "E", "F", "F", "E"];
  const rowBounds: GridBounds = { rowCount: 1, columnCount: rowPattern.length };
  const isEmpty = (position: GridPosition) => rowPattern[position.col] === "E";

  it("runs to the last filled cell before a gap from inside a filled run", () => {
    expect(jumpPosition({ row: 0, col: 0 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 2,
    });
    expect(jumpPosition({ row: 0, col: 1 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 2,
    });
  });

  it("skips the gap to the next filled cell from the end of a run", () => {
    expect(jumpPosition({ row: 0, col: 2 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 5,
    });
  });

  it("lands on the next filled cell from an empty cell", () => {
    expect(jumpPosition({ row: 0, col: 3 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 5,
    });
    expect(jumpPosition({ row: 0, col: 7 }, "left", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 6,
    });
  });

  it("goes to the far edge when nothing else is filled in that direction", () => {
    expect(jumpPosition({ row: 0, col: 6 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 7,
    });
    expect(jumpPosition({ row: 0, col: 5 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 6,
    });
  });

  it("stays put at the boundary", () => {
    expect(jumpPosition({ row: 0, col: 0 }, "left", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 0,
    });
    expect(jumpPosition({ row: 0, col: 7 }, "right", rowBounds, isEmpty)).toEqual({
      row: 0,
      col: 7,
    });
  });

  it("crosses a fully empty grid to the far edge", () => {
    const empty = () => true;
    expect(jumpPosition({ row: 0, col: 1 }, "right", rowBounds, empty)).toEqual({
      row: 0,
      col: 7,
    });
    expect(
      jumpPosition({ row: 3, col: 0 }, "up", { rowCount: 5, columnCount: 1 }, empty)
    ).toEqual({ row: 0, col: 0 });
  });

  it("works vertically with the same semantics", () => {
    const columnPattern = ["F", "E", "F", "F", "E"];
    const columnBounds: GridBounds = { rowCount: columnPattern.length, columnCount: 1 };
    const isRowEmpty = (position: GridPosition) => columnPattern[position.row] === "E";
    expect(jumpPosition({ row: 0, col: 0 }, "down", columnBounds, isRowEmpty)).toEqual({
      row: 2,
      col: 0,
    });
    expect(jumpPosition({ row: 2, col: 0 }, "down", columnBounds, isRowEmpty)).toEqual({
      row: 3,
      col: 0,
    });
    expect(jumpPosition({ row: 3, col: 0 }, "down", columnBounds, isRowEmpty)).toEqual({
      row: 4,
      col: 0,
    });
  });
});

describe("normalizeRange and positionInRange", () => {
  it("normalizes any anchor/focus order into the same rectangle", () => {
    const expected = { top: 1, left: 0, bottom: 3, right: 2 };
    expect(normalizeRange({ row: 1, col: 0 }, { row: 3, col: 2 })).toEqual(expected);
    expect(normalizeRange({ row: 3, col: 2 }, { row: 1, col: 0 })).toEqual(expected);
    expect(normalizeRange({ row: 1, col: 2 }, { row: 3, col: 0 })).toEqual(expected);
  });

  it("collapses to a single cell when anchor equals focus", () => {
    expect(normalizeRange({ row: 2, col: 2 }, { row: 2, col: 2 })).toEqual({
      top: 2,
      left: 2,
      bottom: 2,
      right: 2,
    });
  });

  it("tests membership inclusively", () => {
    const range = normalizeRange({ row: 1, col: 1 }, { row: 3, col: 2 });
    expect(positionInRange({ row: 1, col: 1 }, range)).toBe(true);
    expect(positionInRange({ row: 3, col: 2 }, range)).toBe(true);
    expect(positionInRange({ row: 2, col: 2 }, range)).toBe(true);
    expect(positionInRange({ row: 0, col: 1 }, range)).toBe(false);
    expect(positionInRange({ row: 2, col: 3 }, range)).toBe(false);
  });
});

describe("whole-grid selections and fill ranges", () => {
  it("creates whole-grid, row, and column ranges", () => {
    expect(fullGridRange(bounds)).toEqual({ top: 0, left: 0, bottom: 4, right: 3 });
    expect(fullRowRange(2, bounds)).toEqual({ top: 2, left: 0, bottom: 2, right: 3 });
    expect(fullColumnRange(1, bounds)).toEqual({
      top: 0,
      left: 1,
      bottom: 4,
      right: 1,
    });
    expect(
      sameRange(fullRowRange(2, bounds), { top: 2, left: 0, bottom: 2, right: 3 })
    ).toBe(true);
  });

  it("finds the connected data region around the active cell", () => {
    const filled = new Set(["1:1", "1:2", "2:2", "2:3", "4:0"]);
    const isEmpty = ({ row, col }: GridPosition) => !filled.has(`${row}:${col}`);
    expect(contiguousDataRange({ row: 1, col: 1 }, bounds, isEmpty)).toEqual({
      top: 1,
      left: 1,
      bottom: 2,
      right: 3,
    });
    expect(contiguousDataRange({ row: 4, col: 0 }, bounds, isEmpty)).toEqual({
      top: 4,
      left: 0,
      bottom: 4,
      right: 0,
    });
    expect(contiguousDataRange({ row: 0, col: 0 }, bounds, isEmpty)).toEqual({
      top: 0,
      left: 0,
      bottom: 0,
      right: 0,
    });
  });

  it("maps fill-down and fill-right sources to every destination", () => {
    const range = { top: 1, left: 1, bottom: 3, right: 2 };
    expect(fillPairs(range, "down")).toEqual([
      { source: { row: 1, col: 1 }, target: { row: 2, col: 1 } },
      { source: { row: 1, col: 2 }, target: { row: 2, col: 2 } },
      { source: { row: 1, col: 1 }, target: { row: 3, col: 1 } },
      { source: { row: 1, col: 2 }, target: { row: 3, col: 2 } },
    ]);
    expect(fillPairs(range, "right")).toEqual([
      { source: { row: 1, col: 1 }, target: { row: 1, col: 2 } },
      { source: { row: 2, col: 1 }, target: { row: 2, col: 2 } },
      { source: { row: 3, col: 1 }, target: { row: 3, col: 2 } },
    ]);
  });
});

describe("isPrintableKey", () => {
  it("accepts single printable characters", () => {
    expect(isPrintableKey({ key: "a", ctrlKey: false, metaKey: false })).toBe(true);
    expect(isPrintableKey({ key: "7", ctrlKey: false, metaKey: false })).toBe(true);
    expect(isPrintableKey({ key: " ", ctrlKey: false, metaKey: false })).toBe(true);
  });

  it("rejects named keys and modifier chords", () => {
    expect(isPrintableKey({ key: "Enter", ctrlKey: false, metaKey: false })).toBe(
      false
    );
    expect(isPrintableKey({ key: "ArrowDown", ctrlKey: false, metaKey: false })).toBe(
      false
    );
    expect(isPrintableKey({ key: "c", ctrlKey: true, metaKey: false })).toBe(false);
    expect(isPrintableKey({ key: "v", ctrlKey: false, metaKey: true })).toBe(false);
  });
});

describe("scrollTopToRevealRow", () => {
  it("returns null when the row is already fully visible", () => {
    // Row 2 spans content pixels [125, 159); with scrollTop 0 and a 300px viewport
    // the visible body band is [57, 300).
    expect(scrollTopToRevealRow(2, 100, 0, 300)).toBeNull();
  });

  it("scrolls up just enough to clear the sticky header", () => {
    const scrollTop = 40 * TABLE_ROW_HEIGHT;
    expect(scrollTopToRevealRow(10, 100, scrollTop, 300)).toBe(10 * TABLE_ROW_HEIGHT);
  });

  it("scrolls down just enough to reveal the row bottom", () => {
    const next = scrollTopToRevealRow(30, 100, 0, 300);
    expect(next).toBe(TABLE_HEADER_HEIGHT + 31 * TABLE_ROW_HEIGHT - 300);
  });

  it("returns 0 for the first row", () => {
    expect(scrollTopToRevealRow(0, 100, 500, 300)).toBe(0);
  });

  it("returns null for out-of-range rows", () => {
    expect(scrollTopToRevealRow(-1, 100, 0, 300)).toBeNull();
    expect(scrollTopToRevealRow(100, 100, 0, 300)).toBeNull();
    expect(scrollTopToRevealRow(0, 0, 0, 300)).toBeNull();
  });

  it("compresses offsets with the capped-spacer scale on huge frames", () => {
    const rowCount = 1_000_000_000;
    const scale = (rowCount * TABLE_ROW_HEIGHT) / MAX_VIRTUAL_SCROLL_HEIGHT;
    const next = scrollTopToRevealRow(500_000_000, rowCount, 0, 300);
    expect(next).not.toBeNull();
    expect(next!).toBeLessThanOrEqual(MAX_VIRTUAL_SCROLL_HEIGHT);
    expect(next!).toBeCloseTo(
      TABLE_HEADER_HEIGHT + (500_000_001 * TABLE_ROW_HEIGHT) / scale - 300,
      0
    );
  });
});

describe("scrollLeftToRevealColumn", () => {
  it("returns null when the cell is already visible", () => {
    expect(scrollLeftToRevealColumn(200, 150, 100, 600)).toBeNull();
  });

  it("scrolls left when the cell is cut off at the left edge", () => {
    expect(scrollLeftToRevealColumn(80, 150, 100, 600)).toBe(80);
  });

  it("scrolls right when the cell overflows the right edge", () => {
    expect(scrollLeftToRevealColumn(700, 150, 100, 600)).toBe(250);
  });

  it("respects a sticky leading column", () => {
    expect(scrollLeftToRevealColumn(200, 150, 100, 600, 180)).toBe(20);
    expect(scrollLeftToRevealColumn(300, 150, 100, 600, 180)).toBeNull();
  });

  it("never returns a negative offset", () => {
    expect(scrollLeftToRevealColumn(10, 150, 100, 600, 180)).toBe(0);
  });
});
