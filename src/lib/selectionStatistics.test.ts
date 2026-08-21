import { describe, expect, it } from "vitest";
import type { GridContext, GridFocus } from "../FrameGrid";
import type { ComputedFrame, FrameObject } from "./types";
import { selectionStatistics } from "./selectionStatistics";

const frame = {
  kind: "frame",
  id: "t",
  name: "Numbers",
  columns: [
    { id: "a", name: "A", dataType: "number", formula: null },
    { id: "b", name: "B", dataType: "string", formula: null },
  ],
  rows: [
    {
      id: "r1",
      cells: {
        a: { raw: "10", overrideFormula: null },
        b: { raw: "North", overrideFormula: null },
      },
    },
    {
      id: "r2",
      cells: {
        a: { raw: "20", overrideFormula: null },
        b: { raw: "", overrideFormula: null },
      },
    },
  ],
  derivation: null,
  uniqueKeys: [],
  summaries: [],
} as FrameObject;
const computed = {
  fingerprint: "test",
  formulas: {},
  overrideFormulas: {},
  rows: {},
  summaries: {},
  derivation: null,
  editing: { cells: true, rows: true, overrides: true },
} satisfies ComputedFrame;
const context: GridContext = {
  frame,
  computed,
  displayedRows: frame.rows,
  rowOffset: 0,
  totalRows: 2,
  orientation: "recordsAsRows",
  viewportRows: 10,
};
const focus: GridFocus = {
  viewId: "v",
  objectId: "t",
  rowId: "r2",
  columnId: "b",
  mode: "navigate",
  editSeed: null,
  anchor: { rowId: "r1", columnId: "a" },
  span: null,
};

describe("selection statistics", () => {
  it("counts nonblank cells and averages numeric cells", () => {
    expect(selectionStatistics(context, focus)).toEqual({
      selectedCells: 4,
      count: 3,
      numericCount: 2,
      sum: 30,
      average: 15,
      partial: false,
    });
  });

  it("does not pretend a loaded page is the whole selected column", () => {
    expect(
      selectionStatistics(
        { ...context, totalRows: 2_000 },
        { ...focus, anchor: null, columnId: "a", span: "column" }
      )
    ).toEqual({
      selectedCells: 2_000,
      count: null,
      numericCount: null,
      sum: null,
      average: null,
      partial: true,
    });
  });

  it("counts a transposed whole column once per record, not once per loaded cell", () => {
    expect(
      selectionStatistics(
        { ...context, orientation: "fieldsAsRows", totalRows: 2_000 },
        { ...focus, anchor: null, columnId: "a", span: "column" }
      )?.selectedCells
    ).toBe(2_000);
  });
});
