// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrameCard } from "./FrameCard";
import type { DocumentView, FrameObject } from "./lib/types";
import { clearMocks, fixtures, serveInvoke } from "./test/support";

// The grid has no layout in jsdom, so the measurement it would take of its
// own header is stood in for here. What is under test is everything after
// that: which cells the frozen count reaches, and where each one is told to
// stick. Whether the widths are right is `gridColumnWidths`' own test.
vi.mock("./hooks/useGridColumnWidths", () => ({
  useGridColumnWidths: (_ref: unknown, enabled: boolean, columnCount: number) =>
    enabled ? [40, ...Array<number>(columnCount).fill(100), 26] : null,
}));

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.unstubAllGlobals();
});

/** The fixture, with its one frame frozen through `pinnedColumns` columns. */
function frozenThrough(pinnedColumns: number): DocumentView {
  const view = structuredClone(fixtures.salesWithMargin) as DocumentView;
  const frame = view.objects.find(
    (object): object is FrameObject => object.kind === "frame"
  )!;
  frame.display = { ...frame.display, pinnedColumns } as FrameObject["display"];
  return view;
}

function renderFrozen(pinnedColumns: number) {
  const view = frozenThrough(pinnedColumns);
  const frame = view.objects.find(
    (object): object is FrameObject => object.kind === "frame"
  )!;
  serveInvoke({
    get_frame_page: () => ({
      frameId: frame.id,
      totalRows: frame.rows.length,
      offset: 0,
      limit: frame.rows.length,
      columns: frame.columns,
      rowIds: frame.rows.map((row) => row.id),
      rows: frame.rows.map((row) =>
        frame.columns.map((column) => row.cells[column.id]?.raw ?? "")
      ),
      styleMatches: [],
    }),
  });
  render(
    <FrameCard
      view={view.views.find((candidate) => candidate.objectId === frame.id)!}
      frame={frame}
      computed={view.computedFrames[frame.id]!}
      selection={null}
      gridFocus={null}
      onSelect={() => {}}
      onGridFocus={() => {}}
      onGridStep={() => {}}
      onRenderedRows={() => {}}
      onOperation={vi.fn().mockResolvedValue(null)}
      onRearrangeColumns={() => {}}
      onApplyVector={() => {}}
      onPairVector={() => {}}
      onJoinColumns={() => {}}
      onFilterColumn={() => {}}
      onTransformColumn={() => {}}
      onEditCalculatedColumn={() => {}}
      dataRefreshRevision={0}
    />
  );
  return frame;
}

/** Every cell of one grid column, header rows included, in document order. */
function cellsOfColumn(columnId: string): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>(`[data-column-id="${columnId}"]`));
}

describe("frozen columns in the records grid", () => {
  it("sticks the gutter and the leading columns at their measured offsets", async () => {
    const frame = renderFrozen(2);
    await waitFor(() => expect(screen.getAllByText("West").length).toBeGreaterThan(0));

    // The gutter is frozen with the columns: a key column that scrolls away
    // from its own row number is worse than no freezing at all.
    for (const gutter of document.querySelectorAll<HTMLElement>(
      ".row-number-header, td.row-number"
    )) {
      expect(gutter.classList.contains("pinned-column")).toBe(true);
      expect(gutter.style.getPropertyValue("--pinned-left")).toBe("0px");
    }

    // Header and body alike, and both header rows: a frozen column that only
    // freezes its cells leaves its own name behind.
    const first = cellsOfColumn(frame.columns[0].id);
    const second = cellsOfColumn(frame.columns[1].id);
    expect(first.length).toBeGreaterThan(2);
    expect(second.length).toBe(first.length);
    for (const cell of first) {
      expect(cell.classList.contains("pinned-column")).toBe(true);
      expect(cell.classList.contains("pinned-column-last")).toBe(false);
      expect(cell.style.getPropertyValue("--pinned-left")).toBe("40px");
    }
    for (const cell of second) {
      // 40px of gutter plus the first column's 100.
      expect(cell.style.getPropertyValue("--pinned-left")).toBe("140px");
      // The divider marks where the frozen region ends.
      expect(cell.classList.contains("pinned-column-last")).toBe(true);
    }

    for (const cell of cellsOfColumn(frame.columns[2].id)) {
      expect(cell.classList.contains("pinned-column")).toBe(false);
      expect(cell.style.getPropertyValue("--pinned-left")).toBe("");
    }
  });

  it("freezes nothing at all when the frame asks for nothing", async () => {
    const frame = renderFrozen(0);
    await waitFor(() => expect(screen.getAllByText("West").length).toBeGreaterThan(0));
    expect(document.querySelectorAll(".pinned-column")).toHaveLength(0);
    expect(cellsOfColumn(frame.columns[0].id).length).toBeGreaterThan(0);
  });
});
