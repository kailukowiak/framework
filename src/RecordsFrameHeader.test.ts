import { describe, expect, it } from "vitest";
import { filterUsesColumn } from "./RecordsFrameHeader";

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
