import { describe, expect, it } from "vitest";
import {
  appendSortKey,
  moveSortKey,
  nextSortKeys,
  removeSortKey,
  reorderSortKeys,
  setSortKeyColumn,
  setSortKeyDirection,
} from "./sortKeys";
import type { SortKey } from "./types";

const keys = (...entries: Array<[string, "asc" | "desc"]>): SortKey[] =>
  entries.map(([columnId, direction]) => ({
    columnId,
    descending: direction === "desc",
  }));

describe("nextSortKeys", () => {
  it("cycles a column through asc, desc, and off", () => {
    const asc = nextSortKeys([], "a");
    expect(asc).toEqual(keys(["a", "asc"]));
    const desc = nextSortKeys(asc, "a");
    expect(desc).toEqual(keys(["a", "desc"]));
    expect(nextSortKeys(desc, "a")).toEqual([]);
  });

  it("accumulates keys in click order, keeping the earlier ones ahead", () => {
    const first = nextSortKeys([], "a");
    const second = nextSortKeys(first, "b");
    expect(second).toEqual(keys(["a", "asc"], ["b", "asc"]));
    expect(nextSortKeys(second, "c")).toEqual(
      keys(["a", "asc"], ["b", "asc"], ["c", "asc"])
    );
  });

  it("cycles one accumulated key without disturbing the others", () => {
    const two = keys(["a", "asc"], ["b", "asc"]);
    const flipped = nextSortKeys(two, "b");
    expect(flipped).toEqual(keys(["a", "asc"], ["b", "desc"]));
    expect(nextSortKeys(flipped, "b")).toEqual(keys(["a", "asc"]));
  });

  it('drops every other key in "only" mode', () => {
    expect(nextSortKeys(keys(["a", "desc"], ["b", "asc"]), "c", "only")).toEqual(
      keys(["c", "asc"])
    );
    expect(nextSortKeys(keys(["a", "asc"], ["b", "asc"]), "b", "only")).toEqual(
      keys(["b", "asc"])
    );
  });

  it('cycles a lone key through asc, desc, and off in "only" mode', () => {
    const desc = nextSortKeys(keys(["a", "asc"]), "a", "only");
    expect(desc).toEqual(keys(["a", "desc"]));
    expect(nextSortKeys(desc, "a", "only")).toEqual([]);
  });
});

describe("sidebar sort key editing", () => {
  it("appends the first column that is not already a key", () => {
    expect(appendSortKey(keys(["a", "asc"]), ["a", "b", "c"])).toEqual(
      keys(["a", "asc"], ["b", "asc"])
    );
  });

  it("leaves the list alone when every column is already a key", () => {
    const all = keys(["a", "asc"], ["b", "desc"]);
    expect(appendSortKey(all, ["a", "b"])).toBe(all);
  });

  it("drops the older key when a row is repointed at a column already sorted on", () => {
    expect(setSortKeyColumn(keys(["a", "asc"], ["b", "desc"]), 1, "a")).toEqual(
      keys(["a", "desc"])
    );
  });

  it("changes one key's direction", () => {
    expect(
      setSortKeyDirection(keys(["a", "asc"], ["b", "asc"]), 1, true)
    ).toEqual(keys(["a", "asc"], ["b", "desc"]));
  });

  it("moves a key to change sort precedence", () => {
    expect(moveSortKey(keys(["a", "asc"], ["b", "desc"]), 1, -1)).toEqual(
      keys(["b", "desc"], ["a", "asc"])
    );
  });

  it("refuses to move a key past either end", () => {
    const two = keys(["a", "asc"], ["b", "desc"]);
    expect(moveSortKey(two, 0, -1)).toBe(two);
    expect(moveSortKey(two, 1, 1)).toBe(two);
  });

  it("gives a dragged key the position it was dropped on", () => {
    const three = keys(["a", "asc"], ["b", "asc"], ["c", "asc"]);
    expect(reorderSortKeys(three, 2, 0)).toEqual(
      keys(["c", "asc"], ["a", "asc"], ["b", "asc"])
    );
    expect(reorderSortKeys(three, 0, 2)).toEqual(
      keys(["b", "asc"], ["c", "asc"], ["a", "asc"])
    );
  });

  it("swaps neighbours when one is dragged onto the other", () => {
    expect(reorderSortKeys(keys(["a", "asc"], ["b", "desc"]), 0, 1)).toEqual(
      keys(["b", "desc"], ["a", "asc"])
    );
    expect(reorderSortKeys(keys(["a", "asc"], ["b", "desc"]), 1, 0)).toEqual(
      keys(["b", "desc"], ["a", "asc"])
    );
  });

  it("clamps a drop past the end to the last position and ignores a drop in place", () => {
    const three = keys(["a", "asc"], ["b", "asc"], ["c", "asc"]);
    expect(reorderSortKeys(three, 0, 9)).toEqual(
      keys(["b", "asc"], ["c", "asc"], ["a", "asc"])
    );
    expect(reorderSortKeys(three, 1, 1)).toBe(three);
  });

  it("removes a key by position", () => {
    expect(removeSortKey(keys(["a", "asc"], ["b", "desc"]), 0)).toEqual(
      keys(["b", "desc"])
    );
  });
});
