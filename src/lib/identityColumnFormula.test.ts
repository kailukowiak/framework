import { describe, expect, it } from "vitest";
import { isIdentityColumnFormula } from "./identityColumnFormula";

describe("identity column formula", () => {
  it("recognises a pass-through of the column's own name", () => {
    expect(isIdentityColumnFormula("`Revenue`", "Revenue")).toBe(true);
    expect(isIdentityColumnFormula("  `Revenue`  ", "Revenue")).toBe(true);
    expect(isIdentityColumnFormula("Revenue", "Revenue")).toBe(true);
  });

  it("leaves a real calculation alone", () => {
    expect(isIdentityColumnFormula("`Revenue` * 2", "Revenue")).toBe(false);
    expect(isIdentityColumnFormula("`Revenue`", "Profit")).toBe(false);
    expect(isIdentityColumnFormula("`Revenue`.sum()", "Revenue")).toBe(false);
    expect(isIdentityColumnFormula(undefined, "Revenue")).toBe(false);
    expect(isIdentityColumnFormula("", "Revenue")).toBe(false);
  });
});
