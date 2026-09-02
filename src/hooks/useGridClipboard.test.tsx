// @vitest-environment jsdom

import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { useEffect, type RefObject } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { GridFocus, RenderedGrid } from "../FrameGrid";
import { fixtures, objectNamed } from "../test/support";
import type { Operation } from "../lib/types";
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

function ClipboardHarness({ run }: { run: (operation: Operation) => Promise<string | null> }) {
  const clipboard = useGridClipboard({
    document: documentView,
    gridFocus,
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
});
