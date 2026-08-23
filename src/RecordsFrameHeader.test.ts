import { describe, expect, it } from "vitest";
import {
  filterUsesColumn,
  vectorDropMode,
  vectorTargetColumns,
} from "./RecordsFrameHeader";
import { lookupDragColumnIds } from "./useFrameCardInteraction";

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

describe("vectorDropMode", () => {
  it("pairs an equal-row-length vector dropped on one column", () => {
    expect(vectorDropMode(1, 3)).toBe("pair");
  });

  it("keeps an explicit multi-column vector as a broadcast", () => {
    expect(vectorDropMode(3, 3)).toBe("apply");
  });

  it("keeps a single-value vector in the apply lane", () => {
    expect(vectorDropMode(1, 1)).toBe("apply");
  });
});

describe("lookup column drag", () => {
  const frame = { columns: ["a", "b", "c", "d"].map((id) => ({ id })) } as never;

  it("carries the selected header run when the dragged column is inside it", () => {
    expect(
      lookupDragColumnIds(frame, "c", { top: 0, bottom: 9, left: 1, right: 3 })
    ).toEqual(["b", "c", "d"]);
  });

  it("carries only the dragged column when selection is elsewhere", () => {
    expect(
      lookupDragColumnIds(frame, "a", { top: 0, bottom: 9, left: 1, right: 3 })
    ).toEqual(["a"]);
  });
});
