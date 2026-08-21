import { describe, expect, it } from "vitest";
import { serializeGrid } from "./gridClipboard";
import { parseGrid } from "./parseGrid";

describe("serializeGrid", () => {
  it("writes ordinary ranges as spreadsheet-compatible TSV", () => {
    expect(
      serializeGrid([
        ["A", "12"],
        ["B", "34"],
      ])
    ).toBe("A\t12\nB\t34");
  });

  it("quotes tabs, newlines, and quotes for lossless round trips", () => {
    const grid = [["a\tb", "two\nlines", 'a "quote"']];
    expect(parseGrid(serializeGrid(grid))).toEqual(grid);
  });
});
