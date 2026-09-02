import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useRef } from "react";

type MenuHandlers = Record<string, () => void>;

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
