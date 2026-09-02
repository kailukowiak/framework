// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FrameSummaryDrawer } from "./FrameSummaryFooter";
import type { Column, FrameObject } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

const documentView = fixtures.salesWithMargin;
const frame = objectNamed(documentView, "frame", "Monthly sales");

function drawer(columnWidths: number[] | null, pinned?: number[]) {
  return render(
    <FrameSummaryDrawer
      frame={frame}
      summary={{ data: null, loading: false, error: null }}
      height={150}
      columnWidths={columnWidths}
      pinned={pinned}
      drawerRef={{ current: null }}
      scrollRef={{ current: null }}
      onScroll={vi.fn()}
      onResize={vi.fn()}
      onSetRows={vi.fn()}
    />
  );
}

afterEach(cleanup);

describe("FrameSummaryDrawer", () => {
  it("sizes its columns to the grid's measured widths, gutter and edge included", () => {
    const widths = [40, ...frame.columns.map((_, index) => 150 + index * 10), 26];
    const { container } = drawer(widths);
    const cols = Array.from(container.querySelectorAll("col"));
    expect(cols).toHaveLength(frame.columns.length + 2);
    expect(cols.map((col) => col.style.width)).toEqual(widths.map((width) => `${width}px`));
    const total = widths.reduce((sum, width) => sum + width, 0);
    expect(container.querySelector("table")!.style.width).toBe(`${total}px`);
  });

  it("falls back to its own minimum width until the grid has been measured", () => {
    const fallback = `${frame.columns.length * 150 + 66}px`;
    const { container } = drawer(null);
    const cols = Array.from(container.querySelectorAll("col"));
    expect(cols.every((col) => col.style.width === "")).toBe(true);
    expect(container.querySelector("table")!.style.minWidth).toBe(fallback);
    // A measurement from a different table shape is not trusted either.
    cleanup();
    const mismatched = drawer([40, 150, 26]);
    expect(mismatched.container.querySelector("table")!.style.minWidth).toBe(fallback);
  });

  it("formats a date column's min/max the same way its grid cells would", () => {
    const dateColumn: Column = {
      id: "start",
      name: "Start",
      dataType: "date",
      formula: null,
      format: { style: "plain", datePattern: "dayMonthYear" },
    };
    const dateFrame = {
      id: "frame-2",
      columns: [dateColumn],
      summaries: [],
      display: { summaryRows: ["min"] },
    } as unknown as FrameObject;
    render(
      <FrameSummaryDrawer
        frame={dateFrame}
        summary={{
          data: {
            frameId: "frame-2",
            rows: [
              {
                operation: "min",
                label: "Min",
                cells: {
                  start: {
                    value: null,
                    typedValue: { type: "date", value: "2026-01-15" },
                    display: "2026-01-15",
                    error: null,
                    isOverride: false,
                  },
                },
              },
            ],
          },
          loading: false,
          error: null,
        }}
        height={150}
        columnWidths={null}
        drawerRef={{ current: null }}
        scrollRef={{ current: null }}
        onScroll={vi.fn()}
        onResize={vi.fn()}
        onSetRows={vi.fn()}
      />
    );
    expect(screen.getByText("15 Jan 2026")).toBeTruthy();
    expect(screen.queryByText("2026-01-15")).toBeNull();
  });

  it("freezes the gutter and pinned columns the same way the grid does", () => {
    const pinnedFrame = {
      id: "frame-3",
      columns: [
        { id: "a", name: "A", dataType: "number", formula: null },
        { id: "b", name: "B", dataType: "number", formula: null },
      ],
      summaries: [],
      display: { summaryRows: ["min"] },
    } as unknown as FrameObject;
    render(
      <FrameSummaryDrawer
        frame={pinnedFrame}
        summary={{ data: null, loading: false, error: null }}
        height={150}
        columnWidths={null}
        pinned={[0, 40]}
        drawerRef={{ current: null }}
        scrollRef={{ current: null }}
        onScroll={vi.fn()}
        onResize={vi.fn()}
        onSetRows={vi.fn()}
      />
    );
    const gutter = document.querySelector<HTMLElement>("tbody th")!;
    expect(gutter.className).toContain("pinned-column");
    expect(gutter.style.getPropertyValue("--pinned-left")).toBe("0px");
    const firstCell = document.querySelector<HTMLElement>('td[data-column-id="a"]')!;
    expect(firstCell.className).toContain("pinned-column");
    expect((firstCell as HTMLElement).style.getPropertyValue("--pinned-left")).toBe(
      "40px"
    );
    const secondCell = document.querySelector('td[data-column-id="b"]')!;
    expect(secondCell.className).not.toContain("pinned-column");
  });
});
