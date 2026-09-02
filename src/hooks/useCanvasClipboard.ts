import { useCallback, useEffect } from "react";
import type { GridFocus } from "../FrameGrid";
import { CARD_SIZES, type CardSize } from "../lib/cardPlacement";
import {
  focusGridClipboardTarget,
  isGridClipboardTarget,
} from "../lib/gridClipboardTarget";
import type { DocumentView, Operation } from "../lib/types";

/**
 * Paste with nothing selected: the clipboard becomes a new frame.
 *
 * A paste into an empty frame already lets the clipboard decide the frame's
 * shape — headers, column count, and types, read by the core's Polars
 * reader. This is the same thing without the empty frame to make first: the
 * text goes to the core as one operation, so one undo takes the whole frame
 * back, and the frame lands where any new card would.
 *
 * The grid's paste handler declines when there is no grid focus and this one
 * declines when there is, so the two share the window's paste event without
 * ever both acting on it.
 */
export function useCanvasClipboard({
  document,
  gridFocus,
  run,
  insertPosition,
}: {
  document: DocumentView | null;
  gridFocus: GridFocus | null;
  run: (operation: Operation) => Promise<string | null>;
  insertPosition: (size: CardSize) => { x: number; y: number };
}) {
  const active = document !== null && gridFocus === null;

  // macOS's native Paste needs a first responder to deliver to (see
  // `gridClipboardTarget.ts`). The grid arms it from a cell click; the canvas
  // arms it whenever nothing else holds the keyboard, and never takes the
  // keyboard from something that does — a dialog's button included.
  useEffect(() => {
    if (!active) return;
    const focused = window.document.activeElement;
    if (!focused || focused === window.document.body || isGridClipboardTarget(focused))
      focusGridClipboardTarget();
  }, [active]);

  const handleCanvasPaste = useCallback(
    (event: ClipboardEvent) => {
      if (!document || gridFocus) return;
      // Only a paste aimed at nothing in particular is a canvas paste. One
      // reaching a text field, or a button in a dialog, belongs to it.
      if (!(isGridClipboardTarget(event.target) || event.target === window.document.body))
        return;
      const text = event.clipboardData?.getData("text/plain") ?? "";
      if (!text.trim()) return;
      event.preventDefault();
      void run({
        type: "addFrameFromPastedText",
        name: "Frame 1",
        text,
        ...insertPosition(CARD_SIZES.frame),
      });
    },
    [document, gridFocus, insertPosition, run]
  );

  return { handleCanvasPaste };
}
