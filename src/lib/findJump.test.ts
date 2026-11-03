import { describe, expect, it, vi } from "vitest";
import type { FindHit } from "./documentSearch";
import { jumpToFindHit } from "./findJump";

const targets = () => ({
  jumpToObject: vi.fn(),
  setSelection: vi.fn(),
  setGridFocus: vi.fn(),
});

const hit = (overrides: Partial<FindHit>): FindHit => ({
  id: "hit",
  kind: "object",
  objectId: "sales",
  viewId: "view-sales",
  label: "North Sales",
  snippet: "Frame",
  ...overrides,
});

describe("jumping to a document match", () => {
  it("brings an object forward and goes no further", () => {
    const to = targets();
    jumpToFindHit(hit({}), to);
    expect(to.jumpToObject).toHaveBeenCalledWith("sales");
    expect(to.setSelection).not.toHaveBeenCalled();
    expect(to.setGridFocus).not.toHaveBeenCalled();
  });

  it("selects a column without inventing a row for it", () => {
    const to = targets();
    jumpToFindHit(hit({ kind: "column", columnId: "region" }), to);
    expect(to.setSelection).toHaveBeenCalledWith(
      expect.objectContaining({ objectId: "sales", columnId: "region", rowId: undefined })
    );
    expect(to.setGridFocus).not.toHaveBeenCalled();
  });

  it("puts the cursor on a cell, carrying a paged row's absolute position", () => {
    const to = targets();
    jumpToFindHit(
      hit({ kind: "cell", columnId: "region", rowId: "row-900", rowIndex: 900 }),
      to
    );
    expect(to.setGridFocus).toHaveBeenCalledWith(
      expect.objectContaining({ rowId: "row-900", rowIndex: 900, mode: "navigate" })
    );
    expect(to.setSelection).toHaveBeenCalledWith(
      expect.objectContaining({ rowId: "row-900", columnId: "region" })
    );
  });
});
