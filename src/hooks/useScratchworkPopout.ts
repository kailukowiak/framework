import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useState } from "react";
import type { useActiveFormulaEditorCommands } from "../ActiveFormulaEditor";
import {
  focusScratchworkWindow,
  openScratchworkWindow,
} from "../lib/api";
import { reportIgnoredFailure } from "../lib/errorReporting";
import { sameName, SCRATCHWORK } from "../lib/scratchwork";
import type { DocumentView, Operation } from "../lib/types";

/** Placement and lifecycle of the one native Scratchwork view. */
export function useScratchworkPopout({
  document,
  run,
  insertPosition,
  getActive,
  commitActive,
  clearActive,
  closeDrawer,
  setError,
}: {
  document: DocumentView | null;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  insertPosition: () => { x: number; y: number };
  getActive: ReturnType<typeof useActiveFormulaEditorCommands>["getActive"];
  commitActive: ReturnType<typeof useActiveFormulaEditorCommands>["commit"];
  clearActive: ReturnType<typeof useActiveFormulaEditorCommands>["clear"];
  closeDrawer: () => void;
  setError: (value: string | null) => void;
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    void getCurrentWebviewWindow()
      .listen("framework-scratchwork-window-closed", () => setOpen(false))
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch(reportIgnoredFailure("Scratchwork window lifecycle"));
    return () => {
      disposed = true;
      stop?.();
    };
  }, []);

  const focusIfOpen = useCallback(async () => {
    if (!("__TAURI_INTERNALS__" in window)) return false;
    const focused = await focusScratchworkWindow().catch(() => false);
    if (focused) setOpen(true);
    return focused;
  }, []);

  const show = useCallback(async () => {
    if (!document || !("__TAURI_INTERNALS__" in window)) return;
    try {
      if (getActive()?.kind === "scratchwork") await commitActive();
      clearActive();
      closeDrawer();
      const exists = document.objects.some(
        (object) => object.kind === "block" && sameName(object.name, SCRATCHWORK)
      );
      if (!exists) {
        const failed = await run({
          type: "addBlock",
          name: SCRATCHWORK,
          ...insertPosition(),
        });
        if (failed) return;
      }
      await openScratchworkWindow();
      setOpen(true);
      setError(null);
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, [
    clearActive,
    closeDrawer,
    commitActive,
    document,
    getActive,
    insertPosition,
    run,
    setError,
  ]);

  return { open, show, focusIfOpen };
}
