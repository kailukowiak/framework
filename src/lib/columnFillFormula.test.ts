import { describe, expect, it } from "vitest";
import { columnFillFormula } from "./columnFillFormula";

describe("column fill formulas typed at a cell", () => {
  it("recognizes numeric and date sequences bound to the frame", () => {
    expect(columnFillFormula("=sequence(1, frame.len() + 1)")).toBe(
      "sequence(1, frame.len() + 1)"
    );
    expect(
      columnFillFormula(
        "=sequence(2026-01-31, periods=frame.len(), step=1mo)"
      )
    ).toBe("sequence(2026-01-31, periods=frame.len(), step=1mo)");
    expect(
      columnFillFormula(
        "=sequence(2026-01-31, periods=frame.n_rows(), step=1mo)"
      )
    ).toBe("sequence(2026-01-31, periods=frame.n_rows(), step=1mo)");
  });

  it("leaves literal text and fixed lists alone", () => {
    expect(columnFillFormula("sequence(1, 10)")).toBeNull();
    expect(columnFillFormula("=sequence(1, 10)")).toBeNull();
  });
});
