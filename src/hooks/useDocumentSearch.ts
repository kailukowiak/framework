/**
 * Find's results, in two halves.
 *
 * The half the document already holds is computed on the spot, so the palette
 * fills in on the keystroke. The half only the engine can answer — the rows of
 * a frame read through pages — arrives afterwards and is merged into the same
 * groups, so a paged frame's matches appear under its name beside everything
 * else rather than in a second list somewhere below.
 *
 * The engine half is debounced and every in-flight answer is dropped when the
 * query moves on. A search is a scan of a whole column; running one per
 * keystroke and keeping whichever landed last would both cost the scan and
 * show the wrong answer.
 */

import { useEffect, useMemo, useState } from "react";
import { searchFrameRows } from "../lib/api";
import {
  MAX_FIND_HITS,
  groupFindHits,
  searchDocument,
  type FindGroup,
  type FindHit,
} from "../lib/documentSearch";
import type { DocumentView } from "../lib/types";

/** Long enough that a typed word is one scan, short enough to feel immediate. */
export const SEARCH_DEBOUNCE_MS = 150;

/** Per frame, so one enormous import cannot crowd out every other frame. */
const PAGED_HIT_LIMIT = 25;

export type DocumentSearchResult = {
  hits: FindHit[];
  groups: FindGroup[];
  /** An engine scan is in flight; the groups shown are the in-memory half. */
  searching: boolean;
  error: string | null;
};

export function useDocumentSearch(
  document: DocumentView | null,
  query: string
): DocumentSearchResult {
  const localHits = useMemo(
    () => (document ? searchDocument(document, document, query) : []),
    [document, query]
  );
  const pagedFrameIds = useMemo(
    () =>
      document
        ? document.objects
            .filter(
              (object) =>
                object.kind === "frame" && document.computedFrames[object.id]?.paged
            )
            .map((object) => object.id)
        : [],
    [document]
  );

  const [pagedHits, setPagedHits] = useState<FindHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const frameKey = pagedFrameIds.join(",");

  useEffect(() => {
    const needle = query.trim();
    setPagedHits([]);
    setError(null);
    if (!document || !needle || pagedFrameIds.length === 0) {
      setSearching(false);
      return;
    }
    let live = true;
    setSearching(true);
    const timer = window.setTimeout(() => {
      void Promise.all(
        pagedFrameIds.map((frameId) =>
          searchFrameRows(frameId, needle, PAGED_HIT_LIMIT).then((rows) =>
            rows.map((row, index): FindHit => ({
              id: `cell:paged:${frameId}:${index}`,
              kind: "cell",
              objectId: frameId,
              viewId: viewIdFor(document, frameId),
              columnId: row.columnId,
              rowId: row.rowId ?? undefined,
              rowIndex: Number(row.rowIndex),
              label: columnName(document, frameId, row.columnId),
              snippet: row.snippet,
            }))
          )
        )
      )
        .then((answers) => {
          if (!live) return;
          setPagedHits(answers.flat());
        })
        .catch((reason) => {
          if (live) setError(String(reason).replace(/^Error:\s*/, ""));
        })
        .finally(() => {
          if (live) setSearching(false);
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
    // frameKey stands in for pagedFrameIds, whose identity changes with every
    // document view even when the same frames are paged.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [document, frameKey, query]);

  return useMemo(() => {
    // Merged before grouping so a paged frame's rows land under that frame's
    // own heading rather than after every other object.
    const hits = [...localHits, ...pagedHits].slice(0, MAX_FIND_HITS);
    return {
      hits,
      groups: document ? groupFindHits(document, hits) : [],
      searching,
      error,
    };
  }, [document, error, localHits, pagedHits, searching]);
}

function viewIdFor(document: DocumentView, objectId: string) {
  return (
    document.views.find((view) => view.objectId === objectId)?.id ??
    document.views.find((view) => view.tabObjectIds?.includes(objectId))?.id
  );
}

function columnName(document: DocumentView, frameId: string, columnId: string) {
  const frame = document.objects.find(
    (object) => object.kind === "frame" && object.id === frameId
  );
  if (frame?.kind !== "frame") return "Column";
  return frame.columns.find((column) => column.id === columnId)?.name ?? "Column";
}
