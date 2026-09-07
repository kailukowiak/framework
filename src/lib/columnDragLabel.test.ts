import { describe, expect, it } from "vitest";
import { columnDragActionText, columnDragLabelText } from "./columnDragLabel";

describe("header drag label", () => {
  it("names the carried column, and counts the rest of a run", () => {
    expect(columnDragLabelText(["Budget"])).toBe("Budget");
    expect(columnDragLabelText(["Budget", "Plan", "Owner"])).toBe("Budget +2");
    expect(columnDragLabelText([])).toBe("column");
  });

  it("says what releasing here would do", () => {
    expect(columnDragActionText("join")).toBe("Match to this column");
    expect(columnDragActionText("reorder")).toBe("Move here");
    expect(columnDragActionText(null)).toBe("Drop on a column");
  });
});
