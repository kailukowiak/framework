// @vitest-environment jsdom
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  filterUsesColumn,
  RecordsFrameHeader,
  vectorDropMode,
  vectorTargetColumns,
} from "./RecordsFrameHeader";
import { lookupDragColumnIds } from "./useFrameCardInteraction";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import type { ComputedFrame, FrameObject } from "./lib/types";

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

describe("RecordsFrameHeader column errors", () => {
  afterEach(cleanup);

  it("shows a column's error in its type row and as the header's title", () => {
    const frame = {
      id: "frame-1",
      columns: [{ id: "amount", name: "Amount", dataType: "number", formula: null }],
      display: {},
      uniqueKeys: [],
    } as unknown as FrameObject;
    const computed = {
      formulas: {},
      columnErrors: {
        amount: 'Source field "Amount" is missing since the last refresh',
      },
    } as unknown as ComputedFrame;
    const model = {
      frame,
      computed,
      virtualRange: { start: 0, end: 0, paddingTop: 0, paddingBottom: 0 },
      canAddColumns: true,
      addColumn: vi.fn(),
      editCalculatedColumn: vi.fn(),
      pairVector: vi.fn(),
      selection: null,
      selectionRange: null,
      frameColumnDrop: null,
      filterPredicates: [],
      sortKeys: [],
      onOperation: vi.fn(),
      onSelect: vi.fn(),
      selectWholeColumn: vi.fn(),
      beginFrameColumnDrag: vi.fn(),
      filterColumn: vi.fn(),
    } as unknown as RecordsAsRowsFrameCardProps;

    const { container } = render(
      <table>
        <RecordsFrameHeader model={model} pinned={[]} />
      </table>
    );

    const typeCell = container.querySelector('tr.type-row th[data-column-id="amount"]')!;
    expect(typeCell.querySelector(".formula-error-line")?.textContent).toBe(
      'Source field "Amount" is missing since the last refresh'
    );
    const nameCell = container.querySelector('th[data-column-id="amount"].column-header')!;
    expect(nameCell.getAttribute("title")).toBe(
      'Source field "Amount" is missing since the last refresh'
    );
  });
});
