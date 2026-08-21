/**
 * Putting text on the clipboard, in a webview that may not have the modern
 * way of doing it.
 *
 * `navigator.clipboard` needs a secure context and, in WebKit, a gesture the
 * browser is still willing to call recent. A packaged desktop window does not
 * reliably satisfy either, so the async clipboard is treated as the optimistic
 * path and the deprecated `execCommand("copy")` as the one that actually
 * lands. The old way needs a real selection in the real document, hence the
 * throwaway textarea.
 */
export async function writeClipboardText(
  text: string,
  scope: Pick<Document, "createElement" | "body" | "execCommand"> = document
): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Falls through: a rejection here is exactly the case the fallback is for.
  }
  return writeWithSelection(text, scope);
}

function writeWithSelection(
  text: string,
  scope: Pick<Document, "createElement" | "body" | "execCommand">
): boolean {
  const node = scope.createElement("textarea");
  node.value = text;
  // Off-screen rather than hidden: `display: none` cannot hold a selection,
  // and `readOnly` keeps the keyboard from appearing on touch.
  node.setAttribute("readonly", "");
  node.style.position = "fixed";
  node.style.top = "-1000px";
  node.style.opacity = "0";
  scope.body.append(node);
  try {
    node.select();
    node.setSelectionRange(0, text.length);
    return scope.execCommand("copy");
  } catch {
    return false;
  } finally {
    node.remove();
  }
}
