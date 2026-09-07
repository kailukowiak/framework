import { useEffect, useRef, type RefObject } from "react";
import { scrollLeftToRevealColumn } from "../lib/gridNavigation";
import type { Column } from "../lib/types";

/**
 * Scroll a card's grid sideways when a column is added to it.
 *
 * *Add calculated column* appends its column at the right-hand end of the
 * frame, and on any table wider than its card that end is off screen: the
 * chain opened a formula editor for a column the person could not see, so the
 * first evidence that anything had happened was the Wrangle panel. Nothing
 * was broken, and it read as though nothing had worked.
 *
 * Only genuinely new columns count, and only after the first render — a card
 * arriving with six columns has not just gained six, and should sit where its
 * saved scroll left it. The column is found by its own `data-column-id`
 * rather than by index, so a hidden or reordered column cannot make this
 * scroll to the wrong header.
 *
 * `enabled` is false for a fields-as-rows card, where a column is a row and
 * the thing to move is the vertical scroll — that card windows its records
 * itself, and adding a horizontal jump to it would fight its own paging.
 */
export function useRevealNewColumn(
  columns: readonly Column[],
  scrollRef: RefObject<HTMLElement | null>,
  enabled = true
) {
  const known = useRef<Set<string> | null>(null);
  useEffect(() => {
    const ids = columns.map((column) => column.id);
    const previous = known.current;
    known.current = new Set(ids);
    if (!previous) return;
    const added = ids.filter((id) => !previous.has(id));
    if (added.length === 0 || !enabled) return;
    const element = scrollRef.current;
    if (!element) return;
    // After the row that carries the new column has actually been laid out:
    // the header exists in the DOM only once React has painted this change.
    const frame = requestAnimationFrame(() => {
      const header = element.querySelector<HTMLElement>(
        `[data-column-id="${CSS.escape(added[added.length - 1])}"]`
      );
      if (!header) return;
      const left = scrollLeftToRevealColumn(
        header.offsetLeft,
        header.offsetWidth,
        element.scrollLeft,
        element.clientWidth
      );
      if (left !== null) element.scrollLeft = left;
    });
    return () => cancelAnimationFrame(frame);
  }, [columns, enabled, scrollRef]);
}
