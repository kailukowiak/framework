import { describe, expect, it } from "vitest";
import type { FormulaFunction } from "./types";
import { helpEntries, searchHelpEntries } from "./helpSearch";

const functions: FormulaFunction[] = [
  {
    id: "root.sequence",
    name: "sequence",
    aliases: ["range", "number series", "Excel SEQUENCE"],
    category: "Generators",
    signature: "sequence(start, stop=None, step=1, periods=None)",
    description: "Generate numbers or dates up to, but not including, stop.",
    minimumArguments: 1,
    maximumArguments: 3,
    returnType: "dynamic",
    nullBehavior: "native Polars behavior",
    arguments: [],
  },
  {
    id: "expr.is_between",
    name: ".is_between",
    aliases: ["range", "within"],
    category: "Comparison",
    signature: ".is_between(lower, upper, closed=\"both\")",
    description: "Test whether values fall in a range, ends included.",
    minimumArguments: 2,
    maximumArguments: 2,
    returnType: "boolean",
    nullBehavior: "native Polars behavior",
    arguments: [],
  },
];

describe("help search", () => {
  it("answers an intent question with the guide before raw functions", () => {
    const results = searchHelpEntries(
      helpEntries("formulas", functions),
      "iterator in a variable"
    );

    expect(results[0]?.id).toBe("formula.generate-series-variable");
    expect(results.some((entry) => entry.id === "root.sequence")).toBe(true);
  });

  it("teaches the collection distinction when people search array or list", () => {
    const results = searchHelpEntries(helpEntries("formulas", functions), "array");

    expect(results[0]?.id).toBe("formula.collection-vocabulary");
    expect(results.some((entry) => entry.id === "root.sequence")).toBe(true);
  });

  it("keeps the two meanings of range discoverable", () => {
    const results = searchHelpEntries(helpEntries("formulas", functions), "range");
    const ids = results.map((entry) => entry.id);

    expect(ids).toContain("root.sequence");
    expect(ids).toContain("expr.is_between");
  });

  it("returns exact application references alongside task-shaped guides", () => {
    const join = searchHelpEntries(helpEntries("guide", functions), "join");
    const formatting = searchHelpEntries(
      helpEntries("guide", functions),
      "conditional formatting"
    );

    expect(join[0]?.id).toBe("reference.join");
    expect(join.some((entry) => entry.id === "formula.lookup")).toBe(true);
    expect(formatting[0]?.id).toBe("reference.conditional-format");
  });

  it("surfaces every Wrangle transformation as searchable reference material", () => {
    const entries = helpEntries("guide", functions);

    expect(searchHelpEntries(entries, "pivot")[0]?.id).toBe("reference.pivot");
    expect(searchHelpEntries(entries, "stack frame")[0]?.id).toBe(
      "reference.union"
    );
    expect(searchHelpEntries(entries, "pair vector as column")[0]?.id).toBe(
      "reference.zip-vector"
    );
  });
});
