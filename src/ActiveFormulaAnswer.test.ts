import { describe, expect, it } from "vitest";
import { formulaScopeReading } from "./ActiveFormulaAnswer";
import type { ActiveFormulaEditor } from "./lib/activeFormulaEditor";

const active = (appliesToAllRows?: boolean) =>
  ({
    id: "formula",
    label: "Amount",
    kind: "formula",
    draft: "1",
    selection: { start: 1, end: 1 },
    focused: true,
    canCommit: true,
    completion: { references: [], appliesToAllRows },
  } satisfies ActiveFormulaEditor);

describe("formula scope reading", () => {
  it("distinguishes a cell formula from a column declaration", () => {
    expect(formulaScopeReading(active(false))).toBe("this cell");
    expect(formulaScopeReading(active(true))).toBe("all rows");
    expect(formulaScopeReading(active())).toBe("");
  });
});
