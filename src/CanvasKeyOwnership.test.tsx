// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useEffect, useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GridFocus, RenderedGrid } from "./FrameGrid";
import { useCardSelectionKeys } from "./hooks/useCardSelectionKeys";
import { keyboardBelongsToCanvas } from "./lib/canvasKeyboardOwner";
import type { OperationHandler } from "./lib/handlers";
import type { Selection } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

// The window-level guard App wraps around the card keys, mounted over the
// three places a key can come from: the canvas, a modal dialog and the
// inspector. The claim is which of them may delete the selected card.

afterEach(cleanup);

const document0 = fixtures.salesWithMargin;
const frame = objectNamed(document0, "frame", "Monthly sales");
const selection: Selection = { objectId: frame.id, viewId: document0.views[0]!.id };

function Harness({ run, modal = false }: { run: OperationHandler; modal?: boolean }) {
  const renderedRows = useRef(new Map<string, RenderedGrid>());
  const handleCardKey = useCardSelectionKeys({
    document: document0,
    selection,
    gridFocus: null as GridFocus | null,
    renderedRows,
    setGridFocus: () => {},
    setSelection: () => {},
    run,
    navigateCanvas: () => false,
  });
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (keyboardBelongsToCanvas(event.target)) handleCardKey(event);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handleCardKey]);
  return (
    <>
      <main className="canvas-viewport">
        <div className="canvas-object" data-testid="card">
          Monthly sales
        </div>
      </main>
      <aside className="inspector">
        <p>Inspector note</p>
        <button>Hide</button>
      </aside>
      {modal && (
        <div role="dialog" aria-modal="true" aria-label="Model">
          <h2>Model heading</h2>
        </div>
      )}
    </>
  );
}

const backspace = (target: Element | Window = window) =>
  fireEvent.keyDown(target, { key: "Backspace" });
const deleted = { type: "deleteObject", objectId: frame.id };

describe("who may delete the selected card", () => {
  it("the canvas can, from a target inside it or from body after a bare-canvas click", () => {
    const run = vi.fn().mockResolvedValue(null);
    render(<Harness run={run} />);
    backspace(screen.getByTestId("card"));
    expect(run).toHaveBeenCalledWith(deleted);
    run.mockClear();
    fireEvent.pointerDown(screen.getByTestId("card"));
    backspace(document.body);
    expect(run).toHaveBeenCalledWith(deleted);
  });

  it("a modal dialog blocks it, even with focus fallen to body", () => {
    const run = vi.fn().mockResolvedValue(null);
    render(<Harness run={run} modal />);
    fireEvent.pointerDown(screen.getByText("Model heading"));
    backspace(document.body);
    backspace(screen.getByText("Model heading"));
    expect(run).not.toHaveBeenCalled();
  });

  it("a click on inspector text leaves focus on body but not the key with the canvas", () => {
    const run = vi.fn().mockResolvedValue(null);
    render(<Harness run={run} />);
    fireEvent.pointerDown(screen.getByText("Inspector note"));
    backspace(document.body);
    backspace(screen.getByText("Inspector note"));
    expect(run).not.toHaveBeenCalled();
    // Coming back to the canvas gives it the keyboard again.
    fireEvent.pointerDown(screen.getByTestId("card"));
    backspace(document.body);
    expect(run).toHaveBeenCalledWith(deleted);
  });
});
