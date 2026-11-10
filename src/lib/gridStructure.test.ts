import { describe, expect, it } from "vitest";
import { appendColumnOperation, rowPositionIsMeaningful } from "./gridStructure";
import type { FrameObject, Row } from "./types";

const row = (id: string) => ({ id, cells: {} }) as unknown as Row;
const frame = {
  id: "f",
  columns: [{ id: "a" }, { id: "b" }],
  rows: [row("r1"), row("r2"), row("r3")],
} as unknown as FrameObject;

describe("rowPositionIsMeaningful", () => {
  it("holds while the grid draws rows in stored order", () => {
    expect(rowPositionIsMeaningful(frame, undefined, frame.rows)).toBe(true);
  });

  it("fails for a reordered or filtered display, which a position cannot honour", () => {
    expect(
      rowPositionIsMeaningful(frame, undefined, [row("r3"), row("r1"), row("r2")])
    ).toBe(false);
    expect(rowPositionIsMeaningful(frame, undefined, [row("r1"), row("r3")])).toBe(false);
  });

  it("fails for a frame read from a file or connector", () => {
    const imported = { ...frame, connector: {} } as unknown as FrameObject;
    expect(rowPositionIsMeaningful(imported, undefined, imported.rows)).toBe(false);
  });
});

describe("appendColumnOperation", () => {
  it("follows the last column with a single anchor", () => {
    expect(appendColumnOperation(frame, "N", { dataType: "number", values: { r1: "1" } })).toEqual({
      type: "addColumn",
      frameId: "f",
      name: "N",
      dataType: "number",
      afterColumnId: "b",
      values: { r1: "1" },
    });
  });
});
