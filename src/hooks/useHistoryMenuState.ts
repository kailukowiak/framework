import { useEffect } from "react";
import { setHistoryMenuState } from "../lib/api";
import { hasNativeMenu } from "../lib/applicationShortcuts";
import { reportIgnoredFailure } from "../lib/errorReporting";
import { ownsTextHistory } from "../useApplicationMenu";

/** Both fresh documents and history edits must update native menu enablement. */
export function useHistoryMenuState(canUndo?: boolean, canRedo?: boolean) {
  useEffect(() => {
    if (!hasNativeMenu() || canUndo === undefined || canRedo === undefined) return;
    const sync = () => {
      const textOwnsHistory = ownsTextHistory(document.activeElement);
      void setHistoryMenuState(
        canUndo || textOwnsHistory,
        canRedo || textOwnsHistory
      ).catch(reportIgnoredFailure("history menu state"));
    };
    // The menu item itself has to be enabled before its accelerator can emit
    // anything. A freshly opened workbook can have no document history while
    // an existing formula still has local keystrokes to undo, so focus is a
    // second source of enablement alongside the store's canUndo/canRedo.
    const afterFocusLeaves = () => queueMicrotask(sync);
    sync();
    window.addEventListener("focusin", sync);
    window.addEventListener("focusout", afterFocusLeaves);
    return () => {
      window.removeEventListener("focusin", sync);
      window.removeEventListener("focusout", afterFocusLeaves);
    };
  }, [canUndo, canRedo]);
}
