import { describe, expect, it } from "vitest";
import { referenceFreeArithmetic } from "./literalArithmetic";

describe("referenceFreeArithmetic", () => {
  it("accepts arithmetic and strips the spreadsheet's leading =", () => {
    expect(referenceFreeArithmetic("=10*2")).toBe("10*2");
    expect(referenceFreeArithmetic(" 1200 / 12 ")).toBe("1200 / 12");
    expect(referenceFreeArithmetic("(1 + .5) ** 2 - 3 // 2")).toBe("(1 + .5) ** 2 - 3 // 2");
    expect(referenceFreeArithmetic("$250000 * 4.25%")).toBe("$250000 * 4.25%");
    expect(referenceFreeArithmetic("10 % 3")).toBe("10 % 3");
    expect(referenceFreeArithmetic("20")).toBe("20");
  });

  it("accepts element-wise numeric methods called on a value", () => {
    expect(referenceFreeArithmetic("(10 / 3).round(2)")).toBe("(10 / 3).round(2)");
    expect(referenceFreeArithmetic("(2).pow(10)")).toBe("(2).pow(10)");
    expect(referenceFreeArithmetic("(5).clip(0, 3)")).toBe("(5).clip(0, 3)");
  });

  it("refuses anything that could name a column, value or row", () => {
    for (const formula of [
      "`Cost` * 2",
      "Cost * 2",
      "rate * 2",
      "(1).sum()",
      "(1).shift(1)",
      ".round(2)",
      "round(2)",
      '"10" + 2',
      "[1, 2]",
      "1, 2",
      "3d",
      "(1 + 2",
      "1 + 2)",
      "+",
      "",
      "=",
    ])
      expect(referenceFreeArithmetic(formula), formula).toBeNull();
  });
});
