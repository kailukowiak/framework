import { describe, expect, it } from "vitest";
import { isEditableGridColumn, type GridContext, type GridFocus } from "../FrameGrid";
import type { Column, ComputedFrame, Row, FrameObject } from "./types";
import { formulaBarCell } from "./formulaBarCell";

const columns: Column[] = [
  { id: "a", name: "Amount", dataType: "number", formula: null },
  { id: "b", name: "Tax", dataType: "number", formula: { expression: {} } },
];
const rows: Row[] = [
  {
    id: "r1",
    cells: {
      a: { raw: "12.50", overrideFormula: null },
      b: { raw: "", overrideFormula: null },
    },
  },
];
const frame = {
  kind: "frame",
  id: "frame",
  name: "Sales",
  columns,
  rows,
  derivation: null,
} as FrameObject;
const computed = {
  fingerprint: "test",
  formulas: { b: "`Amount` * 0.05" },
  overrideFormulas: {},
  rows: {},
  summaries: {},
  derivation: null,
  editing: { cells: true, rows: true, overrides: true },
} satisfies ComputedFrame;
const focus = (columnId: string): GridFocus => ({
  viewId: "view",
  objectId: "frame",
  rowId: "r1",
  columnId,
  mode: "navigate",
  editSeed: null,
  anchor: null,
  span: null,
});
const context = (next: ComputedFrame = computed): GridContext => ({
  frame,
  computed: next,
  displayedRows: rows,
  rowOffset: 20,
  totalRows: 21,
  orientation: "recordsAsRows",
  viewportRows: 10,
});

describe("formula bar cells", () => {
  // Sorting a frame moves the cell without changing which cell it is, and
  // the bar names it by where it now sits. The row order the card shows is
  // the only thing that can answer that -- the index the selection was made
  // at was still saying "row 1" after a sort put the cell fourth.
  it("names the row by where the cell sits in the rows on screen", () => {
    const sorted: Row[] = [
      { id: "r0", cells: { a: { raw: "1", overrideFormula: null } } },
      { id: "rx", cells: { a: { raw: "2", overrideFormula: null } } },
      rows[0],
    ];
    expect(
      formulaBarCell(
        {
          ...context(),
          frame: { ...frame, rows: sorted } as FrameObject,
          displayedRows: sorted,
          rowOffset: 0,
          totalRows: 3,
        },
        focus("a")
      )
    ).toMatchObject({ label: "Amount \u00b7 row 3", rowIndex: 2 });
  });

  it("shows the raw literal and the saved calculated-column declaration", () => {
    expect(formulaBarCell(context(), focus("a"))).toMatchObject({
      kind: "literal",
      value: "12.50",
    });
    expect(formulaBarCell(context(), focus("b"))).toMatchObject({
      kind: "calculated",
      value: "`Amount` * 0.05",
    });
  });

  it("routes a Wrangle output back to its chain instead of editing its cells", () => {
    const pipelineColumn = { ...columns[1], formula: null };
    const pipelineContext = context();
    pipelineContext.frame = {
      ...pipelineContext.frame,
      columns: [columns[0], pipelineColumn],
    };

    expect(formulaBarCell(pipelineContext, focus("b"))).toMatchObject({
      kind: "calculated",
      value: "`Amount` * 0.05",
    });
    expect(isEditableGridColumn(computed, pipelineColumn)).toBe(false);
  });

  it("shows a legacy cell formula before the column's default", () => {
    const overridden = {
      ...computed,
      overrideFormulas: { r1: { b: "`Amount` * 0.10" } },
    } satisfies ComputedFrame;
    expect(formulaBarCell(context(overridden), focus("b"))).toMatchObject({
      kind: "override",
      value: "`Amount` * 0.10",
    });
  });

  it("explains a source-backed value instead of offering a dead editor", () => {
    const readOnly = {
      ...computed,
      editing: {
        cells: false,
        rows: false,
        overrides: false,
        reason: "Adopt data to edit its literal values.",
      },
    } as ComputedFrame;
    expect(formulaBarCell(context(readOnly), focus("a"))).toMatchObject({
      kind: "readOnly",
      reason: "Adopt data to edit its literal values.",
    });
  });
});
