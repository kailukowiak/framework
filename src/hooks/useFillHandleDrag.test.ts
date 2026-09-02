// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GridRange } from "../lib/gridNavigation";
import type { FrameObject } from "../lib/types";
import { useFillHandleDrag } from "./useFillHandleDrag";

// The claim under test: the fill handle is a doorway, not a fill. A drag
// down opens the column's formula exactly once; every other direction is
// silence, and the drag paints what it would speak for while it is held.

const frame: FrameObject = {
  kind: "frame",
  id: "sales",
  name: "Sales",
  columns: [
    { id: "column-1", name: "Revenue", dataType: "number", formula: null },
    { id: "column-2", name: "Cost", dataType: "number", formula: null },
  ],
  rows: [],
  derivation: null,
  uniqueKeys: [],
  summaries: [],
  display: { orientation: "recordsAsRows" },
};

const ROW_HEIGHT = 20;
const ONE_CELL: GridRange = { top: 1, left: 0, bottom: 1, right: 0 };

/** A grid whose rows carry the same markers RecordsFrameBody writes. */
function mountRows(count: number) {
  const table = document.createElement("table");
  const body = document.createElement("tbody");
  for (let index = 0; index < count; index += 1) {
    const row = document.createElement("tr");
    row.dataset.rowIndex = String(index);
    const cell = document.createElement("td");
    cell.dataset.columnId = "column-1";
    row.append(cell);
    body.append(row);
  }
  table.append(body);
  document.body.append(table);
  Object.defineProperty(document, "elementFromPoint", {
    configurable: true,
    value: vi.fn((_x: number, y: number) =>
      body.children[Math.floor(y / ROW_HEIGHT)]?.firstChild ?? null
    ),
  });
}

/** The y coordinate that lands in the middle of a row. */
const rowY = (index: number) => index * ROW_HEIGHT + ROW_HEIGHT / 2;

function pointerEvent(type: string, y: number) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, { clientX: { value: 5 }, clientY: { value: y } });
  return event;
}

function handlePress() {
  return {
    button: 0,
    pointerId: 1,
    currentTarget: document.createElement("span"),
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  };
}

afterEach(() => {
  document.body.replaceChildren();
  Reflect.deleteProperty(document, "elementFromPoint");
});

describe("fill handle drag", () => {
  it("opens the column's formula once when dragged down, and previews on the way", () => {
    mountRows(6);
    const onTransformColumn = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, null, onTransformColumn)
    );
    expect(result.current.target).toEqual({ column: frame.columns[0], rowIndex: 1 });

    const press = handlePress();
    act(() => {
      result.current.beginFillDrag(press as unknown as React.PointerEvent);
    });
    // The cell under the handle must not take this press as a click, or the
    // gesture would move the very selection it is describing.
    expect(press.stopPropagation).toHaveBeenCalled();
    expect(press.preventDefault).toHaveBeenCalled();

    act(() => {
      window.dispatchEvent(pointerEvent("pointermove", rowY(2)));
    });
    expect(result.current.preview).toEqual({
      columnId: "column-1",
      top: 1,
      bottom: 2,
    });
    act(() => {
      window.dispatchEvent(pointerEvent("pointermove", rowY(3)));
    });
    expect(result.current.preview).toEqual({
      columnId: "column-1",
      top: 1,
      bottom: 3,
    });
    // The anchor keeps its square; the rows below it read as one column.
    expect(result.current.cellClass("column-1", 1)).toBe("fill-anchor");
    expect(result.current.cellClass("column-1", 2)).toBe("fill-preview");
    expect(result.current.cellClass("column-1", 3)).toBe("fill-preview");
    expect(result.current.cellClass("column-1", 4)).toBe("");
    expect(result.current.cellClass("column-2", 2)).toBe("");

    act(() => {
      window.dispatchEvent(pointerEvent("pointerup", rowY(3)));
    });
    expect(onTransformColumn).toHaveBeenCalledTimes(1);
    expect(onTransformColumn).toHaveBeenCalledWith(frame, frame.columns[0], "");
    expect(result.current.preview).toBeNull();

    // The listeners are gone with the gesture: a stray move asks for nothing.
    act(() => {
      window.dispatchEvent(pointerEvent("pointermove", rowY(5)));
    });
    expect(result.current.preview).toBeNull();
  });

  it("does nothing when dragged up or released on the row it started from", () => {
    mountRows(6);
    const onTransformColumn = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, null, onTransformColumn)
    );

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(0)));
    });
    expect(result.current.preview).toBeNull();
    act(() => {
      window.dispatchEvent(pointerEvent("pointerup", rowY(0)));
    });
    expect(onTransformColumn).not.toHaveBeenCalled();

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(1)));
      window.dispatchEvent(pointerEvent("pointerup", rowY(1)));
    });
    expect(onTransformColumn).not.toHaveBeenCalled();
  });

  it("carries no handle for a selection across several columns", () => {
    mountRows(6);
    const onTransformColumn = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(
        frame,
        { top: 0, left: 0, bottom: 2, right: 1 },
        null,
        null,
        onTransformColumn
      )
    );
    expect(result.current.target).toBeNull();

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(4)));
      window.dispatchEvent(pointerEvent("pointerup", rowY(4)));
    });
    expect(result.current.preview).toBeNull();
    expect(onTransformColumn).not.toHaveBeenCalled();
  });

  it("stands the focused cell in for the range a single cell never paints", () => {
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, null, { row: 2, col: 1 }, null, vi.fn())
    );
    expect(result.current.target).toEqual({ column: frame.columns[1], rowIndex: 2 });
  });

  it("carries no handle while a cell is being edited", () => {
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, { mode: "edit" }, vi.fn())
    );
    expect(result.current.target).toBeNull();
  });
});
