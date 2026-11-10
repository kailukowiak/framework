import { isGridClipboardTarget } from "./gridClipboardTarget";

/**
 * Where the last pointer press landed, as far as the canvas is concerned.
 *
 * Pressing on text in a panel — the inspector, the left rail, a dialog's
 * heading — moves focus to `body`, because nothing there takes it. A key
 * pressed next has `body` as its target, exactly as one pressed after a click
 * on the bare canvas does, so the target alone cannot say whose key it is.
 * The press that put focus there can, and it is recorded in capture phase so
 * a panel that stops propagation cannot hide it. Focus arriving in the canvas
 * by any other means (a keyboard walk, a card selecting itself) clears it.
 */
let pressedOutsideCanvas = false;

const CANVAS = ".canvas-viewport";

if (typeof window !== "undefined") {
  window.addEventListener(
    "pointerdown",
    (event) => {
      pressedOutsideCanvas =
        event.target instanceof Element && !event.target.closest(CANVAS);
    },
    true
  );
  window.addEventListener(
    "focusin",
    (event) => {
      if (event.target instanceof Element && event.target.closest(CANVAS))
        pressedOutsideCanvas = false;
    },
    true
  );
}

/** Whether a modal surface is open; nothing behind it may act on a key. */
function modalIsOpen(): boolean {
  return (
    document.querySelector('[aria-modal="true"], [role="dialog"]:not([aria-modal="false"])') !==
    null
  );
}

/**
 * Whether a key pressed here is the canvas's to handle — arrows and Tab that
 * move between cards, and Delete taking the selected card away. A control
 * that has its own use for the key keeps it.
 *
 * Deleting a card is destructive and invisible from behind a dialog, so the
 * test is structural wherever it can be: the key belongs to the canvas when
 * its target is inside the canvas, or is `body` (where focus rests after a
 * click on bare canvas) with no modal open and the last press not in some
 * other panel. A target in the inspector, a dialog or anywhere else outside
 * the canvas is not the canvas's.
 *
 * The clipboard target is a textarea only so the native Edit menu has a first
 * responder; the keyboard still belongs to the canvas while it holds focus.
 */
export function keyboardBelongsToCanvas(target: EventTarget | null): boolean {
  if (modalIsOpen()) return false;
  if (isGridClipboardTarget(target)) return true;
  if (!(target instanceof HTMLElement)) return false;
  if (target.closest("button, a, input, textarea, select, [contenteditable]")) return false;
  if (target.closest(CANVAS)) return true;
  return target === document.body && !pressedOutsideCanvas;
}
