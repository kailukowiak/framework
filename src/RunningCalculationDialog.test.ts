import { describe, expect, it } from "vitest";
import { runningColumnFormula } from "./RunningCalculationDialog";

describe("running calculation formulas", () => {
  it("builds readable cumulative formulas with optional starts and groups", () => {
    expect(runningColumnFormula("Delta", "sum", 100)).toBe(
      "`Delta`.cum_sum(False) + 100"
    );
    expect(runningColumnFormula("Amount", "sum", 0, "Account")).toBe(
      "`Amount`.cum_sum(False).over([`Account`])"
    );
    expect(runningColumnFormula("Amount", "count")).toBe(
      "`Amount`.cum_count(False)"
    );
  });
});
