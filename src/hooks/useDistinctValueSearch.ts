import { useEffect, useMemo, useState } from "react";
import {
  valueKey,
  type DistinctValue,
  type DistinctValues,
  type LoadDistinctValues,
} from "../lib/valueFilter";

/** One checklist row: a value, its identity, and the text it is shown as. */
export type PickerRow = DistinctValue & { key: string; label: string };

export function pickerRow(entry: DistinctValue): PickerRow {
  const label = entry.value === null ? "(blank)" : String(entry.value);
  return { ...entry, key: valueKey(entry.value), label };
}

/** The engine's answer to one typed query. */
export type DistinctValueSearch = DistinctValues & { query: string };

/**
 * How long typing must pause before the engine is asked. A keystroke each is
 * a group-by over the whole column each; a pause this short is not felt, and
 * it turns "Zelda" into one question instead of five.
 */
const SEARCH_DELAY_MS = 150;

/**
 * Asks the engine for the values matching what the person typed, when the
 * list the checklist holds is only the first page of a longer one.
 *
 * Filtering that page locally would answer "no such value" about a name that
 * is simply further down, which is the defect this exists to fix. When the
 * whole list is already loaded (`enabled` false) nothing is asked: the local
 * filter is complete and costs no round trip.
 *
 * Responses can arrive out of order, so one is kept only while its query is
 * still the one typed; `latest` is therefore always about `query` or stale by
 * its own label, and the caller compares the two. Every value any answer has
 * carried is remembered in `seen`, because a value checked under one query
 * must still be written on Apply after the query changes and it drops out of
 * the list.
 */
export function useDistinctValueSearch({
  frameId,
  columnId,
  load,
  query,
  enabled,
}: {
  frameId: string;
  columnId: string;
  load: LoadDistinctValues;
  query: string;
  enabled: boolean;
}): { latest: DistinctValueSearch | null; seen: DistinctValue[]; error?: string } {
  const [state, setState] = useState<{
    latest: DistinctValueSearch | null;
    seen: DistinctValue[];
    error?: string;
  }>({ latest: null, seen: [] });

  useEffect(() => {
    if (!enabled || query === "") return;
    let current = true;
    const timer = setTimeout(() => {
      load(frameId, columnId, query).then(
        (found) => {
          if (current)
            setState((previous) => ({
              latest: { ...found, query },
              seen: [...previous.seen, ...found.values],
            }));
        },
        (reason: unknown) => {
          if (current)
            setState((previous) => ({
              ...previous,
              error: reason instanceof Error ? reason.message : String(reason),
            }));
        }
      );
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [frameId, columnId, load, query, enabled]);

  return state;
}

/**
 * The rows a checklist shows for what is typed, and every row it knows.
 *
 * With no query, or over a complete list, that is the first page filtered
 * locally. Over a truncated list a query is the engine's to answer; until
 * it does, and for what it cannot match (the blank group has no text), the
 * known rows that match locally stand in, so the list does not blank out
 * between keystrokes. `knownRows` -- first page first, then everything any
 * answer carried -- is what a check can name, and so what Apply can write.
 * `shownOf` is set only when the list on screen stops short of what there is.
 */
export function useSearchedRows({
  frameId,
  columnId,
  load,
  loaded,
  allRows,
  query,
}: {
  frameId: string;
  columnId: string;
  load: LoadDistinctValues;
  loaded: DistinctValues | null;
  allRows: PickerRow[];
  query: string;
}): {
  rows: PickerRow[];
  knownRows: PickerRow[];
  shownOf: { shown: number; total: number } | null;
  searchError?: string;
} {
  const truncated = loaded !== null && loaded.total > loaded.values.length;
  const needle = query.trim().toLowerCase();
  const search = useDistinctValueSearch({
    frameId,
    columnId,
    load,
    query: query.trim(),
    enabled: truncated,
  });
  const knownRows = useMemo(() => {
    const keys = new Set(allRows.map((row) => row.key));
    const rows = [...allRows];
    for (const row of search.seen.map(pickerRow))
      if (!keys.has(row.key)) {
        keys.add(row.key);
        rows.push(row);
      }
    return rows;
  }, [allRows, search.seen]);
  const answer =
    truncated && search.latest?.query.toLowerCase() === needle ? search.latest : null;
  const rows = useMemo(() => {
    if (!needle) return allRows;
    const local = knownRows.filter((row) => row.label.toLowerCase().includes(needle));
    if (!answer) return local;
    const listed = answer.values.map(pickerRow);
    const keys = new Set(listed.map((row) => row.key));
    return [...listed, ...local.filter((row) => !keys.has(row.key))];
  }, [allRows, knownRows, needle, answer]);
  const page = !needle ? loaded : answer;
  const shownOf =
    truncated && page && page.total > page.values.length
      ? { shown: page.values.length, total: page.total }
      : null;
  return { rows, knownRows, shownOf, searchError: search.error };
}
