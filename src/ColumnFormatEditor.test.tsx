// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ColumnFormatEditor } from "./ColumnFormatEditor";
import type { OperationHandler } from "./lib/handlers";
import type { Column, FrameObject } from "./lib/types";

afterEach(cleanup);

const column = (id: string, dataType: Column["dataType"]): Column =>
  ({ id, name: id.toUpperCase(), dataType, formula: null }) as Column;
const frame = {
  kind: "frame",
  id: "f",
  name: "Sales",
  columns: [column("a", "integer"), column("b", "integer"), column("d", "date")],
  rows: [],
} as unknown as FrameObject;

describe("ColumnFormatEditor", () => {
  it("formats every column in the range and leaves dates alone", () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <ColumnFormatEditor
        frame={frame}
        column={frame.columns[0]}
        columns={frame.columns}
        onOperation={onOperation}
      />
    );
    fireEvent.change(screen.getByLabelText(/Style/), {
      target: { value: "accounting" },
    });
    const targets = onOperation.mock.calls
      .map(([operation]) => operation)
      .filter((operation) => operation.type === "setColumnFormat")
      .map((operation) => (operation as { columnId: string }).columnId);
    expect(targets).toEqual(["a", "b"]);
    expect(screen.getByText(/2 columns/)).not.toBeNull();
  });

  it("says the format is column-wide when one cell is selected", () => {
    render(
      <ColumnFormatEditor
        frame={frame}
        column={frame.columns[0]}
        onOperation={vi.fn(async () => null)}
      />
    );
    expect(screen.getByText(/whole column/)).not.toBeNull();
  });
});
