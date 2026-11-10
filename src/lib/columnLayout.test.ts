import { describe, expect, it } from "vitest";
import {
  AUTOMATIC_COLUMN_WIDTH,
  columnLayoutOperation,
  columnWidthFor,
  gridRowHeight,
  gridTableStyle,
  resizeTargets,
  usesFixedColumns,
  WRAPPED_ROW_HEIGHT,
} from "./columnLayout";

type Layout = { columns: unknown[] };

const frame = {
  id: "f",
  columns: [{ id: "a" }, { id: "b" }, { id: "c" }],
  display: { columnLayout: { a: { width: 90, wrap: true }, b: { wrap: true } } },
};

describe("columnLayoutOperation", () => {
  it("repeats the stored wrap on a resize and the stored width on a wrap toggle", () => {
    expect(columnLayoutOperation(frame, [{ columnId: "a", width: 200 }])).toEqual({
      type: "setFrameDisplayColumnLayout",
      frameId: "f",
      columns: [{ columnId: "a", width: 200, wrap: true }],
    });
    expect((columnLayoutOperation(frame, [{ columnId: "a", wrap: false }]) as Layout).columns).toEqual([
      { columnId: "a", width: 90, wrap: false },
    ]);
  });

  it("clamps widths and maps reset to null", () => {
    const op = columnLayoutOperation(frame, [
      { columnId: "c", width: 5 },
      { columnId: "a", width: null },
    ]);
    expect((op as Layout).columns).toEqual([
      { columnId: "c", width: 24, wrap: false },
      { columnId: "a", width: null, wrap: true },
    ]);
  });
});

describe("rendering helpers", () => {
  it("gives automatic columns a fixed stand-in once any column is sized", () => {
    expect(usesFixedColumns(frame)).toBe(true);
    expect(columnWidthFor(frame, "a")).toBe(90);
    expect(columnWidthFor(frame, "c")).toBe(AUTOMATIC_COLUMN_WIDTH);
    expect(columnWidthFor(frame, "a", { a: 300 })).toBe(300);
    expect(gridTableStyle(frame, { a: 300 }).width).toBe(300 + 150 + 150 + 66);
  });

  it("leaves an unstyled frame to size itself", () => {
    const plain = { id: "f", columns: [{ id: "a" }], display: {} };
    expect(columnWidthFor(plain, "a")).toBeNull();
    expect(gridRowHeight(plain)).toBe(34);
    expect(gridRowHeight(frame)).toBe(WRAPPED_ROW_HEIGHT);
  });

  it("applies a resize to the whole-column selection only when it holds the column", () => {
    expect(resizeTargets(frame, "b", { left: 0, right: 1 })).toEqual(["a", "b"]);
    expect(resizeTargets(frame, "c", { left: 0, right: 1 })).toEqual(["c"]);
    expect(resizeTargets(frame, "c", null)).toEqual(["c"]);
  });
});
