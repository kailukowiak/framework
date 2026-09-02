import { useLayoutEffect, useState, type RefObject } from "react";
import { measureGridColumnWidths } from "../lib/gridColumnWidths";

/**
 * The rendered widths of the grid table inside `scrollRef`, kept current
 * while `enabled`.
 *
 * The summary drawer is a second table under the grid, and its columns line
 * up with the grid's only if they are told the grid's widths. They are
 * measured rather than recomputed: whatever sizes the grid — content, card
 * width, zoom — sizes the statistics identically, and a header cell changing
 * width re-measures.
 */
export function useGridColumnWidths(
  scrollRef: RefObject<HTMLDivElement | null>,
  enabled: boolean,
  columnCount: number
): number[] | null {
  const [widths, setWidths] = useState<number[] | null>(null);
  useLayoutEffect(() => {
    if (!enabled) return;
    const table = scrollRef.current?.querySelector("table");
    if (!table) return;
    const measure = () =>
      setWidths((current) => {
        const next = measureGridColumnWidths(table, columnCount + 2);
        return sameWidths(current, next) ? current : next;
      });
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(table);
    for (const cell of table.querySelectorAll("thead tr:first-child > th"))
      observer.observe(cell);
    return () => observer.disconnect();
  }, [enabled, scrollRef, columnCount]);
  return enabled ? widths : null;
}

function sameWidths(current: number[] | null, next: number[] | null): boolean {
  if (current === next) return true;
  if (!current || !next || current.length !== next.length) return false;
  return current.every((width, index) => Math.abs(width - next[index]) < 0.5);
}
