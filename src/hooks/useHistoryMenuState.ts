import { useEffect } from "react";
import { setHistoryMenuState } from "../lib/api";
import { hasNativeMenu } from "../lib/applicationShortcuts";
import { reportIgnoredFailure } from "../lib/errorReporting";

/** Both fresh documents and history edits must update native menu enablement. */
export function useHistoryMenuState(canUndo?: boolean, canRedo?: boolean) {
  useEffect(() => {
    if (!hasNativeMenu() || canUndo === undefined || canRedo === undefined) return;
    void setHistoryMenuState(canUndo, canRedo).catch(reportIgnoredFailure("history menu state"));
  }, [canUndo, canRedo]);
}
