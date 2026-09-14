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

  function tableCard(objectId: string) {
    const card = document.createElement("section");
    card.className = "canvas-object";
    card.dataset.objectId = objectId;
    const table = document.createElement("table");
    const edge = document.createElement("th");
    edge.className = "frame-edge-header";
    const plus = document.createElement("button");
    edge.append(plus);
    table.append(edge);
    card.append(table);
    document.body.append(card);
    return { card, table, edge, plus };
  }

  it("pairs a column dropped on another table's edge, live, without joining", () => {
    const own = tableCard("sales");
    const other = tableCard("scored");
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => other.plus),
    });
    let dropped: VectorDrag | null = null;
    other.edge.addEventListener("drop", (event) => {
      dropped = readVectorDrag((event as DragEvent).dataTransfer!);
    });
    const rearrange = vi.fn();
    const join = vi.fn();
    const { result } = renderHook(() =>
      useFrameColumnDrag(frame, 6, null, rearrange, join)
    );
    act(() => {
      result.current.beginFrameColumnDrag(
        { button: 0, clientX: 10, clientY: 10, currentTarget: own.edge } as unknown as React.PointerEvent,
        "column-1"
      );
      window.dispatchEvent(pointerEvent("pointermove", 20, 20));
      expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
        "Column16 values · Add column"
      );
      expect(other.table.classList.contains("vector-edge-drop")).toBe(true);
      window.dispatchEvent(pointerEvent("pointerup", 20, 20));
    });
    expect(dropped).toEqual(frameColumnVector(frame, "column-1", 6));
    expect(rearrange).not.toHaveBeenCalled();
    expect(join).not.toHaveBeenCalled();
    expect(other.table.classList.contains("vector-edge-drop")).toBe(false);
  });

  it("does not offer a table its own edge", () => {
    const own = tableCard("sales");
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => own.plus),
    });
    let dropped: VectorDrag | null = null;
    own.edge.addEventListener("drop", (event) => {
      dropped = readVectorDrag((event as DragEvent).dataTransfer!);
    });
    const { result } = renderHook(() =>
      useFrameColumnDrag(frame, 6, null, vi.fn(), vi.fn())
    );
    act(() => {
      result.current.beginFrameColumnDrag(
        { button: 0, clientX: 10, clientY: 10, currentTarget: own.edge } as unknown as React.PointerEvent,
        "column-1"
      );
      window.dispatchEvent(pointerEvent("pointermove", 20, 20));
      window.dispatchEvent(pointerEvent("pointerup", 20, 20));
    });
    expect(dropped).toBeNull();
  });
});
