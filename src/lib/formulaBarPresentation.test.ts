import { describe, expect, it } from "vitest";
import { functionAtCursor } from "./formulaBarPresentation";
import type { FormulaReference } from "./formulaReferences";

const fn = (label: string, id: string): FormulaReference => ({
  id,
  label,
  token: `${label}(`,
  kind: "function",
  detail: "",
});
const references = [fn("sequence", "root.sequence"), fn(".filter", "expr.filter"), fn(".sum", "expr.sum")];
const at = (source: string) => functionAtCursor(source, source.indexOf("|"), references)?.id;

describe("the function a Scratchwork cursor is inside", () => {
  it("names the innermost open call, method calls included", () => {
    expect(at("sequence(1, |")).toBe("root.sequence");
    expect(at("`Amount`.filter(`Region` == 1|).sum()")).toBe("expr.filter");
    expect(at("`Amount`.filter(`Region` == 1).sum(|)")).toBe("expr.sum");
  });

  it("is nothing once the call is closed, or outside any call", () => {
    expect(at("sequence(1, 13) + |")).toBeUndefined();
    expect(at("revenue - cost|")).toBeUndefined();
  });

  it("ignores brackets inside text and quoted names", () => {
    expect(at('sequence(1, `Odd (name)`, "a(b|')).toBe("root.sequence");
    expect(at('"sequence(" + |')).toBeUndefined();
  });
});
