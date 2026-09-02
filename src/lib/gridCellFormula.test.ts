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
    ...options,
  });

describe("grid formula keyboard gestures", () => {
  it("opens the column formula from any cell, range, or selected header", () => {
    expect(action("=")).toEqual({ kind: "column" });
    expect(action("=", { singleCell: false })).toEqual({ kind: "column" });
    expect(action("=", { modifier: true })).toBeNull();
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
    expect(action("=", { isOverride: true })).toEqual({ kind: "edit", seed: "" });
    expect(action("=", { isOverride: true, singleCell: false })).toEqual({ kind: "column" });
  });
});
