import { describe, expect, it } from "vitest";
import type { GridFocus } from "../FrameGrid";
import {
  columnRangeKey,
  emptyColumnRangeMemo,
  stableColumnRange,
} from "./selectedColumnRange";

const focus: GridFocus = {
  viewId: "v",
  objectId: "f",
  rowId: "r1",
  columnId: "profit",
  mode: "navigate",
  editSeed: null,
  anchor: { rowId: "r1", columnId: "revenue" },
  span: "column",
};

describe("selected column range", () => {
  it("adopts whatever the current selection resolves to", () => {
    const key = columnRangeKey(focus);
    expect(
      stableColumnRange(emptyColumnRangeMemo, key, ["revenue", "cost", "profit"])
    ).toEqual({ key, ids: ["revenue", "cost", "profit"] });
  });

  // Applying a format re-reads a chained frame's pages, and for a render or
  // two the card has no rows for the range to resolve against.
  it("keeps the range while the selection is unchanged and nothing resolves", () => {
    const key = columnRangeKey(focus);
    const memo = stableColumnRange(emptyColumnRangeMemo, key, [
      "revenue",
      "cost",
      "profit",
    ]);
    expect(stableColumnRange(memo, key, [])).toEqual(memo);
  });

  it("drops the range as soon as the selection itself moves", () => {
    const memo = stableColumnRange(emptyColumnRangeMemo, columnRangeKey(focus), [
      "revenue",
      "cost",
      "profit",
    ]);
    const narrowed: GridFocus = { ...focus, anchor: null, span: null };
    expect(
      stableColumnRange(memo, columnRangeKey(narrowed), ["profit"])
    ).toEqual({ key: columnRangeKey(narrowed), ids: ["profit"] });
  });

  it("has no range at all with nothing selected", () => {
    expect(columnRangeKey(null)).toBe("");
    expect(stableColumnRange(emptyColumnRangeMemo, "", [])).toEqual(
      emptyColumnRangeMemo
    );
  });

  it("tells a whole-column selection apart from the same cell alone", () => {
    expect(columnRangeKey(focus)).not.toBe(
      columnRangeKey({ ...focus, anchor: null, span: null })
    );
  });
});
