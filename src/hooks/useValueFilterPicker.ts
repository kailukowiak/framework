import { useCallback, useEffect, useMemo, useState } from "react";
import { pickerRow, useSearchedRows, type PickerRow } from "./useDistinctValueSearch";
import type { StepDraft } from "../lib/pipelineSteps";
import type { Column } from "../lib/types";
import {
  readValueFilter,
  valueKey,
  writeValueFilter,
  type DistinctValue,
  type LoadDistinctValues,
  type ValueFilterState,
} from "../lib/valueFilter";

export type { PickerRow };

export type ValueFilterPicker = {
  status: "loading" | "ready" | "error";
  error?: string;
  existing: ValueFilterState;
  query: string;
  setQuery: (query: string) => void;
  /** Rows matching the query, in the source's order. */
  rows: PickerRow[];
  /**
   * When the list is a page of a longer one, how much of it is shown: of the
   * whole column with no query, of the matches with one. Null when the list
   * is complete.
   */
  shownOf: { shown: number; total: number } | null;
  isChecked: (row: PickerRow) => boolean;
  toggle: (row: PickerRow) => void;
  /** Check or uncheck exactly the rows the query leaves visible. */
  setVisible: (checked: boolean) => void;
  activeIndex: number;
  moveActive: (delta: number) => void;
  toggleActive: () => void;
  /**
   * Whether Apply means anything: a filter that keeps no value is not
   * writable (`is_in([])` is not a formula), and "keep nothing" is never what
   * a person who opened a checklist wants -- Excel disables OK there too.
   */
  canApply: boolean;
  /** The chain after committing, or null when committing changes nothing. */
  commit: () => StepDraft[] | null;
};

/**
 * The state behind the column-header value checklist.
 *
 * The values come from `load` -- the engine, over the whole column -- never
 * from the rows on screen, which for a paged import are a window onto the
 * data. When the engine reports more distinct values than it returned, the
 * list starts with nothing checked rather than everything: an all-checked
 * start would commit as "keep exactly the values I can see", silently
 * dropping every unseen one. Nothing checked, narrow by typing, check what
 * you mean is the only start that cannot lie about what it will keep.
 *
 * Typing into a truncated list asks the engine again (see
 * `useDistinctValueSearch`), since the value wanted is likely the one not on
 * the first page. Checks are kept by value, not by row, so a value checked
 * under one query stays checked -- and is written on Apply -- after a later
 * query no longer lists it.
 */
export function useValueFilterPicker({
  frameId,
  column,
  steps,
  load,
}: {
  frameId: string;
  column: Pick<Column, "id" | "name">;
  steps: StepDraft[];
  load: LoadDistinctValues;
}): ValueFilterPicker {
  const existing = useMemo(() => readValueFilter(steps, column), [steps, column]);
  const [loaded, setLoaded] = useState<{
    values: DistinctValue[];
    total: number;
  } | null>(null);
  const [error, setError] = useState<string>();
  const [checked, setChecked] = useState<Set<string> | null>(null);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoaded(null);
    setChecked(null);
    setError(undefined);
    load(frameId, column.id).then(
      (result) => {
        if (!cancelled) setLoaded(result);
      },
      (reason: unknown) => {
        if (!cancelled)
          setError(reason instanceof Error ? reason.message : String(reason));
      }
    );
    return () => {
      cancelled = true;
    };
    // The picker is opened for one column of one chain; re-reading the chain
    // as the person toggles must not reload from the engine.
  }, [frameId, column.id, load]);

  const allRows = useMemo<PickerRow[]>(() => {
    if (!loaded) return [];
    const rows = loaded.values.map(pickerRow);
    // A value the filter keeps but the capped list no longer shows must stay
    // visible and checked, or rewriting the step would silently drop it.
    if (existing.kind === "picker") {
      const seen = new Set(rows.map((row) => row.key));
      for (const value of existing.values)
        if (!seen.has(valueKey(value)))
          rows.unshift(pickerRow({ value }));
    }
    return rows;
  }, [loaded, existing]);

  const truncated = loaded !== null && loaded.total > loaded.values.length;
  const { rows, knownRows, shownOf, searchError } = useSearchedRows({
    frameId,
    columnId: column.id,
    load,
    loaded,
    allRows,
    query,
  });
  const initial = useMemo(() => {
    if (existing.kind === "picker") return new Set(existing.values.map(valueKey));
    return truncated ? new Set<string>() : new Set(allRows.map((row) => row.key));
  }, [existing, truncated, allRows]);
  const current = checked ?? initial;

  const kept = knownRows.filter((row) => current.has(row.key));
  const keptCount = kept.length;
  const clampedActive = Math.min(activeIndex, Math.max(rows.length - 1, 0));

  const update = useCallback(
    (change: (next: Set<string>) => void) => {
      const next = new Set(current);
      change(next);
      setChecked(next);
    },
    [current]
  );

  return {
    status: error || searchError ? "error" : loaded ? "ready" : "loading",
    error: error ?? searchError,
    existing,
    query,
    setQuery: (value) => {
      setQuery(value);
      setActiveIndex(0);
    },
    rows,
    shownOf,
    isChecked: (row) => current.has(row.key),
    toggle: (row) =>
      update((next) => {
        if (!next.delete(row.key)) next.add(row.key);
      }),
    setVisible: (value) =>
      update((next) => {
        for (const row of rows) {
          if (value) next.add(row.key);
          else next.delete(row.key);
        }
      }),
    activeIndex: clampedActive,
    moveActive: (delta) =>
      setActiveIndex(Math.min(Math.max(clampedActive + delta, 0), Math.max(rows.length - 1, 0))),
    toggleActive: () => {
      const row = rows[clampedActive];
      if (row)
        update((next) => {
          if (!next.delete(row.key)) next.add(row.key);
        });
    },
    canApply: loaded !== null && keptCount > 0,
    commit: () => {
      if (existing.kind === "foreign" || !loaded || keptCount === 0) return null;
      const everything = !truncated && kept.length === allRows.length;
      const next = writeValueFilter(
        steps,
        column,
        everything ? null : kept.map((row) => row.value)
      );
      return next === steps ? null : next;
    },
  };
}
