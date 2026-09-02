// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SEARCH_DEBOUNCE_MS, useDocumentSearch } from "./useDocumentSearch";
import type { DocumentView } from "../lib/types";

const searchFrameRows = vi.fn();
vi.mock("../lib/api", () => ({
  searchFrameRows: (...args: unknown[]) => searchFrameRows(...args),
}));

/** One paged frame (rows only the engine can see) and one that is not. */
const view = (): DocumentView =>
  ({
    objects: [
      {
        kind: "frame",
        id: "ledger",
        name: "Ledger",
        columns: [{ id: "vendor", name: "Vendor" }],
        rows: [],
      },
      {
        kind: "frame",
        id: "rates",
        name: "Rates",
        columns: [{ id: "code", name: "Code" }],
        rows: [{ id: "row-1" }],
      },
    ],
    views: [{ id: "view-ledger", objectId: "ledger" }],
    computedFrames: {
      ledger: { paged: true, formulas: {}, rows: {} },
      rates: { formulas: {}, rows: { "row-1": { code: { display: "Fenwick" } } } },
    },
    computedBlocks: {},
  }) as unknown as DocumentView;

describe("useDocumentSearch", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    searchFrameRows.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  it("asks the engine only for paged frames, and merges the answer into their group", async () => {
    searchFrameRows.mockResolvedValue([
      { rowIndex: 8123, rowId: "source:ledger:8123", columnId: "vendor", snippet: "Fenwick Ltd" },
    ]);
    const document = view();
    const { result } = renderHook(() => useDocumentSearch(document, "fenwick"));

    // The in-memory half is there before anything is asked of the engine.
    expect(result.current.hits.map((hit) => hit.objectId)).toEqual(["rates"]);
    expect(searchFrameRows).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    });
    expect(result.current.searching).toBe(false);

    expect(searchFrameRows).toHaveBeenCalledExactlyOnceWith("ledger", "fenwick", 25);
    expect(result.current.groups.map((group) => group.name)).toEqual(["Rates", "Ledger"]);
    expect(result.current.groups[1].hits[0]).toMatchObject({
      kind: "cell",
      objectId: "ledger",
      viewId: "view-ledger",
      columnId: "vendor",
      rowId: "source:ledger:8123",
      rowIndex: 8123,
      label: "Vendor",
      snippet: "Fenwick Ltd",
    });
  });

  it("debounces, so a typed word costs one scan rather than one per keystroke", async () => {
    searchFrameRows.mockResolvedValue([]);
    const document = view();
    const { rerender } = renderHook(({ query }) => useDocumentSearch(document, query), {
      initialProps: { query: "f" },
    });
    for (const query of ["fe", "fen", "fenw"]) {
      act(() => {
        vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS - 40);
      });
      rerender({ query });
    }
    expect(searchFrameRows).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    });
    expect(searchFrameRows).toHaveBeenCalledExactlyOnceWith("ledger", "fenw", 25);
  });

  it("drops an answer that lands after the query moved on", async () => {
    let settleFirst: (rows: unknown[]) => void = () => {};
    searchFrameRows
      .mockImplementationOnce(() => new Promise((resolve) => (settleFirst = resolve)))
      .mockResolvedValue([]);
    const document = view();
    const { result, rerender } = renderHook(
      ({ query }) => useDocumentSearch(document, query),
      { initialProps: { query: "fenwick" } }
    );
    await act(async () => {
      vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    });

    rerender({ query: "acme" });
    await act(async () => {
      settleFirst([
        { rowIndex: 1, rowId: "source:ledger:1", columnId: "vendor", snippet: "Fenwick Ltd" },
      ]);
    });

    expect(
      result.current.hits.some((hit) => hit.snippet === "Fenwick Ltd"),
      "a superseded scan must not paint its answer over the current query"
    ).toBe(false);
  });
});
