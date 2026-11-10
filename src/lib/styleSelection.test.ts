import { describe, expect, it } from "vitest";
import type { GridContext, GridFocus } from "../FrameGrid";
import { emptyFrameCellStyle } from "../FrameGrid";
import { styleSelectionOperation, selectedStyleSelection } from "./styleSelection";
import type { FrameObject, Row } from "./types";

const rows = ["r1", "r2", "r3", "r4"].map((id) => ({ id, cells: {} }) as unknown as Row);
const frame = {
  kind: "frame",
  id: "f",
  columns: ["a", "b", "c"].map((id) => ({ id, name: id, dataType: "number" })),
  rows,
} as unknown as FrameObject;
const context = {
  frame,
  displayedRows: rows,
  rowOffset: 0,
  totalRows: 4,
  orientation: "recordsAsRows",
  viewportRows: 10,
} as unknown as GridContext;
const focus = (over: Partial<GridFocus>): GridFocus => ({
  viewId: "v",
  objectId: "f",
  rowId: "r1",
  columnId: "a",
  mode: "navigate",
  editSeed: null,
  anchor: null,
  span: null,
  ...over,
});

describe("selectedStyleSelection", () => {
  it("is null for a single cell", () => {
    expect(selectedStyleSelection(context, focus({}))).toBeNull();
  });

  it("covers a dragged rectangle in displayed order", () => {
    expect(
      selectedStyleSelection(context, focus({ rowId: "r3", columnId: "b", anchor: { rowId: "r2", columnId: "a" } }))
    ).toEqual({ kind: "range", rowIds: ["r2", "r3"], columnIds: ["a", "b"] });
  });

  it("keeps whole columns as columns and a single whole column as null", () => {
    expect(
      selectedStyleSelection(
        context,
        focus({ columnId: "b", anchor: { rowId: "r1", columnId: "a" }, span: "column" })
      )
    ).toEqual({ kind: "columns", columnIds: ["a", "b"] });
    expect(selectedStyleSelection(context, focus({ span: "column" }))).toBeNull();
  });
});

describe("styleSelectionOperation", () => {
  const exact = () => emptyFrameCellStyle();

  it("sends only the carried property for a range", () => {
    expect(
      styleSelectionOperation(
        "f",
        { kind: "range", rowIds: ["r1"], columnIds: ["a", "b"] },
        { bold: true },
        exact,
        emptyFrameCellStyle()
      )
    ).toEqual({
      type: "setFrameStyle",
      frameId: "f",
      target: { kind: "range", rowIds: ["r1"], columnIds: ["a", "b"] },
      style: { ...emptyFrameCellStyle(), bold: true },
    });
  });

  it("refuses a null-only patch, which the engine would read as clear-all", () => {
    expect(
      styleSelectionOperation(
        "f",
        { kind: "range", rowIds: ["r1"], columnIds: ["a"] },
        { fillColor: null },
        exact,
        emptyFrameCellStyle()
      )
    ).toBeNull();
  });

  it("sends the empty style for an explicit clear", () => {
    const op = styleSelectionOperation(
      "f",
      { kind: "range", rowIds: ["r1"], columnIds: ["a"] },
      null,
      exact,
      emptyFrameCellStyle()
    );
    expect(op).toMatchObject({ style: emptyFrameCellStyle() });
  });

  it("batches one column target per whole column", () => {
    const op = styleSelectionOperation(
      "f",
      { kind: "columns", columnIds: ["a", "b"] },
      { bold: true },
      exact,
      emptyFrameCellStyle()
    );
    expect(op).toMatchObject({ type: "batch" });
    expect((op as { operations: unknown[] }).operations).toHaveLength(2);
  });
});
