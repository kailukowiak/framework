import { describe, expect, it } from "vitest";
import { aliasFromFormula } from "./formulaAlias";

describe("aliasFromFormula", () => {
  it("names an aggregate the way you would say it", () => {
    expect(aliasFromFormula("`debit`.sum()")).toBe("Debit Sum");
    expect(aliasFromFormula("`col 1`.sum()")).toBe("Col 1 Sum");
  });

  it("keeps a chain in the order it was written", () => {
    expect(aliasFromFormula("`amount`.abs().sum()")).toBe("Amount Abs Sum");
    expect(aliasFromFormula("`posted_date`.dt.year()")).toBe("Posted_date Year");
  });

  it("names a bare reference after the column", () => {
    expect(aliasFromFormula("`account_code`")).toBe("Account_code");
  });

  // Two columns have no obvious short name, and a confident wrong one is
  // worse than an empty field: it survives into every formula written
  // against the column.
  it("declines when the formula reads more than one column", () => {
    expect(aliasFromFormula("`debit`.sum() - `credit`.sum()")).toBe("");
  });

  it("declines when there is no column to name", () => {
    expect(aliasFromFormula("")).toBe("");
    expect(aliasFromFormula("1 + 1")).toBe("");
    expect(aliasFromFormula("``")).toBe("");
  });

  it("reads an escaped backtick as part of the name", () => {
    expect(aliasFromFormula("`odd``name`.sum()")).toBe("Odd`name Sum");
  });

  it("keeps a name short enough to stay a column header", () => {
    const alias = aliasFromFormula(`\`${"long name ".repeat(8)}\`.sum()`);
    expect(alias.length).toBeLessThanOrEqual(40);
    expect(alias.endsWith("…")).toBe(true);
  });
});
