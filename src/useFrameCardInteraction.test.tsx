// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { FrameObject } from "./lib/types";
import { readVectorDrag, type VectorDrag } from "./lib/vectorDrag";
import { frameColumnVector, useFrameColumnDrag } from "./useFrameCardInteraction";

const frame: FrameObject = {
  kind: "frame",
  id: "sales",
  name: "MyTable",
  columns: [{ id: "column-1", name: "Column1", dataType: "number", formula: null }],
  rows: [],
  derivation: null,
  uniqueKeys: [],
  summaries: [],
  display: { orientation: "recordsAsRows" },
};

function pointerEvent(type: string, x: number, y: number) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, { clientX: { value: x }, clientY: { value: y } });
  return event;
}

afterEach(() => {
  document.body.replaceChildren();
  Reflect.deleteProperty(document, "elementFromPoint");
});

describe("frame column vector drag", () => {
  it("builds the canonical table-column source", () => {
    expect(frameColumnVector(frame, "column-1", 6)).toEqual({
      objectId: "column-1",
      name: "Column1",
      formula: "`MyTable`.`Column1`",
      length: 6,
    });
  });

  it("drops a column on a declared Matrix axis without joining or reordering", () => {
    const target = document.createElement("section");
    target.dataset.vectorDropTarget = "true";
    target.dataset.vectorDropAction = "Use in rows";
    const child = document.createElement("span");
    target.append(child);
    document.body.append(target);
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => child),
    });
    let dropped: VectorDrag | null = null;
    target.addEventListener("drop", (event) => {
      dropped = readVectorDrag((event as DragEvent).dataTransfer!);
    });
    const rearrange = vi.fn();
    const join = vi.fn();
    const { result } = renderHook(() =>
      useFrameColumnDrag(frame, 6, null, rearrange, join)
    );

    act(() => {
      result.current.beginFrameColumnDrag(
        {
          button: 0,
          clientX: 10,
          clientY: 10,
          currentTarget: document.createElement("th"),
        } as unknown as React.PointerEvent,
        "column-1"
      );
      window.dispatchEvent(pointerEvent("pointermove", 20, 20));
      expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
        "Column16 values · Use in rows"
      );
      window.dispatchEvent(pointerEvent("pointerup", 20, 20));
    });

    expect(dropped).toEqual(frameColumnVector(frame, "column-1", 6));
    expect(rearrange).not.toHaveBeenCalled();
    expect(join).not.toHaveBeenCalled();
    expect(document.querySelector(".vector-drag-preview")).toBeNull();
  });
});
