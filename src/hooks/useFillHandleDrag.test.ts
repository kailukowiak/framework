// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GridRange } from "../lib/gridNavigation";
import { useFillHandleDrag } from "./useFillHandleDrag";

import { fixtures, objectNamed } from "../test/support";
const view = fixtures.salesBeforeFormula;
const frame = objectNamed(view, "frame", "Monthly sales");
const computed = view.computedFrames[frame.id];
const options = (onOperation = vi.fn().mockResolvedValue(null)) => ({ computed, rows: frame.rows, onOperation });

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
    cell.dataset.columnId = frame.columns[0].id;
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
    currentTarget: document.querySelector("td")!,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  };
}

afterEach(() => {
  document.body.replaceChildren();
  Reflect.deleteProperty(document, "elementFromPoint");
});

describe("fill handle drag", () => {
  it("writes the previewed literal rows as one operation", () => {
    mountRows(6);
    const onOperation = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, null, options(onOperation))
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
      columnId: frame.columns[0].id,
      top: 1,
      bottom: 2,
    });
    act(() => {
      window.dispatchEvent(pointerEvent("pointermove", rowY(3)));
    });
    expect(result.current.preview).toEqual({
      columnId: frame.columns[0].id,
      top: 1,
      bottom: 3,
    });
    // The anchor keeps its square; the rows below it read as one column.
    expect(result.current.cellClass(frame.columns[0].id, 1)).toBe("fill-anchor");
    expect(result.current.cellClass(frame.columns[0].id, 2)).toBe("fill-preview");
    expect(result.current.cellClass(frame.columns[0].id, 3)).toBe("fill-preview");
    expect(result.current.cellClass(frame.columns[0].id, 4)).toBe("");
    expect(result.current.cellClass(frame.columns[1].id, 2)).toBe("");

    act(() => {
      window.dispatchEvent(pointerEvent("pointerup", rowY(3)));
    });
    expect(onOperation).toHaveBeenCalledTimes(1);
    expect(onOperation).toHaveBeenCalledWith({ type: "setCells", frameId: frame.id,
      cells: frame.rows.slice(2, 4).map((row) => ({ rowId: row.id, columnId: frame.columns[0].id, raw: frame.rows[1].cells[frame.columns[0].id].raw })) });
    expect(result.current.preview).toBeNull();

    // The listeners are gone with the gesture: a stray move asks for nothing.
    act(() => {
      window.dispatchEvent(pointerEvent("pointermove", rowY(5)));
    });
    expect(result.current.preview).toBeNull();
  });

  it("fills upwards, but does nothing on the starting row", () => {
    mountRows(6);
    const onOperation = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, null, options(onOperation))
    );

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(0)));
    });
    expect(result.current.preview).not.toBeNull();
    act(() => {
      window.dispatchEvent(pointerEvent("pointerup", rowY(0)));
    });
    expect(onOperation).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(1)));
      window.dispatchEvent(pointerEvent("pointerup", rowY(1)));
    });
    expect(onOperation).toHaveBeenCalledTimes(1);
  });

  it("carries no handle for a selection across several columns", () => {
    mountRows(6);
    const onOperation = vi.fn();
    const { result } = renderHook(() =>
      useFillHandleDrag(
        frame,
        { top: 0, left: 0, bottom: 2, right: 1 },
        null,
        null,
        options(onOperation)
      )
    );
    expect(result.current.target).toBeNull();

    act(() => {
      result.current.beginFillDrag(handlePress() as unknown as React.PointerEvent);
      window.dispatchEvent(pointerEvent("pointermove", rowY(4)));
      window.dispatchEvent(pointerEvent("pointerup", rowY(4)));
    });
    expect(result.current.preview).toBeNull();
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("stands the focused cell in for the range a single cell never paints", () => {
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, null, { row: 2, col: 1 }, null, options())
    );
    expect(result.current.target).toEqual({ column: frame.columns[1], rowIndex: 2 });
  });

  it("carries no handle while a cell is being edited", () => {
    const { result } = renderHook(() =>
      useFillHandleDrag(frame, ONE_CELL, null, { mode: "edit" }, options())
    );
    expect(result.current.target).toBeNull();
  });
});
