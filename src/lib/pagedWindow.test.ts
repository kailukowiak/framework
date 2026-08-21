import { describe, expect, it } from "vitest";
import {
  PAGED_ROW_PAGE_SIZE,
  PagedRowCache,
  directionBetween,
  isStaleRequest,
  lastPageIndex,
  pageIndexForRow,
  pageRowOffset,
  pagedGenerationSignature,
  pagesForRowRange,
  planFlingFetch,
  planPageFetch,
} from "./pagedWindow";

describe("PAGED_ROW_PAGE_SIZE", () => {
  it("matches the core's get_frame_page request cap (limit.min(1000)), the largest single fetch the UI can ask for", () => {
    expect(PAGED_ROW_PAGE_SIZE).toBe(1000);
  });
});

describe("pageIndexForRow / pageRowOffset", () => {
  it("maps a row index to its page and back using an explicit page size", () => {
    expect(pageIndexForRow(0, 200)).toBe(0);
    expect(pageIndexForRow(199, 200)).toBe(0);
    expect(pageIndexForRow(200, 200)).toBe(1);
    expect(pageIndexForRow(1_180_000, 200)).toBe(5900);
    expect(pageRowOffset(1, 200)).toBe(200);
    expect(pageRowOffset(5900, 200)).toBe(1_180_000);
  });

  it("defaults to the production page size when none is given", () => {
    expect(pageIndexForRow(PAGED_ROW_PAGE_SIZE)).toBe(1);
    expect(pageIndexForRow(PAGED_ROW_PAGE_SIZE - 1)).toBe(0);
    expect(pageRowOffset(3)).toBe(3 * PAGED_ROW_PAGE_SIZE);
  });
});

describe("lastPageIndex", () => {
  it("handles an empty frame", () => {
    expect(lastPageIndex(0, 200)).toBe(0);
  });

  it("stays on page 0 for a frame that exactly fills one page", () => {
    expect(lastPageIndex(200, 200)).toBe(0);
  });

  it("rolls to the next page for one extra row", () => {
    expect(lastPageIndex(201, 200)).toBe(1);
  });

  it("computes the last page for a large frame", () => {
    expect(lastPageIndex(1_180_000, 200)).toBe(5899);
  });
});

describe("pagesForRowRange", () => {
  it("returns nothing for an empty or inverted range", () => {
    expect(pagesForRowRange(10, 10, 200)).toEqual([]);
    expect(pagesForRowRange(10, 5, 200)).toEqual([]);
  });

  it("returns a single page for a range inside one page", () => {
    expect(pagesForRowRange(5, 20, 200)).toEqual([0]);
  });

  it("does not pull in a neighboring page when the range lands exactly on a boundary", () => {
    expect(pagesForRowRange(0, 200, 200)).toEqual([0]);
    expect(pagesForRowRange(200, 400, 200)).toEqual([1]);
  });

  it("covers every page a range overlaps", () => {
    expect(pagesForRowRange(190, 410, 200)).toEqual([0, 1, 2]);
  });

  it("covers a range deep inside a huge frame", () => {
    expect(pagesForRowRange(1_179_990, 1_180_010, 200)).toEqual([5899, 5900]);
  });

  it("clamps a negative start to row 0", () => {
    expect(pagesForRowRange(-50, 10, 200)).toEqual([0]);
  });

  it("overfetches a full page at the production page size for a small viewport (~40 rows)", () => {
    // The viewport is a few dozen rows; a page covers 1000, so an ordinary
    // scroll inside a cached page needs no fetch at all.
    expect(pagesForRowRange(10_000, 10_040)).toEqual([10]);
  });
});

describe("pagedGenerationSignature", () => {
  it("is stable for identical input", () => {
    const input = {
      revision: "r3",
      sort: [{ columnId: "c1", descending: false }],
      filters: [{ columnId: "c2" }],
    };
    expect(pagedGenerationSignature(input)).toBe(
      pagedGenerationSignature({ ...input })
    );
  });

  it("changes when the document revision changes", () => {
    const base = { revision: "r1", sort: [], filters: [] };
    expect(pagedGenerationSignature(base)).not.toBe(
      pagedGenerationSignature({ ...base, revision: "r2" })
    );
  });

  it("changes when the sort keys change", () => {
    const base = {
      revision: "r1",
      sort: [{ columnId: "c1", descending: false }],
      filters: [],
    };
    const resorted = {
      revision: "r1",
      sort: [{ columnId: "c1", descending: true }],
      filters: [],
    };
    expect(pagedGenerationSignature(base)).not.toBe(pagedGenerationSignature(resorted));
  });

  it("changes when the filters change", () => {
    const base = { revision: "r1", sort: [], filters: [{ columnId: "c1" }] };
    const refiltered = { revision: "r1", sort: [], filters: [] };
    expect(pagedGenerationSignature(base)).not.toBe(
      pagedGenerationSignature(refiltered)
    );
  });
});

describe("PagedRowCache", () => {
  it("round-trips a page under the same generation", () => {
    const cache = new PagedRowCache<string[][]>();
    cache.set("view1", 0, "gen-a", [["1"], ["2"]]);
    expect(cache.get("view1", 0, "gen-a")).toEqual([["1"], ["2"]]);
    expect(cache.has("view1", 0, "gen-a")).toBe(true);
  });

  it("misses when the requested generation does not match what's stored", () => {
    const cache = new PagedRowCache<string[][]>();
    cache.set("view1", 0, "gen-a", [["1"]]);
    expect(cache.get("view1", 0, "gen-b")).toBeUndefined();
    expect(cache.has("view1", 0, "gen-b")).toBe(false);
  });

  it("a sort/filter/revision change invalidates every previously cached page for that view", () => {
    const cache = new PagedRowCache<string[][]>();
    cache.set("view1", 0, "gen-a", [["1"]]);
    cache.set("view1", 1, "gen-a", [["2"]]);
    // Simulate a sort click: the caller now reads under a new signature.
    expect(cache.get("view1", 0, "gen-b")).toBeUndefined();
    expect(cache.get("view1", 1, "gen-b")).toBeUndefined();
  });

  it("keeps pages for different views independent", () => {
    const cache = new PagedRowCache<string[][]>();
    cache.set("view1", 0, "gen-a", [["1"]]);
    cache.set("view2", 0, "gen-a", [["2"]]);
    expect(cache.get("view1", 0, "gen-a")).toEqual([["1"]]);
    expect(cache.get("view2", 0, "gen-a")).toEqual([["2"]]);
  });

  it("evicts the least-recently-used page once the cache exceeds its max size", () => {
    const cache = new PagedRowCache<number>(2);
    cache.set("view1", 0, "gen-a", 0);
    cache.set("view1", 1, "gen-a", 1);
    cache.set("view1", 2, "gen-a", 2);
    expect(cache.size()).toBe(2);
    expect(cache.has("view1", 0, "gen-a")).toBe(false);
    expect(cache.has("view1", 1, "gen-a")).toBe(true);
    expect(cache.has("view1", 2, "gen-a")).toBe(true);
  });

  it("touching a page on read protects it from eviction", () => {
    const cache = new PagedRowCache<number>(2);
    cache.set("view1", 0, "gen-a", 0);
    cache.set("view1", 1, "gen-a", 1);
    // Reading page 0 makes it the most-recently used.
    cache.get("view1", 0, "gen-a");
    cache.set("view1", 2, "gen-a", 2);
    expect(cache.has("view1", 0, "gen-a")).toBe(true);
    expect(cache.has("view1", 1, "gen-a")).toBe(false);
  });

  it("clear empties the cache", () => {
    const cache = new PagedRowCache<number>();
    cache.set("view1", 0, "gen-a", 0);
    cache.clear();
    expect(cache.size()).toBe(0);
  });
});

describe("directionBetween", () => {
  it("is none when there is no previous position", () => {
    expect(directionBetween(null, 100)).toBe("none");
  });

  it("is none when the position hasn't moved", () => {
    expect(directionBetween(50, 50)).toBe("none");
  });

  it("is forward when scrolling to a larger row index", () => {
    expect(directionBetween(50, 100)).toBe("forward");
  });

  it("is backward when scrolling to a smaller row index", () => {
    expect(directionBetween(100, 50)).toBe("backward");
  });
});

describe("planFlingFetch", () => {
  const alwaysMiss = () => false;

  it("requests the pages covering the visible range plus one page of forward prefetch", () => {
    const pages = planFlingFetch({
      visibleStartRow: 190,
      visibleEndRow: 410,
      totalRows: 10_000,
      direction: "forward",
      pageSize: 200,
      isCached: alwaysMiss,
    });
    expect(pages).toEqual([0, 1, 2, 3]);
  });

  it("prefetches backward instead when scrolling up", () => {
    const pages = planFlingFetch({
      visibleStartRow: 250,
      visibleEndRow: 460,
      totalRows: 10_000,
      direction: "backward",
      pageSize: 200,
      isCached: alwaysMiss,
    });
    expect(pages).toEqual([0, 1, 2]);
  });

  it("adds no prefetch page when direction is none", () => {
    const pages = planFlingFetch({
      visibleStartRow: 190,
      visibleEndRow: 410,
      totalRows: 10_000,
      direction: "none",
      pageSize: 200,
      isCached: alwaysMiss,
    });
    expect(pages).toEqual([0, 1, 2]);
  });

  it("does not prefetch past the last page of the frame", () => {
    const totalRows = 250; // pages 0 and 1 only at a 200-row page size
    const pages = planFlingFetch({
      visibleStartRow: 190,
      visibleEndRow: 250,
      totalRows,
      direction: "forward",
      pageSize: 200,
      isCached: alwaysMiss,
    });
    expect(pages).toEqual([0, 1]);
  });

  it("does not prefetch below page 0 when scrolling backward at the top", () => {
    const pages = planFlingFetch({
      visibleStartRow: 0,
      visibleEndRow: 30,
      totalRows: 10_000,
      direction: "backward",
      pageSize: 200,
      isCached: alwaysMiss,
    });
    expect(pages).toEqual([0]);
  });

  it("drops pages that are already cached, keeping only the gap", () => {
    const pages = planFlingFetch({
      visibleStartRow: 0,
      visibleEndRow: 410,
      totalRows: 10_000,
      direction: "forward",
      pageSize: 200,
      isCached: (pageIndex) => pageIndex === 0 || pageIndex === 1,
    });
    expect(pages).toEqual([2, 3]);
  });

  it("at the production page size, an ordinary scroll within a ~40-row viewport stays inside one cached page (no fetch, no skeleton)", () => {
    const pages = planFlingFetch({
      visibleStartRow: 12_000,
      visibleEndRow: 12_040,
      totalRows: 1_180_000,
      direction: "forward",
      isCached: (pageIndex) => pageIndex === 12 || pageIndex === 13,
    });
    expect(pages).toEqual([]);
  });

  it("this is the fling-coalescing contract: only the settled target range is planned, never intermediate frames", () => {
    // Simulate a fling that passes through pages 5..40 but settles on page 41-42;
    // a caller that only calls planFlingFetch once, with the settled range, must
    // never request the pages it merely scrolled past.
    const settledStart = 41 * PAGED_ROW_PAGE_SIZE;
    const settledEnd = settledStart + 300;
    const pages = planFlingFetch({
      visibleStartRow: settledStart,
      visibleEndRow: settledEnd,
      totalRows: 1_000_000,
      direction: "forward",
      isCached: alwaysMiss,
    });
    expect(pages).not.toContain(5);
    expect(pages).not.toContain(20);
    expect(pages[0]).toBe(41);
  });
});

describe("planPageFetch", () => {
  const alwaysMissing = () => false;

  it("reads page 0 when the row count is unknown, even though the visible range is empty", () => {
    // A derived frame reports no count, so the virtualizer has no height and
    // asks for no rows. Without this the frame would sit empty forever: no
    // fetch, so no count, so no height, so no fetch.
    expect(
      planPageFetch({
        visibleStartRow: 0,
        visibleEndRow: 0,
        knownTotalRows: null,
        direction: "forward",
        isCached: alwaysMissing,
      })
    ).toEqual([0]);
  });

  it("does not re-request the bootstrap page while it is already pending", () => {
    expect(
      planPageFetch({
        visibleStartRow: 0,
        visibleEndRow: 0,
        knownTotalRows: null,
        direction: "forward",
        isCached: (page) => page === 0,
      })
    ).toEqual([]);
  });

  it("plans by visible range once the count is known", () => {
    expect(
      planPageFetch({
        visibleStartRow: 0,
        visibleEndRow: 0,
        knownTotalRows: 0,
        direction: "forward",
        isCached: alwaysMissing,
      })
    ).toEqual([]);

    const start = 3 * PAGED_ROW_PAGE_SIZE;
    expect(
      planPageFetch({
        visibleStartRow: start,
        visibleEndRow: start + 10,
        knownTotalRows: 100_000,
        direction: "forward",
        isCached: alwaysMissing,
      })
    ).toEqual([3, 4]);
  });
});

describe("isStaleRequest", () => {
  it("is not stale when the generation hasn't advanced", () => {
    expect(isStaleRequest(3, 3)).toBe(false);
  });

  it("is stale once the generation has moved on", () => {
    expect(isStaleRequest(3, 4)).toBe(true);
  });
});
