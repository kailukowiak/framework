// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useRef, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrameCard } from "./FrameCard";
import { isTextEntryTarget, type GridFocus, type RenderedGrid } from "./FrameGrid";
import { useGridKeyboardNavigation } from "./hooks/useGridKeyboardNavigation";
import type { OperationHandler } from "./lib/handlers";
import type { DocumentView, FrameObject, Selection } from "./lib/types";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "./test/support";

// The claims under test here are the grid's side of the engine's
// `FrameEditing` contract: an editor opens only where a keystroke can become
// a real operation, a refused keystroke says why instead of doing nothing,
// and the rendered grid always re-derives from the DocumentView it is
// handed. What the emitted operations *mean* is Rust's test to write.

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

/** The one page a fixture frame's literal rows make, replayed verbatim. */
function pageOfLiteralRows(frame: FrameObject) {
  return {
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
  };
}

/** The imported fixture's parquet contents, written down, not computed. */
const IMPORTED_ROWS = [
  ["2026-04", "West", "142000", "91000"],
  ["2026-01", "East", "118000", "76000"],
  ["2026-06", "East", "168000", "104000"],
  ["2026-03", "East", "136000", "85000"],
  ["2026-02", "West", "124000", "79000"],
  ["2026-05", "East", "151000", "96000"],
];

function pageOfImportedRows(frame: FrameObject) {
  return {
    frameId: frame.id,
    totalRows: IMPORTED_ROWS.length,
    offset: 0,
    limit: IMPORTED_ROWS.length,
    columns: frame.columns,
    rowIds: [],
    rows: IMPORTED_ROWS,
    styleMatches: [],
  };
}

/**
 * FrameCard mounted the way App mounts it: real grid-focus state, the real
 * keyboard-navigation hook on window keydown, and the rendered-rows map the
 * hook reads. Nothing here computes — the document view is a fixture and
 * pages are replayed by serveInvoke.
 */
function GridHarness({
  documentView,
  onOperation,
  onEditCalculatedColumn = () => {},
}: {
  documentView: DocumentView;
  onOperation: OperationHandler;
  onEditCalculatedColumn?: (frame: FrameObject, column: unknown, rowIndex: number) => void;
}) {
  const frame = documentView.objects.find(
    (object): object is FrameObject => object.kind === "frame"
  )!;
  const view = documentView.views.find((candidate) => candidate.objectId === frame.id)!;
  const computed = documentView.computedFrames[frame.id]!;
  const [, setSelection] = useState<Selection | null>(null);
  const [gridFocus, setGridFocus] = useState<GridFocus | null>(null);
  const renderedRows = useRef(new Map<string, RenderedGrid>());
  const cellFormulaToken = useRef(0);
  const transformColumnToken = useRef(0);
  const handleNavigateKey = useGridKeyboardNavigation({
    document: documentView,
    gridFocus,
    renderedRows,
    cellFormulaToken,
    transformColumnToken,
    setCellFormulaRequest: () => {},
    setTransformColumnRequest: () => {},
    clearActiveFormulaEditor: () => {},
    setGridFocus,
    setSelection,
    setInspectorSection: () => {},
    run: onOperation,
  });
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (isTextEntryTarget(event.target)) return;
      if (gridFocus?.mode === "navigate") handleNavigateKey(event);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [gridFocus, handleNavigateKey]);
  return (
    <FrameCard
      view={view}
      frame={frame}
      computed={computed}
      selection={null}
      gridFocus={gridFocus}
      onSelect={setSelection}
      onGridFocus={setGridFocus}
      onGridStep={() => {}}
      onRenderedRows={(frameId, grid) => {
        if (grid) renderedRows.current.set(frameId, grid);
        else renderedRows.current.delete(frameId);
      }}
      onOperation={onOperation}
      onRearrangeColumns={() => {}}
      onApplyVector={() => {}}
      onPairVector={() => {}}
      onJoinColumns={() => {}}
      onFilterColumn={() => {}}
      onTransformColumn={() => {}}
      onEditCalculatedColumn={onEditCalculatedColumn}
      dataRefreshRevision={0}
    />
  );
}

describe("FrameCard editing gate", () => {
  it("refuses to open an editor on an immutable paged frame and says why", async () => {
    const view = fixtures.importedSales;
    const frame = objectNamed(view, "frame", "Imported sales");
    const reason = view.computedFrames[frame.id]!.editing.reason!;
    serveInvoke({ get_frame_page: () => pageOfImportedRows(frame) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();
    render(<GridHarness documentView={view} onOperation={onOperation} />);

    const cell = (await screen.findAllByText("West"))[0];
    await user.click(cell);
    // Typing lands on the window handler exactly as it does in the app.
    fireEvent.keyDown(window, { key: "9" });
    expect(document.querySelector("input.cell-editor")).toBeNull();
    expect(await screen.findByText(reason)).not.toBeNull();

    // A double-click attempt is refused the same way, and F2 too.
    await user.dblClick(cell);
    fireEvent.keyDown(window, { key: "F2" });
    expect(document.querySelector("input.cell-editor")).toBeNull();
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("opens an editor on a literal column beside a calculated one and emits setCell", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    const region = frame.columns.find((column) => column.name === "Region")!;
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();
    render(<GridHarness documentView={view} onOperation={onOperation} />);

    const cell = (await screen.findAllByText("West"))[0];
    await user.dblClick(cell);
    const editor = document.querySelector<HTMLInputElement>("input.cell-editor");
    expect(editor).not.toBeNull();
    expect(editor!.value).toBe("West");

    await user.clear(editor!);
    await user.type(editor!, "North{Enter}");
    const rowId = frame.rows[0]!.id;
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith({
        type: "setCell",
        frameId: frame.id,
        rowId,
        columnId: region.id,
        raw: "North",
      })
    );
  });

  // Return ends an inline edit everywhere else in the application; here it
  // used to leave the caret sitting in the field with the rename unsent, so
  // the tab strip and the inspector went on showing the old name.
  it("commits a frame rename on Return and gives the field back", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();
    render(<GridHarness documentView={view} onOperation={onOperation} />);

    const title = document.querySelector<HTMLInputElement>("input.frame-name")!;
    await user.clear(title);
    await user.type(title, "Sales by month{Enter}");
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith({
        type: "renameObject",
        objectId: frame.id,
        name: "Sales by month",
      })
    );
    expect(document.activeElement).not.toBe(title);
  });

  it("opens the editor from a typed key, seeded with the keystroke", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();
    render(<GridHarness documentView={view} onOperation={onOperation} />);

    const cell = (await screen.findAllByText("West"))[0];
    await user.click(cell);
    fireEvent.keyDown(window, { key: "5" });
    const editor = document.querySelector<HTMLInputElement>("input.cell-editor");
    expect(editor).not.toBeNull();
    // Type-to-replace: the keystroke that opened the editor is its content.
    expect(editor!.value).toBe("5");
  });

  it("routes a double-click on the calculated column to its formula, not a cell editor", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    const margin = frame.columns.find((column) => column.name === "Margin")!;
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const onEditCalculatedColumn = vi.fn();
    const user = userEvent.setup();
    render(
      <GridHarness
        documentView={view}
        onOperation={onOperation}
        onEditCalculatedColumn={onEditCalculatedColumn}
      />
    );

    await screen.findAllByText("West");
    const marginCell = document.querySelector<HTMLElement>(
      `td[data-column-id="${margin.id}"] button.computed-cell`
    );
    expect(marginCell).not.toBeNull();
    await user.dblClick(marginCell!);
    expect(document.querySelector("input.cell-editor")).toBeNull();
    expect(onEditCalculatedColumn).toHaveBeenCalled();
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("drops a deleted column as soon as the document view does", async () => {
    const before = fixtures.salesWithMargin;
    const after = fixtures.salesMarginDeleteRegion;
    const beforeFrame = objectNamed(before, "frame", "Monthly sales");
    const afterFrame = objectNamed(after, "frame", "Monthly sales");
    let frameForPages = beforeFrame;
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frameForPages) });
    const onOperation = vi.fn().mockResolvedValue(null);
    const { rerender } = render(
      <GridHarness documentView={before} onOperation={onOperation} />
    );
    expect(await screen.findByText("Region")).not.toBeNull();
    await screen.findAllByText("West");

    // The document moves — a delete-column chain save, or its undo, landing
    // as a new DocumentView. The grid must re-derive, header and rows both.
    frameForPages = afterFrame;
    rerender(<GridHarness documentView={after} onOperation={onOperation} />);
    expect(screen.queryByText("Region")).toBeNull();
    await waitFor(() => expect(screen.queryAllByText("West")).toHaveLength(0));
  });
});
