import { describe, expect, it, vi } from "vitest";
import { commitFrameCellEdit } from "./commitFrameCellEdit";
import type { Column, FrameObject, Row } from "./types";

const column = { id: "date", name: "Date" } as Column;
const row = {
  id: "row-1",
  cells: { date: { raw: "2026-01-31", overrideFormula: null } },
} as Row;
const frame = {
  id: "months",
  columns: [column],
  entryColumns: [],
} as unknown as FrameObject;

describe("grid cell commits", () => {
  it("promotes a frame-bound sequence to the column declaration", () => {
    const onOperation = vi.fn(async () => null);
    const onTransformColumn = vi.fn();
    commitFrameCellEdit({
      frame,
      row,
      column,
      raw: "=sequence(2026-01-31, periods=frame.len(), step=1mo)",
      move: null,
      onOperation,
      onTransformColumn,
      onGridStep: vi.fn(),
      onSettle: vi.fn(),
    });

    expect(onOperation).not.toHaveBeenCalled();
    expect(onTransformColumn).toHaveBeenCalledWith(
      frame,
      column,
      "sequence(2026-01-31, periods=frame.len(), step=1mo)"
    );
  });
});
