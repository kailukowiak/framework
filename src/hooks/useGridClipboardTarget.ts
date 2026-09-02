import { useEffect } from "react";
import {
  focusGridClipboardTarget,
  isGridClipboardTarget,
} from "../lib/gridClipboardTarget";

/** Re-arms the native target when editing returns to grid navigation. */
export function useGridClipboardTarget(active: boolean) {
  useEffect(() => {
    if (active && !isOrdinaryTextEntry(document.activeElement))
      focusGridClipboardTarget();
  }, [active]);
}

function isOrdinaryTextEntry(value: Element | null): boolean {
  return (
    value instanceof HTMLInputElement ||
    (value instanceof HTMLTextAreaElement && !isGridClipboardTarget(value)) ||
    value instanceof HTMLSelectElement ||
    (value instanceof HTMLElement && value.isContentEditable)
  );
}
