import type { FrameStyleMatches } from "../FrameGrid";
import { pageIndexForRow, pageRowOffset, type PagedRowCache } from "./pagedWindow";
import type { Column, FrameStyleMatch, Row } from "./types";

/**
 * One cached page: the values as they came back, and what the rules made of
 * each of those rows. Cached together because they are one read — a page
 * whose styles were fetched separately would draw the old colors on the new
 * rows for as long as the second request was in flight.
 */
export type PagedRows = {
  values: string[][];
  rowIds: string[];
  styleMatches: FrameStyleMatch[][];
};

/**
 * The rendered window for the records-as-rows paged path: exactly the rows
 * in `[start, end)`, built from whatever the cache currently has under the
 * live generation. Missing rows render as fixed-height placeholders (empty
 * cells, flagged via `placeholderOffsets`, which counts from `start`) so
 * scroll geometry never jumps while their page is in flight.
 */
export function pagedVisibleWindow(
  cache: PagedRowCache<PagedRows>,
  frame: { id: string; columns: Column[] },
  signature: string,
  start: number,
  end: number
) {
  const rows: Row[] = [];
  const placeholderOffsets = new Set<number>();
  const styleMatches: FrameStyleMatches = {};
  for (let rowIndex = start; rowIndex < end; rowIndex += 1) {
    const pageIndex = pageIndexForRow(rowIndex);
    const cachedPage = cache.get(frame.id, pageIndex, signature);
    const offsetInPage = rowIndex - pageRowOffset(pageIndex);
    const raw = cachedPage?.values[offsetInPage];
    if (!raw) placeholderOffsets.add(rowIndex - start);
    // Literal frames keep stable stored row ids through a row-preserving
    // Wrangle chain. Imported and derived pages have no such id and keep
    // the positional spelling they have always used.
    const id = cachedPage?.rowIds[offsetInPage] ?? `source:${frame.id}:${rowIndex}`;
    // The rules were run over the page these values came from, so the
    // answers arrive with them rather than being recomputed here -- a
    // formula is the core's to evaluate, on this path as on every other.
    const matched = cachedPage?.styleMatches[offsetInPage];
    if (matched?.length) styleMatches[id] = matched;
    rows.push({
      id,
      cells: Object.fromEntries(
        frame.columns.map((column, columnIndex) => [
          column.id,
          { raw: raw?.[columnIndex] ?? "", overrideFormula: null },
        ])
      ),
    });
  }
  return { rows, placeholderOffsets, styleMatches };
}
