import { describe, expect, it } from "vitest";
import { parseRecurrenceFormula, recurrenceFormula } from "./RecurrenceDialog";

describe("recurrence formula authoring", () => {
  it("round-trips nested formulas and an optional restart column", () => {
    const formula = recurrenceFormula(
      "100",
      "when(`Close`).then(0).otherwise(previous() + `Change`)",
      "Account"
    );
    expect(parseRecurrenceFormula(formula)).toEqual({
      seed: "100",
      next: "when(`Close`).then(0).otherwise(previous() + `Change`)",
      partitionName: "Account",
    });
  });

  it("refuses incomplete or unrecognized wrappers", () => {
    expect(parseRecurrenceFormula("recur(0)")).toBeNull();
    expect(parseRecurrenceFormula("sum(previous())")).toBeNull();
  });
});
