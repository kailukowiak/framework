// Pure page-window math for scroll-driven fetching over `get_frame_page`.
//
// The virtualizer (frameVirtualization.ts) reports a visible *logical* row
// range for a paged frame that may have well over a million rows. This
// module turns that range into the fixed-size pages that need to be
// fetched, keeps a small LRU cache of already-fetched pages, and decides
// which pages a settled fling should request (plus one page of prefetch in
// the scroll direction) so a rapid scrollbar drag does not queue a fetch
// per intermediate frame.
//
// Everything here is synchronous and side-effect free; the React wiring in
// App.tsx owns the debounce timer, the actual `getFramePage` calls, and
// translating cache contents into rendered rows.

// Fetch cost for `get_frame_page` is dominated by the round trip and Tauri
// IPC serialization, not by row count, so pages are sized to scroll
// distance rather than to the ~40-row viewport: overfetching means an
// ordinary scroll almost never needs a network round trip, and skeleton
// rows should only appear on a genuine fling across a large distance. 1000
// is also the core's hardcoded per-request cap (`get_frame_page` clamps
// `limit` to `.min(1000)`), so this is the largest single fetch the UI can
// ask for. A ~40-page LRU keeps at most ~40k resident rows (a few MB) while
// covering many screens worth of prefetch in either scroll direction.
export const PAGED_ROW_PAGE_SIZE = 1000;
export const PAGED_ROW_CACHE_MAX_PAGES = 40;

export function pageIndexForRow(
  rowIndex: number,
  pageSize = PAGED_ROW_PAGE_SIZE
): number {
  return Math.floor(rowIndex / pageSize);
}

export function pageRowOffset(
  pageIndex: number,
  pageSize = PAGED_ROW_PAGE_SIZE
): number {
  return pageIndex * pageSize;
}

export function lastPageIndex(
  totalRows: number,
  pageSize = PAGED_ROW_PAGE_SIZE
): number {
  return totalRows <= 0 ? 0 : Math.floor((totalRows - 1) / pageSize);
}

/**
 * The sorted list of page indices that fully cover the half-open logical
 * row range [start, end). Boundaries land exactly on page edges without
 * pulling in a neighboring page; an empty or inverted range yields no pages.
 */
export function pagesForRowRange(
  start: number,
  end: number,
  pageSize = PAGED_ROW_PAGE_SIZE
): number[] {
  if (end <= start) return [];
  const firstPage = pageIndexForRow(Math.max(0, start), pageSize);
  const lastPage = pageIndexForRow(Math.max(0, end - 1), pageSize);
  const pages: number[] = [];
  for (let page = firstPage; page <= lastPage; page += 1) pages.push(page);
  return pages;
}

export interface PagedGenerationInput {
  /**
   * The frame's lineage hash: moves when anything that decides its rows
   * moves, and stays put for every edit elsewhere in the document.
   */
  revision: string;
  sort: ReadonlyArray<{ columnId: string; descending: boolean }>;
  /** Anything JSON-serializable that identifies the view's active filters. */
  filters: unknown;
  /**
   * The frame's conditional-formatting rules. Not part of its lineage --
   * they change nothing about which rows exist or what they hold -- but a
   * page carries what the rules made of its rows, so a page fetched under
   * the old rules is drawn in the old colors.
   */
  styleRules?: unknown;
}

/**
 * Stable signature for the (revision, sort, filter, rules) tuple a cached
 * page depends on. A sort click, a filter change, an edited rule, or an
 * upstream edit produces a different signature, so previously cached pages
 * simply stop matching -- no explicit sweep is required, they just become
 * unreachable and get reclaimed by ordinary LRU pressure once new pages are
 * written.
 */
export function pagedGenerationSignature(input: PagedGenerationInput): string {
  return JSON.stringify([input.revision, input.sort, input.filters, input.styleRules ?? []]);
}

interface CachedPage<T> {
  rows: T;
  generation: string;
}

/**
 * LRU cache of fetched pages, keyed by (viewId, pageIndex) and scoped to a
 * generation signature. A read or write under a generation that doesn't
 * match what's stored is a miss -- stale pages are never returned.
 */
export class PagedRowCache<T> {
  private entries = new Map<string, CachedPage<T>>();

  constructor(private readonly maxPages: number = PAGED_ROW_CACHE_MAX_PAGES) {}

  private key(viewId: string, pageIndex: number): string {
    return `${viewId}::${pageIndex}`;
  }

  get(viewId: string, pageIndex: number, generation: string): T | undefined {
    const key = this.key(viewId, pageIndex);
    const entry = this.entries.get(key);
    if (!entry || entry.generation !== generation) return undefined;
    // Touch for LRU recency: re-insert so it sorts after fresher entries.
    this.entries.delete(key);
    this.entries.set(key, entry);
    return entry.rows;
  }

  has(viewId: string, pageIndex: number, generation: string): boolean {
    return this.get(viewId, pageIndex, generation) !== undefined;
  }

  set(viewId: string, pageIndex: number, generation: string, rows: T): void {
    const key = this.key(viewId, pageIndex);
    this.entries.delete(key);
    this.entries.set(key, { rows, generation });
    while (this.entries.size > this.maxPages) {
      const oldest = this.entries.keys().next().value;
      if (oldest === undefined) break;
      this.entries.delete(oldest);
    }
  }

  size(): number {
    return this.entries.size;
  }

  clear(): void {
    this.entries.clear();
  }
}

export type ScrollDirection = "forward" | "backward" | "none";

export function directionBetween(
  previousStartRow: number | null,
  nextStartRow: number
): ScrollDirection {
  if (previousStartRow === null || nextStartRow === previousStartRow) return "none";
  return nextStartRow > previousStartRow ? "forward" : "backward";
}

export interface FlingFetchInput {
  visibleStartRow: number;
  /** Exclusive. */
  visibleEndRow: number;
  totalRows: number;
  direction: ScrollDirection;
  pageSize?: number;
  isCached: (pageIndex: number) => boolean;
}

/**
 * Given the settled visible row range (i.e. the target of a fling, after
 * the caller has debounced rapid intermediate scroll positions), returns
 * the pages that need to be requested: every page covering the visible
 * range that isn't already cached, plus one page of prefetch in the scroll
 * direction. Callers must only invoke this with the debounced target --
 * calling it once per scroll event would defeat the coalescing and queue a
 * fetch per frame during a fling.
 */
export function planFlingFetch(input: FlingFetchInput): number[] {
  const pageSize = input.pageSize ?? PAGED_ROW_PAGE_SIZE;
  const pages = pagesForRowRange(input.visibleStartRow, input.visibleEndRow, pageSize);
  const maxPage = lastPageIndex(input.totalRows, pageSize);
  if (pages.length > 0) {
    if (input.direction === "forward") {
      const next = pages[pages.length - 1] + 1;
      if (next <= maxPage && !pages.includes(next)) pages.push(next);
    } else if (input.direction === "backward") {
      const prev = pages[0] - 1;
      if (prev >= 0 && !pages.includes(prev)) pages.unshift(prev);
    }
  }
  return pages.filter((page) => !input.isCached(page));
}

/**
 * The pages to request when the total row count may not be known yet.
 *
 * A derived frame reports no count -- producing one would mean running its
 * whole query -- so it arrives with `knownTotalRows` null. The virtualizer
 * then has no height to work with and reports an empty visible range, which
 * means {@link planFlingFetch} would ask for nothing, the first page would
 * never be fetched, and the count would never be learned: the frame renders
 * empty forever. Reading page 0 breaks that circle, and every later fetch
 * plans against the count that page reports.
 */
export function planPageFetch(
  input: Omit<FlingFetchInput, "totalRows"> & { knownTotalRows: number | null }
): number[] {
  if (input.knownTotalRows === null) {
    return input.isCached(0) ? [] : [0];
  }
  return planFlingFetch({ ...input, totalRows: input.knownTotalRows });
}

/**
 * Generation-counter guard: a paged fetch captures the request generation
 * at the moment it starts. If the counter has since advanced -- frame or
 * view switched, sort/filter/revision changed, or a newer fetch superseded
 * it -- the response must be discarded instead of applied, no matter how
 * late it lands.
 */
export function isStaleRequest(
  requestGeneration: number,
  currentGeneration: number
): boolean {
  return requestGeneration !== currentGeneration;
}
