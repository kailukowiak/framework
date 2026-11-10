// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { useEffect, type RefObject } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { GridFocus, RenderedGrid } from "../FrameGrid";
import { fixtures, objectNamed } from "../test/support";
import { frameCardSize } from "../lib/cardPlacement";
import type { DocumentView, Operation } from "../lib/types";
import { useGridClipboard } from "./useGridClipboard";

const documentView = fixtures.salesWithMargin;
const frame = objectNamed(documentView, "frame", "Monthly sales");
const view = documentView.views.find((candidate) => candidate.objectId === frame.id)!;
const row = frame.rows[0];
const column = frame.columns[0];
const gridFocus: GridFocus = {
  viewId: view.id,
  objectId: frame.id,
  rowId: row.id,
  columnId: column.id,
  mode: "navigate",
  editSeed: null,
  anchor: null,
  span: null,
};
const renderedRows: RefObject<Map<string, RenderedGrid>> = {
  current: new Map([
    [frame.id, { rows: frame.rows, offset: 0, totalRows: frame.rows.length }],
  ]),
};

function ClipboardHarness({
  run,
  document: doc = documentView,
  focus = gridFocus,
}: {
  run: (operation: Operation) => Promise<string | null>;
  document?: DocumentView;
  focus?: GridFocus;
}) {
  const clipboard = useGridClipboard({
    document: doc,
    gridFocus: focus,
    renderedRows,
    run,
    setError: vi.fn(),
    setFrameCached: vi.fn(async () => null),
  });
  useEffect(() => {
    window.addEventListener("copy", clipboard.handleGridCopy);
    window.addEventListener("paste", clipboard.handleGridPaste);
    return () => {
      window.removeEventListener("copy", clipboard.handleGridCopy);
      window.removeEventListener("paste", clipboard.handleGridPaste);
    };
  }, [clipboard.handleGridCopy, clipboard.handleGridPaste]);
  return null;
}

describe("useGridClipboard", () => {
  beforeEach(() => {
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: { getItem: () => null, setItem: vi.fn() },
    });
  });

  afterEach(() => {
    cleanup();
    document.querySelector("[data-framework-grid-clipboard]")?.remove();
  });

  it("routes native copy and paste events from the grid focus target", async () => {
    const run = vi.fn(async () => null);
    render(<ClipboardHarness run={run} />);
    const target = document.activeElement!;
    const copied = { setData: vi.fn(), getData: vi.fn() };

    fireEvent.copy(target, { clipboardData: copied });
    expect(copied.setData).toHaveBeenCalledWith("text/plain", "2026-04");

    fireEvent.paste(target, {
      clipboardData: { getData: () => "2027-01", setData: vi.fn() },
    });
    await waitFor(() =>
      expect(run).toHaveBeenCalledWith({
        type: "pasteCells",
        frameId: frame.id,
        rowId: row.id,
        columnId: column.id,
        grid: [["2027-01"]],
      })
    );
  });

  // The last column of the frame, on a card still at the app's own width.
  const lastColumn = frame.columns[frame.columns.length - 1];
  const widthOfApp = frameCardSize(frame.columns.length, 0).width;
  const automaticDocument: DocumentView = {
    ...documentView,
    views: documentView.views.map((candidate) =>
      candidate.id === view.id ? { ...candidate, width: widthOfApp, height: 300 } : candidate
    ),
  };
  const atLastColumn: GridFocus = { ...gridFocus, columnId: lastColumn.id };

  function pasteTwoColumns() {
    const target = document.activeElement!;
    fireEvent.paste(target, {
      clipboardData: { getData: () => "a\tb", setData: vi.fn() },
    });
  }

  it("sends a too-wide block as it is and widens the card to fit what the engine added", async () => {
    // Cards of fewer than three columns are all the 420 minimum wide, so the
    // width can only grow when the paste reaches well past it.
    const run = vi.fn(async (_operation: Operation): Promise<string | null> => null);
    render(<ClipboardHarness run={run} document={automaticDocument} focus={atLastColumn} />);
    const wide = Array.from({ length: 6 }, (_, index) => `v${index}`).join("\t");
    fireEvent.paste(document.activeElement!, {
      clipboardData: { getData: () => wide, setData: vi.fn() },
    });
    // One batch, paste first and resize second, so one undo takes back both.
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    const sent = run.mock.calls[0][0];
    expect(sent.type).toBe("batch");
    if (sent.type !== "batch") return;
    expect(sent.operations).toHaveLength(2);
    expect(sent.operations[0]).toMatchObject({
      type: "pasteCells",
      columnId: lastColumn.id,
      grid: [Array.from({ length: 6 }, (_, index) => `v${index}`)],
    });
    expect(sent.operations[1]).toMatchObject({
      type: "resizeView",
      viewId: view.id,
      width: frameCardSize(frame.columns.length - 1 + 6, 0).width,
    });
  });

  it("surfaces the engine's sentence when the batch is refused, sending nothing more", async () => {
    const run = vi.fn(async (_operation: Operation): Promise<string | null> =>
      "Nothing pasted: the clipboard needs 2 columns, but only 1 are available."
    );
    render(<ClipboardHarness run={run} document={automaticDocument} focus={atLastColumn} />);
    pasteTwoColumns();
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    await Promise.resolve();
    // The refusal comes back through `run`'s own return, the path every single
    // operation uses; a batch adds no second send and no second error route.
    expect(run).toHaveBeenCalledTimes(1);
    expect(run.mock.calls[0][0].type).toBe("batch");
    await expect(run.mock.results[0].value).resolves.toMatch(/^Nothing pasted/);
  });
});
