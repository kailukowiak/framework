import { describe, expect, it } from "vitest";
import { reconcileSelection } from "./reconcileSelection";
import type { DocumentView, FrameObject } from "./types";

const frame = (rows: FrameObject["rows"] = []): FrameObject => ({
  id: "frame",
  kind: "frame",
  name: "Imported",
  columns: [{ id: "amount", name: "Amount", dataType: "number", categories: null }],
  rows,
} as unknown as FrameObject);

const view = (object: FrameObject, computedRows = {}): DocumentView => ({
  objects: [object],
  computedFrames: { [object.id]: { rows: computedRows } },
} as unknown as DocumentView);

describe("reconcileSelection", () => {
  it("keeps an imported frame's inspector open by falling back from its paged row", () => {
    expect(
      reconcileSelection(view(frame()), {
        objectId: "frame",
        columnId: "amount",
        rowId: "source:frame:37",
      })
    ).toEqual({ objectId: "frame", columnId: "amount", viewId: undefined });
  });

  it("keeps a row that the document or computed frame still carries", () => {
    const selection = { objectId: "frame", columnId: "amount", rowId: "row-1" };
    expect(reconcileSelection(view(frame(), { "row-1": {} }), selection)).toEqual({
      ...selection,
      viewId: undefined,
    });
  });

  it("falls back to the frame when a selected column was deleted", () => {
    expect(
      reconcileSelection(view(frame()), {
        objectId: "frame",
        columnId: "gone",
      })
    ).toEqual({ objectId: "frame", viewId: undefined, columnId: undefined });
  });

  it("closes only when the selected object itself is gone", () => {
    expect(
      reconcileSelection({ objects: [], computedFrames: {} } as unknown as DocumentView, {
        objectId: "gone",
      })
    ).toBeNull();
  });
});
