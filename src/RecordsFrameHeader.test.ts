import { describe, expect, it } from "vitest";
import { filterUsesColumn, vectorTargetColumns } from "./RecordsFrameHeader";

describe("filterUsesColumn", () => {
  it("marks the header whose backticked reference appears in a condition", () => {
    const predicates = ['`Account name` == "Retail"', "`Amount` > 0"];
    expect(filterUsesColumn(predicates, "Account name")).toBe(true);
    expect(filterUsesColumn(predicates, "Account")).toBe(false);
  });

  it("supports escaped backticks in column names", () => {
    expect(filterUsesColumn(["`a``b`.is_not_null()"], "a`b")).toBe(true);
  });
});

describe("vectorTargetColumns", () => {
  const columns = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

  it("spills a list right from the header it is dropped on", () => {
    expect(vectorTargetColumns(columns, 1, null, 3)).toEqual(["Feb", "Mar", "Apr"]);
  });

  it("uses an evenly divisible selected header run so a pattern can repeat", () => {
    expect(
      vectorTargetColumns(
        columns,
        3,
        { top: 0, left: 0, bottom: 9, right: 5 },
        3
      )
    ).toEqual(columns);
  });
});
