// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CrosstabTable } from "./CrosstabTable";
import type { Column, CrosstabDisplay, FrameObject, Row } from "./lib/types";

function column(id: string, name: string, extra: Partial<Column> = {}): Column {
  return { id, name, dataType: "string", formula: null, ...extra };
}

const dateColumn = column("date", "Date", {
  dataType: "date",
  format: { style: "plain", datePattern: "dayMonthYear" },
});
const nameColumn = column("name", "Name");
const valueColumn = column("value", "Value", { dataType: "number" });

const frame = {
  id: "frame-1",
  columns: [dateColumn, nameColumn, valueColumn],
} as unknown as FrameObject;

function row(id: string, date: string, name: string, value: string): Row {
  return {
    id,
    cells: {
      date: { raw: date, overrideFormula: null },
      name: { raw: name, overrideFormula: null },
      value: { raw: value, overrideFormula: null },
    },
  };
}

const rows: Row[] = [
  row("r1", "2026-09-01", "Alice", "10"),
  row("r2", "2026-09-01", "Bob", "20"),
];

const crosstab: CrosstabDisplay = { namesColumnId: "name", valuesColumnId: "value" };

afterEach(cleanup);

describe("CrosstabTable", () => {
  it("formats a group column's date the same way the grid would", () => {
    render(<CrosstabTable frame={frame} rows={rows} crosstab={crosstab} onOperation={vi.fn()} />);
    expect(screen.getByText("1 Sep 2026")).toBeTruthy();
    expect(screen.queryByText("2026-09-01")).toBeNull();
  });
});
