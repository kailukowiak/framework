// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { usePipelineColumnRequests } from "./usePipelineColumnRequests";
import type { Column, FrameObject } from "../lib/types";

const frame = { id: "sales", kind: "frame" } as FrameObject;
const column: Column = {
  id: "revenue",
  name: "Revenue",
  dataType: "number",
  formula: null,
};

function requests() {
  return renderHook(() =>
    usePipelineColumnRequests({
      setContextMenu: vi.fn(),
      setSelection: vi.fn(),
      setInspectorSection: vi.fn(),
    })
  ).result;
}

describe("the requests that open a column's formula", () => {
  it("prepares the editor without opening Wrangle for grid edits", () => {
    const setInspectorSection = vi.fn();
    const { result } = renderHook(() => usePipelineColumnRequests({
      setContextMenu: vi.fn(), setSelection: vi.fn(), setInspectorSection,
    }));
    act(() => result.current.requestColumnFill(frame, column, "", 0));
    expect(setInspectorSection).toHaveBeenLastCalledWith({ panel: "prepare", section: "wrangle" });
    act(() => result.current.requestCalculatedColumnEdit(frame, column, 0));
    expect(setInspectorSection).toHaveBeenLastCalledWith({ panel: "prepare", section: "wrangle" });
    act(() => result.current.requestColumnTransformation(frame, column, "`Revenue` * 2"));
    expect(setInspectorSection).toHaveBeenLastCalledWith("wrangle");
  });
  // The row is the whole reason a formula started from a cell can point at
  // another cell and mean "one row back". It used to be taken and dropped,
  // so every reference pointed at afterwards came out as the whole column.
  it("carries the cell's row as the formula's anchor", () => {
    const result = requests();
    act(() => result.current.requestColumnFill(frame, column, "", 4));
    expect(result.current.transformColumnRequest).toMatchObject({
      frameId: "sales",
      columnId: "revenue",
      formula: "`Revenue`",
      focus: true,
      anchorRowIndex: 4,
    });
  });

  it("leaves the anchor out when the gesture began at no row", () => {
    const result = requests();
    act(() =>
      result.current.requestColumnTransformation(frame, column, "`Revenue` * 2")
    );
    expect(result.current.transformColumnRequest?.anchorRowIndex).toBeUndefined();
  });
});
