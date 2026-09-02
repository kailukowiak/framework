import { describe, expect, it, vi } from "vitest";
import { fixtures, objectNamed } from "../test/support";
import { commitDraftRowEdit, commitFrameCellEdit } from "./commitFrameCellEdit";
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

describe("commitDraftRowEdit", () => {
  const frame = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
  const [region, revenue] = [frame.columns[1], frame.columns[2]];
  const commit = async (
    draftRow: Record<string, string>,
    allowEmpty = false,
    failure: string | null = null
  ) => {
    const onOperation = vi.fn(async () => failure);
    const onTransformColumn = vi.fn();
    const onReset = vi.fn();
    await commitDraftRowEdit({
      frame,
      draftRow,
      allowEmpty,
      onOperation,
      onTransformColumn,
      onReset,
    });
    return { onOperation, onTransformColumn, onReset };
  };

  it("declares a column from a formula and still adds the rest as a row", async () => {
    const { onOperation, onTransformColumn, onReset } = await commit({
      [region.id]: "East",
      [revenue.id]: "=`Cost` * 12",
    });
    expect(onTransformColumn).toHaveBeenCalledWith(frame, revenue, "`Cost` * 12");
    expect(onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: frame.id,
      values: { [region.id]: "East" },
    });
    expect(onReset).toHaveBeenCalled();
  });

  it("adds no row for a formula alone, and does nothing for an untouched line", async () => {
    const only = await commit({ [revenue.id]: "=`Cost` * 12" });
    expect(only.onTransformColumn).toHaveBeenCalledTimes(1);
    expect(only.onOperation).not.toHaveBeenCalled();
    expect(only.onReset).toHaveBeenCalled();
    const untouched = await commit({ [region.id]: "" });
    expect(untouched.onOperation).not.toHaveBeenCalled();
    expect(untouched.onReset).not.toHaveBeenCalled();
    expect((await commit({}, true)).onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: frame.id,
      values: {},
    });
  });

  it("keeps the typed line when the row is refused, so it can be fixed", async () => {
    const refused = await commit(
      { [revenue.id]: "lots" },
      false,
      "'lots' is not a valid integer for column 'Revenue'; use a whole number"
    );
    expect(refused.onOperation).toHaveBeenCalledTimes(1);
    expect(refused.onReset).not.toHaveBeenCalled();
  });
});
