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
    expect(parseGrid('City,Amount\n"Calgary, AB",120')).toEqual([
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
});
