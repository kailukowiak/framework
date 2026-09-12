import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef } from "react";
import { isGridClipboardTarget } from "./lib/gridClipboardTarget";

type MenuHandlers = Record<string, () => void>;

export function ownsTextHistory(target: Element | null): boolean {
  // The grid's hidden textarea receives native clipboard commands, but has
  // no editable draft. Its Undo belongs to the workbook just like its cells.
  if (isGridClipboardTarget(target)) return false;
  if (target instanceof HTMLInputElement) {
    return ![
      "button",
      "checkbox",
      "color",
      "file",
      "image",
      "radio",
      "range",
      "reset",
      "submit",
    ].includes(target.type);
  }
  return Boolean(
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

/**
 * A native menu accelerator reaches the application before WebKit can give
 * the focused field its ordinary editing shortcut. Route history back into
 * that field while it owns the keyboard; workbook history begins only once
 * the draft has been committed.
 */
export function runFocusedTextHistory(
  command: string,
  scope: Pick<Document, "activeElement" | "execCommand"> = document
): boolean {
  if (command !== "undo" && command !== "redo") return false;
  const target = scope.activeElement;
  if (!ownsTextHistory(target)) return false;
  scope.execCommand(command);
  return true;
}

/** Keeps native menu plumbing separate from the document actions it names. */
export function useApplicationMenu(
  enabled: boolean,
  handlers: MenuHandlers,
  onError: (message: string) => void
) {
  const handlersRef = useRef(handlers);
  const errorRef = useRef(onError);
  handlersRef.current = handlers;
  errorRef.current = onError;

  useEffect(() => {
    if (!enabled) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    // Tauri's top-level `listen` defaults to the `Any` target. An Any
    // listener is intentionally reached even by `emit_to(window_label, ...)`,
    // so using it here makes one native menu command run in every open
    // workbook. Listening through this webview window gives the listener the
    // exact WebviewWindow target that menu::forward emits to.
    void getCurrentWebviewWindow().listen<string>("framework-menu-command", (event) => {
      if (!runFocusedTextHistory(event.payload))
        handlersRef.current[event.payload]?.();
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((reason) =>
        errorRef.current(`Could not subscribe to the menu: ${String(reason)}`)
      );
    return () => {
      disposed = true;
      stop?.();
    };
  }, [enabled]);
}
