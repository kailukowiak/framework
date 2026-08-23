// @vitest-environment jsdom

import { createElement, type DragEvent as ReactDragEvent } from "react";
import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  beginVectorPointerDrag,
  readVectorDrag,
  type VectorDrag,
} from "./vectorDrag";

const vector: VectorDrag = {
  objectId: "scenario",
  formula: "`Scenario vectors`.`Scenario`",
  name: "Scenario",
  length: 3,
};

function pointerEvent(type: string, x: number, y: number) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, {
    clientX: { value: x },
    clientY: { value: y },
  });
  return event;
}

describe("vector pointer drag", () => {
  afterEach(() => {
    document.body.replaceChildren();
    Reflect.deleteProperty(document, "elementFromPoint");
    vi.restoreAllMocks();
  });

  it("delivers a pointer gesture to the existing vector drop path", () => {
    const source = document.createElement("small");
    let dropped: VectorDrag | null = null;
    const { container } = render(
      createElement("div", {
        className: "column-header",
        onDrop: (event: ReactDragEvent) => {
          dropped = readVectorDrag(event.dataTransfer);
        },
      })
    );
    const target = container.firstElementChild as HTMLElement;
    target.dataset.vectorColumnsAfter = "1";
    target.dataset.vectorSelectedColumns = "0";
    document.body.append(source);
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => target),
    });
    beginVectorPointerDrag(
      {
        button: 0,
        clientX: 10,
        clientY: 10,
        currentTarget: source,
        preventDefault: vi.fn(),
        stopPropagation: vi.fn(),
      } as unknown as PointerEvent,
      vector
    );
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));

    expect(target.classList.contains("vector-column-drop")).toBe(true);
    expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
      "Scenario3 values · Add column"
    );

    window.dispatchEvent(pointerEvent("pointerup", 20, 20));

    expect(dropped).toEqual(vector);
    expect(target.classList.contains("vector-column-drop")).toBe(false);
    expect(document.querySelector(".vector-drag-preview")).toBeNull();
  });

  it("uses the full visible table edge as the add-column target", () => {
    const source = document.createElement("small");
    const table = document.createElement("table");
    const edgeHeader = document.createElement("th");
    const edgeCell = document.createElement("td");
    edgeHeader.className = "frame-edge-header";
    edgeCell.className = "frame-edge-cell";
    table.append(edgeHeader, edgeCell);
    document.body.append(source, table);
    let dropped: VectorDrag | null = null;
    edgeHeader.addEventListener("drop", (event) => {
      dropped = readVectorDrag((event as DragEvent).dataTransfer!);
    });
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => edgeCell),
    });

    beginVectorPointerDrag(
      {
        button: 0,
        clientX: 10,
        clientY: 10,
        currentTarget: source,
        preventDefault: vi.fn(),
        stopPropagation: vi.fn(),
      } as unknown as PointerEvent,
      vector
    );
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));

    expect(edgeHeader.classList.contains("vector-column-drop")).toBe(true);
    expect(table.classList.contains("vector-edge-drop")).toBe(true);
    expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
      "Scenario3 values · Add column"
    );

    window.dispatchEvent(pointerEvent("pointerup", 20, 20));

    expect(dropped).toEqual(vector);
    expect(table.classList.contains("vector-edge-drop")).toBe(false);
  });

  it("previews the explicit layout choice on a one-column vector table", () => {
    const source = document.createElement("small");
    const target = document.createElement("th");
    target.className = "column-header";
    target.dataset.vectorCombine = "true";
    document.body.append(source, target);
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => target),
    });

    beginVectorPointerDrag(
      {
        button: 0,
        clientX: 10,
        clientY: 10,
        currentTarget: source,
        preventDefault: vi.fn(),
        stopPropagation: vi.fn(),
      } as unknown as PointerEvent,
      vector
    );
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));

    expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
      "Scenario3 values · Choose layout"
    );

    window.dispatchEvent(pointerEvent("pointercancel", 20, 20));
  });

  it("delivers the pointer gesture to a declared matrix axis", () => {
    const source = document.createElement("small");
    const target = document.createElement("section");
    const child = document.createElement("textarea");
    target.dataset.vectorDropTarget = "true";
    target.dataset.vectorDropAction = "Use in rows";
    target.append(child);
    document.body.append(source, target);
    let dropped: VectorDrag | null = null;
    target.addEventListener("drop", (event) => {
      dropped = readVectorDrag((event as DragEvent).dataTransfer!);
    });
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => child),
    });

    beginVectorPointerDrag(
      {
        button: 0,
        clientX: 10,
        clientY: 10,
        currentTarget: source,
        preventDefault: vi.fn(),
        stopPropagation: vi.fn(),
      } as unknown as PointerEvent,
      vector
    );
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));

    expect(target.classList.contains("vector-column-drop")).toBe(true);
    expect(document.querySelector(".vector-drag-preview")?.textContent).toBe(
      "Scenario3 values · Use in rows"
    );

    window.dispatchEvent(pointerEvent("pointerup", 20, 20));

    expect(dropped).toEqual(vector);
    expect(target.classList.contains("vector-column-drop")).toBe(false);
  });

  it("keeps an ordinary press from becoming a drop", () => {
    const source = document.createElement("small");
    const target = document.createElement("th");
    target.className = "column-header";
    document.body.append(source, target);
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => target),
    });
    const drop = vi.fn();
    target.addEventListener("drop", drop);

    beginVectorPointerDrag(
      {
        button: 0,
        clientX: 10,
        clientY: 10,
        currentTarget: source,
        preventDefault: vi.fn(),
        stopPropagation: vi.fn(),
      } as unknown as PointerEvent,
      vector
    );
    window.dispatchEvent(pointerEvent("pointerup", 10, 10));

    expect(drop).not.toHaveBeenCalled();
  });
});
