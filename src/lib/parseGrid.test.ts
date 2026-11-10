import { describe, expect, it } from "vitest";
import { parseGrid } from "./parseGrid";

describe("parseGrid", () => {
  it("parses cells copied from a spreadsheet", () => {
    expect(parseGrid("Name\tAmount\nAlpha\t120\nBeta\t85")).toEqual([
      ["Name", "Amount"],
      ["Alpha", "120"],
      ["Beta", "85"],
    ]);
  });

  it("keeps commas inside quoted CSV values", () => {
    expect(parseGrid('City,Amount\n"Calgary, AB",120', { csv: true })).toEqual([
      ["City", "Amount"],
      ["Calgary, AB", "120"],
    ]);
  });

  it("preserves tabs and newlines inside quoted clipboard cells", () => {
    expect(parseGrid('Alpha\t"two\tparts"\nBeta\t"two\nlines"')).toEqual([
      ["Alpha", "two\tparts"],
      ["Beta", "two\nlines"],
    ]);
  });

  it("does not create a phantom row for a trailing newline", () => {
    expect(parseGrid("A\tB\n")).toEqual([["A", "B"]]);
  });

  it("treats a comma as data when pasting into cells", () => {
    expect(parseGrid("1,234")).toEqual([["1,234"]]);
    expect(parseGrid("Smith, John")).toEqual([["Smith, John"]]);
    expect(parseGrid("a,b\nc,d")).toEqual([["a,b"], ["c,d"]]);
    expect(parseGrid("x,y\t3")).toEqual([["x,y", "3"]]);
  });

  it("splits on commas only when asked to read CSV", () => {
    expect(parseGrid("City, Amount\nCalgary, 120", { csv: true })).toEqual([
      ["City", "Amount"],
      ["Calgary", "120"],
    ]);
  });

  it("keeps a cell's own spaces when tab-separated", () => {
    expect(parseGrid("  a \t b")).toEqual([["  a ", " b"]]);
  });

  it("drops a trailing CRLF", () => {
    expect(parseGrid("A\tB\r\n")).toEqual([["A", "B"]]);
  });
});
