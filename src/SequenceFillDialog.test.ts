import { describe, expect, it } from "vitest";
import {
  dateSequenceColumnFormula,
  sequenceColumnFormula,
} from "./SequenceFillDialog";

describe("sequence fill formulas", () => {
  it("ties an ascending or descending series to the frame row count", () => {
    expect(sequenceColumnFormula(1, 1)).toBe(
      "sequence(1, 1 + 1 * frame.len(), step=1)"
    );
    expect(sequenceColumnFormula(10, -2)).toBe(
      "sequence(10, 10 - 2 * frame.len(), step=-2)"
    );
  });

  it("ties a calendar series to the frame row count", () => {
    expect(dateSequenceColumnFormula("2026-01-31", 1, "mo")).toBe(
      "sequence(2026-01-31, periods=frame.len(), step=1mo)"
    );
  });
});
