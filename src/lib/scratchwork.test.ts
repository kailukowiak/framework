import { describe, expect, it } from "vitest";
import {
  appendScratchworkLine,
  continueScratchworkLine,
  mergeStoredScratchwork,
  replaceScratchworkLine,
  scratchworkLineAt,
} from "./scratchwork";

describe("appendScratchworkLine", () => {
  it("makes the first formula the first line", () => {
    expect(appendScratchworkLine("", "4100 * 1.2")).toEqual({
      source: "4100 * 1.2",
      lineIndex: 0,
    });
  });

  it("adds one separator after existing work", () => {
    expect(appendScratchworkLine("rate = 5%", "200 * rate")).toEqual({
      source: "rate = 5%\n200 * rate",
      lineIndex: 1,
    });
  });

  it("uses an existing trailing blank instead of adding another", () => {
    expect(appendScratchworkLine("rate = 5%\n", "200 * rate")).toEqual({
      source: "rate = 5%\n200 * rate",
      lineIndex: 1,
    });
  });

  it("counts deliberate blank lines when locating the appended answer", () => {
    expect(appendScratchworkLine("rate = 5%\n\n", "200 * rate")).toEqual({
      source: "rate = 5%\n\n200 * rate",
      lineIndex: 2,
    });
  });
});

describe("scratchworkLineAt", () => {
  it("isolates the line containing the cursor", () => {
    expect(scratchworkLineAt("first\nsecond\nthird", 9)).toEqual({
      start: 6,
      end: 12,
      source: "second",
    });
  });

  it("chooses the new line when the cursor follows a newline", () => {
    expect(scratchworkLineAt("first\nsecond", 6)).toEqual({
      start: 6,
      end: 12,
      source: "second",
    });
  });

  it("represents a trailing blank line", () => {
    expect(scratchworkLineAt("first\n", 6)).toEqual({
      start: 6,
      end: 6,
      source: "",
    });
  });

  it("represents a blank first line", () => {
    expect(scratchworkLineAt("\nsecond", 0)).toEqual({
      start: 0,
      end: 0,
      source: "",
    });
  });
});

describe("replaceScratchworkLine", () => {
  it("changes only the mirrored line and carries its local selection back", () => {
    expect(
      replaceScratchworkLine("first\nsecond\nthird", 9, "2 + 2", {
        start: 5,
        end: 5,
      })
    ).toEqual({
      source: "first\n2 + 2\nthird",
      selection: { start: 11, end: 11 },
    });
  });
});

describe("mergeStoredScratchwork", () => {
  it("accepts a stable-id rename in the focused expression", () => {
    expect(
      mergeStoredScratchwork(
        "=`Sales by Region`.`Revenue Sum`.sum()",
        "=`Monthly sales frame`.`Revenue Sum`.sum()",
        0
      )
    ).toBe("=`Sales by Region`.`Revenue Sum`.sum()");
  });

  it("keeps only a declaration name that is still being typed", () => {
    expect(
      mergeStoredScratchwork(
        "check = `Sales by Region`.`Revenue Sum`.sum()",
        "check_2 = `Monthly sales frame`.`Revenue Sum`.sum()",
        0
      )
    ).toBe("check_2 = `Sales by Region`.`Revenue Sum`.sum()");
  });

  it("handles a quoted declaration and leaves other lines to the document", () => {
    expect(
      mergeStoredScratchwork(
        "`net check` = 10\n`Sales by Region`.`Revenue Sum`.sum()",
        "`new check` = 10\n`Monthly sales frame`.`Revenue Sum`.sum()",
        0
      )
    ).toBe("`new check` = 10\n`Sales by Region`.`Revenue Sum`.sum()");
  });
});

describe("continueScratchworkLine", () => {
  it("expands only the calculation under the block cursor", () => {
    const source = "first = 1\ntotal = values\nlast = 3";
    const cursor = source.indexOf("values") + "values".length;
    expect(continueScratchworkLine(source, cursor, cursor).source).toBe(
      "first = 1\ntotal = (\n  values\n  \n)\nlast = 3"
    );
  });
});
