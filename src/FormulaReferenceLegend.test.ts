import { describe, expect, it } from "vitest";
import { relativeRowReading } from "./FormulaReferenceLegend";

describe("relative row readings", () => {
  it("translates shift direction into spreadsheet language", () => {
    expect(relativeRowReading(0)).toBeNull();
    expect(relativeRowReading(1)).toBe("previous row");
    expect(relativeRowReading(3)).toBe("3 rows earlier");
    expect(relativeRowReading(-1)).toBe("next row");
    expect(relativeRowReading(-2)).toBe("2 rows later");
  });
});
