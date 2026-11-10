const TARGET_ATTRIBUTE = "data-framework-grid-clipboard";
let target: HTMLTextAreaElement | null = null;

function clipboardTarget(): HTMLTextAreaElement {
  if (target?.isConnected) return target;
  target = document.createElement("textarea");
  target.setAttribute(TARGET_ATTRIBUTE, "");
  target.setAttribute("aria-hidden", "true");
  target.tabIndex = -1;
  target.spellcheck = false;
  Object.assign(target.style, {
    position: "fixed",
    width: "1px",
    height: "1px",
    left: "-10000px",
    opacity: "0",
    pointerEvents: "none",
  });
  document.body.append(target);
  return target;
}

/**
 * Gives macOS's native Cut/Copy/Paste selectors a real first responder.
 *
 * A grid selection lives in React state rather than a browser text selection,
 * so WKWebView otherwise considers the predefined Edit-menu commands
 * inapplicable and never raises the clipboard events the grid handles. The
 * selected space makes Copy available; the grid's copy/cut handler replaces
 * it synchronously with the selected cells, while Paste is prevented and
 * interpreted from the event's clipboard data.
 */
export function focusGridClipboardTarget() {
  const element = clipboardTarget();
  element.value = " ";
  element.focus({ preventScroll: true });
  element.select();
}

export function isGridClipboardTarget(value: EventTarget | null): boolean {
  return value instanceof HTMLElement && value.hasAttribute(TARGET_ATTRIBUTE);
}
