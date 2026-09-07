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

  // A frame with a transformation chain is read through pages, so its rows
  // arrive already evaluated while `computed.rows` still describes the
  // stored input — where a calculated column has no literal and reads as
  // null. Trusting that made the aggregate skip the calculated columns.
  it("aggregates a paged frame's calculated column from the rows on screen", () => {
    const pagedFrame = {
      ...frame,
      columns: [
        ...frame.columns,
        { id: "c", name: "Profit", dataType: "number", formula: null },
      ],
      rows: [
        { id: "r1", cells: { ...frame.rows[0].cells, c: { raw: "4", overrideFormula: null } } },
        { id: "r2", cells: { ...frame.rows[1].cells, c: { raw: "6", overrideFormula: null } } },
      ],
    } as FrameObject;
    const pagedContext: GridContext = {
      ...context,
      frame: pagedFrame,
      displayedRows: pagedFrame.rows,
      computed: {
        ...computed,
        paged: true,
        // What the engine publishes for a chained frame: the stored input,
        // in which the chain's own column is empty.
        rows: {
          r1: { c: { value: null, typedValue: { type: "null" }, display: "", error: null, isOverride: false } },
          r2: { c: { value: null, typedValue: { type: "null" }, display: "", error: null, isOverride: false } },
        },
      },
    };
    expect(
      selectionStatistics(pagedContext, {
        ...focus,
        columnId: "c",
        anchor: { rowId: "r1", columnId: "c" },
      })
    ).toEqual({
      selectedCells: 2,
      count: 2,
      numericCount: 2,
      sum: 10,
      average: 5,
      partial: false,
    });
  });

  // The evaluated answer still wins where there is one: a percentage column
  // holds 0.15 as a typed value and "15%" as text, and only one of those is
  // the number to add up.
  it("prefers the evaluated cell over the text beside it", () => {
    expect(
      selectionStatistics(
        {
          ...context,
          computed: {
            ...computed,
            rows: {
              r1: {
                a: {
                  value: 0.15,
                  typedValue: { type: "number", value: 0.15 },
                  display: "15%",
                  error: null,
                  isOverride: false,
                },
              },
            },
          },
        },
        { ...focus, rowId: "r1", columnId: "a", anchor: { rowId: "r1", columnId: "a" } }
      )?.sum
    ).toBe(0.15);
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
