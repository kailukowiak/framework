import { describe, expect, it } from "vitest";
import {
  calculateVirtualRowRange,
  MAX_VIRTUAL_SCROLL_HEIGHT,
  TABLE_ROW_HEIGHT,
} from "./frameVirtualization";

describe("calculateVirtualRowRange", () => {
  it("returns an empty range for an empty frame", () => {
    expect(calculateVirtualRowRange(0, 0, 300)).toEqual({
      start: 0,
      end: 0,
      paddingTop: 0,
      paddingBottom: 0,
    });
  });

  it("renders the first viewport plus overscan at the top", () => {
    const range = calculateVirtualRowRange(1_000, 0, 340);
    expect(range.start).toBe(0);
    expect(range.end).toBe(18);
    expect(range.paddingTop).toBe(0);
    expect(range.paddingBottom).toBe((1_000 - 18) * TABLE_ROW_HEIGHT);
  });

  it("keeps the DOM window bounded deep inside a million-row frame", () => {
    const range = calculateVirtualRowRange(1_000_000, 500_000, 340);
    expect(range.start).toBeGreaterThan(0);
    expect(range.end - range.start).toBeLessThanOrEqual(26);
    expect(range.paddingTop + range.paddingBottom).toBeLessThanOrEqual(
      MAX_VIRTUAL_SCROLL_HEIGHT
    );
  });

  it("caps spacer pixels for a billion-row frame", () => {
    const range = calculateVirtualRowRange(1_000_000_000, 4_000_000, 680);
    expect(range.end - range.start).toBeLessThanOrEqual(36);
    expect(range.paddingTop + range.paddingBottom).toBeLessThanOrEqual(
      MAX_VIRTUAL_SCROLL_HEIGHT
    );
    expect(range.paddingTop).toBeGreaterThan(1_000_000);
    expect(range.paddingBottom).toBeGreaterThan(1_000_000);
  });

  it("clamps the window at the final row", () => {
    const range = calculateVirtualRowRange(100, Number.MAX_SAFE_INTEGER, 340);
    expect(range.end).toBe(100);
    expect(range.paddingBottom).toBe(0);
  });
});
