// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrameCard } from "./FrameCard";
import type { GridFocus } from "./FrameGrid";
import type { FrameObject } from "./lib/types";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "./test/support";

// Where the fill handle is allowed to appear. It answers "which column is
// this about" with one square, so a selection that names no single column —
// two columns wide, or one being typed into — carries none at all.

const documentView = fixtures.salesWithMargin;
const frame = objectNamed(documentView, "frame", "Monthly sales");
const canvasView = documentView.views.find(
  (candidate) => candidate.objectId === frame.id
)!;
const computed = documentView.computedFrames[frame.id]!;
const region = frame.columns.find((column) => column.name === "Region")!;
const revenue = frame.columns.find((column) => column.name === "Revenue")!;

function pageOfLiteralRows(source: FrameObject) {
  return {
    frameId: source.id,
    totalRows: source.rows.length,
    offset: 0,
    limit: source.rows.length,
    columns: source.columns,
    rowIds: source.rows.map((row) => row.id),
    rows: source.rows.map((row) =>
      source.columns.map((column) => row.cells[column.id]?.raw ?? "")
    ),
    styleMatches: [],
  };
}

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  );
  serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame) });
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.unstubAllGlobals();
});

function renderGrid(focus: Partial<GridFocus>) {
  return render(
    <FrameCard
      view={canvasView}
      frame={frame}
      computed={computed}
      selection={null}
      gridFocus={{
        viewId: canvasView.id,
        objectId: frame.id,
        rowId: frame.rows[0]!.id,
        columnId: region.id,
        mode: "navigate",
        editSeed: null,
        anchor: null,
        span: null,
        ...focus,
      }}
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
}

/** The row index each rendered handle sits on, in document order. */
function handleRows(): string[] {
  return Array.from(document.querySelectorAll<HTMLElement>("span.fill-handle")).map(
    (handle) => handle.closest("tr")?.dataset.rowIndex ?? "?"
  );
}

describe("fill handle visibility", () => {
  it("puts one square on the bottom cell of a single-column selection", async () => {
    renderGrid({});
    await screen.findAllByText("West");
    expect(handleRows()).toEqual(["0"]);
    // It belongs to the selected column, and sits in a cell that can hold it.
    const handle = document.querySelector<HTMLElement>("span.fill-handle")!;
    const cell = handle.closest("td")!;
    expect(cell.dataset.columnId).toBe(region.id);
    expect(cell.className).toContain("fill-anchor");
  });

  it("follows a taller selection to its last row, still one square", async () => {
    renderGrid({
      rowId: frame.rows[2]!.id,
      anchor: { rowId: frame.rows[0]!.id, columnId: region.id },
    });
    await screen.findAllByText("West");
    expect(handleRows()).toEqual(["2"]);
  });

  it("shows none when the selection spans several columns", async () => {
    renderGrid({
      rowId: frame.rows[2]!.id,
      columnId: revenue.id,
      anchor: { rowId: frame.rows[0]!.id, columnId: region.id },
    });
    await screen.findAllByText("West");
    expect(handleRows()).toEqual([]);
  });

  it("shows none while the cell is being edited", async () => {
    renderGrid({ mode: "edit" });
    await screen.findAllByText("East");
    expect(document.querySelector("input.cell-editor")).not.toBeNull();
    expect(handleRows()).toEqual([]);
  });
});
