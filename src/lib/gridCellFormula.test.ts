import { describe, expect, it } from "vitest";
import { gridCellFormulaAction } from "./gridCellFormula";

const action = (
  key: string,
  options: Partial<Parameters<typeof gridCellFormulaAction>[0]> = {}
) =>
  gridCellFormulaAction({
    key,
    modifier: false,
    printable: key.length === 1,
    isOverride: false,
    singleCell: true,
    wholeColumn: false,
    ...options,
  });

describe("grid formula keyboard gestures", () => {
  it("sends a cell-level equals gesture to Scratchwork", () => {
    expect(action("=")).toEqual({ kind: "scratchwork" });
    expect(action("=", { modifier: true })).toBeNull();
  });

  it("uses the same key on a selected header for the whole column", () => {
    expect(action("=", { wholeColumn: true, singleCell: false })).toEqual({
      kind: "column",
    });
  });

  it("does not turn a range anchor into a hidden cell formula", () => {
    expect(action("=", { singleCell: false })).toEqual({ kind: "scratchwork" });
  });

  it("edits, replaces, and clears an existing formula", () => {
    expect(action("F2", { isOverride: true, printable: false })).toEqual({
      kind: "edit",
      seed: null,
    });
    expect(action("7", { isOverride: true })).toEqual({ kind: "edit", seed: "7" });
    expect(action("Delete", { isOverride: true, printable: false })).toEqual({
      kind: "clear",
    });
    expect(action("=", { isOverride: true })).toEqual({
      kind: "edit",
      seed: "",
    });
  });
});
