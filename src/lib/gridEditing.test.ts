import { describe, expect, it } from "vitest";
import type { GridContext, GridFocus } from "../FrameGrid";
import type { Column, ComputedFrame, Row, FrameObject } from "./types";
import { clearGridRangeUpdates, fillGridRangeUpdates } from "./gridEditing";

const columns: Column[] = [
  { id: "amount", name: "Amount", dataType: "number", formula: null },
  { id: "memo", name: "Memo", dataType: "string", formula: null },
  {
    id: "total",
    name: "Total",
    dataType: "number",
    formula: { expression: { kind: "literal", value: { type: "number", value: 0 } } },
  } as Column,
];
const rows: Row[] = [
  {
    id: "first",
    cells: {
      amount: { raw: "10", overrideFormula: null },
      memo: { raw: "A", overrideFormula: null },
      total: { raw: "", overrideFormula: null },
    },
  },
  {
    id: "second",
    cells: {
      amount: { raw: "20", overrideFormula: null },
      memo: { raw: "B", overrideFormula: null },
      total: { raw: "", overrideFormula: null },
    },
  },
];
const frame = {
  kind: "frame",
  id: "ledger",
  name: "Ledger",
  columns,
  rows,
  derivation: null,
  uniqueKeys: [],
  summaries: [],
} as FrameObject;
const computed = {
  editing: { cells: true, rows: true, overrides: true },
  rows: {},
  // Always present on a real one, so a fixture without it is a lie the grid
  // reads straight through: a Wrangle output is a calculated column even
  // when nothing is declared on the column itself, and that is the map it
  // asks. Total below carries its formula on the column, which is the other
  // half of the same affordance.
  formulas: {},
} as ComputedFrame;
const context: GridContext = {
  frame,
  computed,
  displayedRows: rows,
  rowOffset: 0,
  totalRows: 2,
  orientation: "recordsAsRows",
  viewportRows: 2,
};
const focus: GridFocus = {
  viewId: "view",
  objectId: frame.id,
  rowId: "second",
  columnId: "total",
  mode: "navigate",
  editSeed: null,
  anchor: { rowId: "first", columnId: "amount" },
  span: null,
};

describe("literal range editing", () => {
  it("clears editable cells and leaves calculated columns alone", () => {
    expect(clearGridRangeUpdates(context, focus)).toEqual([
      { rowId: "first", columnId: "amount", raw: "" },
      { rowId: "first", columnId: "memo", raw: "" },
      { rowId: "second", columnId: "amount", raw: "" },
      { rowId: "second", columnId: "memo", raw: "" },
    ]);
  });

  it("fills values down without materializing formulas cell by cell", () => {
    expect(fillGridRangeUpdates(context, focus, "down")).toEqual([
      { rowId: "second", columnId: "amount", raw: "10" },
      { rowId: "second", columnId: "memo", raw: "A" },
    ]);
  });
});
