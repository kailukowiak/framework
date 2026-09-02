import { describe, expect, it } from "vitest";
import { fixtures, objectNamed } from "../test/support";
import { needsDeclaredOrder, splitDraftRow, typedColumnFormula } from "./typedColumnFormula";

describe("formulas typed at a cell", () => {
  it("reads anything after = as the column's formula, and nothing else", () => {
    expect(typedColumnFormula("= `Cost` * 10")).toBe("`Cost` * 10");
    expect(typedColumnFormula("=sequence(1, frame.len() + 1)")).toBe(
      "sequence(1, frame.len() + 1)"
    );
    expect(typedColumnFormula("12*100")).toBeNull();
    expect(typedColumnFormula("=")).toBeNull();
    expect(typedColumnFormula("  =  ")).toBeNull();
  });

  it("asks for a declared order only ahead of a row-position fill", () => {
    expect(needsDeclaredOrder("sequence(2026-01-31, periods=frame.len(), step=1mo)")).toBe(true);
    expect(needsDeclaredOrder("`Cost` * 10")).toBe(false);
  });

  it("splits the new-row line into a row and the columns it declares", () => {
    const frame = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
    const [region, revenue] = [frame.columns[1], frame.columns[2]];
    const { values, formulas } = splitDraftRow(frame, {
      [region.id]: "East",
      [revenue.id]: "=`Cost` * 12",
      [frame.columns[3].id]: "",
    });
    expect(values).toEqual({ [region.id]: "East" });
    expect(formulas).toEqual([{ column: revenue, formula: "`Cost` * 12" }]);
  });
});
