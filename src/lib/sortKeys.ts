import type { Id, SortKey } from "./types";

/**
 * How a header click folds a column into the frame's existing sort keys.
 * "accumulate" is what a plain click does: keys pile up in click order, so
 * clicking Region then Date sorts by Region with Date breaking ties, and
 * each column's own ordinal shows its place. "only" is the shift-click
 * escape hatch that drops the rest and sorts by this column alone.
 */
export type SortClickMode = "accumulate" | "only";

/**
 * Cycles one column through none → asc → desc → none on each click,
 * appending it as the next key (or, in "only" mode, as the sole key).
 */
export function nextSortKeys(
  current: SortKey[],
  columnId: Id,
  mode: SortClickMode = "accumulate"
): SortKey[] {
  const existing = current.find((key) => key.columnId === columnId);
  if (mode === "only") {
    const alone = current.length === 1 && existing;
    if (!existing || !alone) return [{ columnId, descending: false }];
    if (!existing.descending) return [{ columnId, descending: true }];
    return [];
  }
  if (!existing) return [...current, { columnId, descending: false }];
  if (!existing.descending) {
    return current.map((key) =>
      key.columnId === columnId ? { ...key, descending: true } : key
    );
  }
  return current.filter((key) => key.columnId !== columnId);
}

/**
 * Appends a key for the first column not already sorted on, so the sidebar's
 * "Add" never produces a duplicate key the core would reject. Returns the
 * list unchanged when every column is already a key.
 */
export function appendSortKey(
  current: SortKey[],
  columnIds: readonly Id[]
): SortKey[] {
  const used = new Set(current.map((key) => key.columnId));
  const candidate = columnIds.find((columnId) => !used.has(columnId));
  return candidate ? [...current, { columnId: candidate, descending: false }] : current;
}

/**
 * Repoints the key at `index` at another column. Picking a column that is
 * already a key elsewhere would duplicate it, so the other key is dropped
 * and this position wins.
 */
export function setSortKeyColumn(
  current: SortKey[],
  index: number,
  columnId: Id
): SortKey[] {
  if (index < 0 || index >= current.length) return current;
  return current
    .map((key, position) => (position === index ? { ...key, columnId } : key))
    .filter((key, position) => position === index || key.columnId !== columnId);
}

export function setSortKeyDirection(
  current: SortKey[],
  index: number,
  descending: boolean
): SortKey[] {
  if (index < 0 || index >= current.length) return current;
  return current.map((key, position) =>
    position === index ? { ...key, descending } : key
  );
}

/** Moves a key one position earlier or later; key order is sort precedence. */
export function moveSortKey(
  current: SortKey[],
  index: number,
  offset: -1 | 1
): SortKey[] {
  const target = index + offset;
  if (index < 0 || index >= current.length || target < 0 || target >= current.length)
    return current;
  const next = [...current];
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

/**
 * Moves the key at `from` to position `to` -- the position it is dropped
 * on, which is the one it ends up occupying. Dragging key 1 onto key 2
 * therefore swaps them, the way a dragged list row is expected to behave.
 * Positions past the end clamp to last, so dropping on the list's empty
 * space sends a key to the bottom.
 */
export function reorderSortKeys(
  current: SortKey[],
  from: number,
  to: number
): SortKey[] {
  if (from < 0 || from >= current.length || to < 0) return current;
  const target = Math.min(to, current.length - 1);
  if (target === from) return current;
  const next = [...current];
  const [moved] = next.splice(from, 1);
  next.splice(target, 0, moved);
  return next;
}

export function removeSortKey(
  current: SortKey[],
  index: number
): SortKey[] {
  if (index < 0 || index >= current.length) return current;
  return current.filter((_, position) => position !== index);
}
