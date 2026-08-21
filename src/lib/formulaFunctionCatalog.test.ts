import { describe, expect, it } from "vitest";
import { previewFormulaFunctions } from "./formulaFunctionCatalog";
import { generatedFormulaFunctions } from "./formulaFunctionCatalog.generated";
import { filterFormulaReferences, type FormulaReference } from "./formulaReferences";

const expectedLength = 83 + generatedFormulaFunctions.length;

describe("preview formula function catalog", () => {
  it("mirrors the canonical native Polars registry without duplicate identities", () => {
    expect(previewFormulaFunctions).toHaveLength(expectedLength);
    expect(new Set(previewFormulaFunctions.map(({ id }) => id)).size).toBe(
      expectedLength
    );
    expect(new Set(previewFormulaFunctions.map(({ name }) => name)).size).toBe(
      expectedLength
    );
    expect(
      previewFormulaFunctions.every(
        ({ signature, description }) => signature && description
      )
    ).toBe(true);
    expect(
      previewFormulaFunctions.find(({ id }) => id === "dt.month_end")?.returnType
    ).toBe("date");
    expect(
      previewFormulaFunctions.find(({ id }) => id === "root.today")?.returnType
    ).toBe("date");
    expect(
      previewFormulaFunctions.find(({ id }) => id === "root.coalesce")?.nullBehavior
    ).toBe("handles nulls");
    expect(
      previewFormulaFunctions.find(({ id }) => id === "str.strip_chars")
    ).toBeDefined();
    expect(
      previewFormulaFunctions.find(({ id }) => id === "expr.quantile")?.signature
    ).toBe(".quantile(fraction)");
  });

  it("makes friendly aliases searchable by autocomplete", () => {
    const references: FormulaReference[] = previewFormulaFunctions.map((fn) => ({
      id: fn.id,
      label: fn.name,
      token: `${fn.name}(`,
      detail: `${fn.signature} · ${fn.category}`,
      kind: "function",
      searchTerms: fn.aliases,
    }));

    expect(filterFormulaReferences(references, "cube root")[0]?.id).toBe("expr.cbrt");
    expect(filterFormulaReferences(references, "inverse tangent")[0]?.id).toBe(
      "expr.arctan"
    );
    expect(filterFormulaReferences(references, "significant figures")[0]?.id).toBe(
      "expr.round_sig_figs"
    );
    expect(filterFormulaReferences(references, "replace null")[0]?.id).toBe(
      "expr.fill_null"
    );
    expect(filterFormulaReferences(references, "month end")[0]?.id).toBe(
      "dt.month_end"
    );
    expect(filterFormulaReferences(references, "row sum")[0]?.id).toBe(
      "root.sum_horizontal"
    );
    expect(filterFormulaReferences(references, "Excel SEQUENCE")[0]?.id).toBe(
      "root.sequence"
    );
    expect(filterFormulaReferences(references, "row count")[0]?.id).toBe(
      "root.frame_len"
    );
    expect(filterFormulaReferences(references, "calculate down rows")[0]?.id).toBe(
      "root.recur"
    );
    expect(filterFormulaReferences(references, "SUMIFS")[0]?.id).toBe("expr.filter");
    expect(filterFormulaReferences(references, "DATEVALUE")[0]?.id).toBe("str.to_date");
  });
});
