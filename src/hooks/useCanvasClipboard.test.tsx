// @vitest-environment jsdom

import { cleanup, fireEvent, render } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GridFocus } from "../FrameGrid";
import { fixtures, objectNamed } from "../test/support";
import type { Operation } from "../lib/types";
import { useCanvasClipboard } from "./useCanvasClipboard";

const documentView = fixtures.salesWithMargin;
const frame = objectNamed(documentView, "frame", "Monthly sales");
const view = documentView.views.find((candidate) => candidate.objectId === frame.id)!;
const cellFocus: GridFocus = {
  viewId: view.id,
  objectId: frame.id,
  rowId: frame.rows[0].id,
  columnId: frame.columns[0].id,
  mode: "navigate",
  editSeed: null,
  anchor: null,
  span: null,
};
const clipboard = "Item\tAmount\nWidget\t3\n";

function CanvasHarness({
  run,
  gridFocus = null,
}: {
  run: (operation: Operation) => Promise<string | null>;
  gridFocus?: GridFocus | null;
}) {
  const { handleCanvasPaste } = useCanvasClipboard({
    document: documentView,
    gridFocus,
    run,
    insertPosition: () => ({ x: 110, y: 100 }),
  });
  useEffect(() => {
    window.addEventListener("paste", handleCanvasPaste);
    return () => window.removeEventListener("paste", handleCanvasPaste);
  }, [handleCanvasPaste]);
  return null;
}

function paste(target: Element, text: string) {
  return fireEvent.paste(target, {
    clipboardData: { getData: () => text, setData: vi.fn() },
  });
}

describe("useCanvasClipboard", () => {
  afterEach(() => {
    cleanup();
    document.querySelector("[data-framework-grid-clipboard]")?.remove();
  });

  it("arms the native clipboard target and turns a paste into a new frame", () => {
    const run = vi.fn(async () => null);
    render(<CanvasHarness run={run} />);
    const target = document.activeElement!;
    expect(target.hasAttribute("data-framework-grid-clipboard")).toBe(true);

    // `fireEvent` reports whether the default survived: a claimed paste
    // must not also land in the textarea as text.
    expect(paste(target, clipboard)).toBe(false);
    expect(run).toHaveBeenCalledWith({
      type: "addFrameFromPastedText",
      name: "Frame 1",
      text: clipboard,
      x: 110,
      y: 100,
    });
  });

  it("leaves the paste to the grid while a cell has the focus", () => {
    const run = vi.fn(async () => null);
    render(<CanvasHarness run={run} gridFocus={cellFocus} />);
    expect(paste(document.body, clipboard)).toBe(true);
    expect(run).not.toHaveBeenCalled();
  });

  it("makes nothing of an empty clipboard", () => {
    const run = vi.fn(async () => null);
    render(<CanvasHarness run={run} />);
    expect(paste(document.activeElement!, "  \n")).toBe(true);
    expect(run).not.toHaveBeenCalled();
  });

  it("neither takes the keyboard from a text field nor claims its paste", () => {
    const input = document.createElement("input");
    document.body.append(input);
    input.focus();
    const run = vi.fn(async () => null);
    render(<CanvasHarness run={run} />);
    expect(document.activeElement).toBe(input);
    expect(paste(input, clipboard)).toBe(true);
    expect(run).not.toHaveBeenCalled();
    input.remove();
  });
});
