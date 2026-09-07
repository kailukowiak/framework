import { expect, it } from "vitest";
import type { Column, Row } from "./types";
import { dragFillCells } from "./dragFill";

const column: Column = { id: "value", name: "Value", dataType: "number", formula: null };
const rows = (values: string[]): Row[] => values.map((raw, index) => ({ id: `row-${index}`, cells: { value: { raw, overrideFormula: null } } }));
const values = (data: string[], first: number, last: number, destination: number, type = column.dataType) =>
  dragFillCells({ ...column, dataType: type }, rows(data), first, last, destination).map((cell) => cell.raw);

it("copies a single number and extends an arithmetic series from two seeds", () => {
  expect(values(["5", "", ""], 0, 0, 2)).toEqual(["5", "5"]);
  expect(values(["5", "10", "", ""], 0, 1, 3)).toEqual(["15", "20"]);
  expect(values(["", "", "5", "10"], 2, 3, 0)).toEqual(["-5", "0"]);
});

it("repeats text patterns and preserves deliberate blanks", () => {
  expect(values(["yes", "no", "", ""], 0, 1, 3, "string")).toEqual(["yes", "no"]);
  expect(values(["5", "", "", ""], 0, 1, 3)).toEqual(["5", ""]);
});

it("extends calendar-month dates and writes stable row addresses", () => {
  expect(values(["2026-01-31", "2026-02-28", "", ""], 0, 1, 3, "date")).toEqual(["2026-03-31", "2026-04-30"]);
  const reordered = rows(["10", "20", "30"]).reverse();
  expect(dragFillCells(column, reordered, 0, 0, 2).map((cell) => cell.rowId)).toEqual(["row-1", "row-0"]);
});
