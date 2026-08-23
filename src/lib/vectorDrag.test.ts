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
