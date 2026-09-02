import { describe, expect, it } from "vitest";
import {
  pinnedCellClass,
  pinnedCellStyle,
  pinnedColumnOffsets,
  pinnedThrough,
} from "./pinnedColumns";

// The measured widths a three-column grid reports: gutter, three columns,
// then the trailing edge, in the order `measureGridColumnWidths` returns them.
const WIDTHS = [40, 150, 120, 200, 26];

describe("pinnedColumnOffsets", () => {
  it("stacks the gutter and each frozen column at its cumulative left edge", () => {
    expect(pinnedColumnOffsets(WIDTHS, 2, 3)).toEqual([0, 40, 190]);
  });

  it("freezes nothing when nothing is asked for, or before the grid is measured", () => {
    expect(pinnedColumnOffsets(WIDTHS, 0, 3)).toEqual([]);
    expect(pinnedColumnOffsets(null, 2, 3)).toEqual([]);
  });

  // A dropped or reordered column can leave the stored count wider than the
  // frame: the view freezes what is there rather than refusing to draw.
  it("clamps a count larger than the frame is wide", () => {
    expect(pinnedColumnOffsets(WIDTHS, 9, 3)).toEqual([0, 40, 190, 310]);
    expect(pinnedColumnOffsets([40, 150, 26], 9, 1)).toEqual([0, 40]);
  });

  it("waits rather than guessing when the measurement is short", () => {
    expect(pinnedColumnOffsets([40, 150], 3, 3)).toEqual([]);
  });
});

describe("pinnedCellStyle and pinnedCellClass", () => {
  const offsets = pinnedColumnOffsets(WIDTHS, 2, 3);

  it("marks the gutter and the frozen columns, and the divider on the last", () => {
    expect(pinnedCellClass(0, offsets)).toBe("pinned-column");
    expect(pinnedCellClass(1, offsets)).toBe("pinned-column");
    expect(pinnedCellClass(2, offsets)).toBe("pinned-column pinned-column-last");
  });

  it("leaves every scrolling cell alone", () => {
    expect(pinnedCellClass(3, offsets)).toBe("");
    expect(pinnedCellStyle(3, offsets)).toEqual({});
    expect(pinnedCellClass(0, [])).toBe("");
  });

  it("hands the stylesheet the offset it should stick to", () => {
    expect(pinnedCellStyle(0, offsets)).toEqual({ "--pinned-left": "0px" });
    expect(pinnedCellStyle(2, offsets)).toEqual({ "--pinned-left": "190px" });
  });
});

describe("pinnedThrough", () => {
  const frame = {
    columns: [{ id: "a" }, { id: "b" }, { id: "c" }],
    display: { pinnedColumns: 2 },
  };

  it("freezes up to and including an unfrozen column", () => {
    expect(pinnedThrough(frame, { id: "c" })).toBe(3);
    expect(pinnedThrough({ columns: frame.columns }, { id: "a" })).toBe(1);
  });

  it("reads a second press on an already frozen column as unfreezing", () => {
    expect(pinnedThrough(frame, { id: "a" })).toBe(0);
    expect(pinnedThrough(frame, { id: "b" })).toBe(0);
  });

  it("asks for nothing on a column this frame does not have", () => {
    expect(pinnedThrough(frame, { id: "gone" })).toBe(0);
  });
});
