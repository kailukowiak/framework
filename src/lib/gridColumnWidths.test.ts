// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { measureGridColumnWidths } from "./gridColumnWidths";

/** A laid-out grid table: header cells starting at `lefts`, on a canvas at `scale`. */
function gridTable(lefts: number[], right: number, scale: number) {
  const table = document.createElement("table");
  table.innerHTML = `<thead><tr>${lefts.map(() => "<th></th>").join("")}</tr></thead>`;
  const width = right - lefts[0];
  Object.defineProperty(table, "offsetWidth", { value: width / scale });
  table.getBoundingClientRect = () => ({ left: lefts[0], right, width }) as DOMRect;
  table.querySelectorAll("th").forEach((cell, index) => {
    cell.getBoundingClientRect = () => ({ left: lefts[index] }) as DOMRect;
  });
  return table;
}

describe("measureGridColumnWidths", () => {
  it("reads each column's extent from where the header cells start, in layout pixels", () => {
    // A card zoomed to 50%: screen pixels are half of layout pixels.
    const table = gridTable([100, 120, 220, 340], 360, 0.5);
    expect(measureGridColumnWidths(table, 4)).toEqual([40, 200, 240, 40]);
  });

  it("declines a header that is not the grid's, and a table with no layout yet", () => {
    expect(measureGridColumnWidths(gridTable([0, 40], 100, 1), 3)).toBeNull();
    const unlaid = document.createElement("table");
    unlaid.innerHTML = "<thead><tr><th></th></tr></thead>";
    expect(measureGridColumnWidths(unlaid, 1)).toBeNull();
  });
});
