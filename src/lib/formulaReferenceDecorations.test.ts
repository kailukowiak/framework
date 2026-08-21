import { describe, expect, it } from "vitest";
import type { FormulaReference } from "./formulaReferences";
import {
  formulaReferenceDecorations,
  rowRangeAfter,
} from "./formulaReferenceDecorations";

const references: FormulaReference[] = [
  {
    id: "revenue",
    label: "Revenue",
    token: "`Revenue`",
    kind: "column",
    detail: "number column",
    frameId: "sales",
  },
  {
    id: "cost",
    label: "Cost",
    token: "`Cost`",
    kind: "column",
    detail: "number column",
    frameId: "sales",
  },
  {
    id: "remote-revenue",
    label: "Forecast.Revenue",
    token: "`Forecast`.`Revenue`",
    kind: "column",
    detail: "number column",
    frameId: "forecast",
  },
];

describe("formula reference decorations", () => {
  it("gives each resolved reference a color and reuses it", () => {
    const decorated = formulaReferenceDecorations(
      "`Revenue` - `Cost` + `Revenue`",
      references
    );
    expect(decorated.map(({ reference, colorIndex }) => [reference.id, colorIndex]))
      .toEqual([
        ["revenue", 0],
        ["cost", 1],
        ["revenue", 0],
      ]);
  });

  it("recognizes forgiving local names without stealing qualified names", () => {
    const decorated = formulaReferenceDecorations(
      "Revenue + `Forecast`.`Revenue`",
      references
    );
    expect(decorated.map(({ reference }) => reference.id)).toEqual([
      "revenue",
      "remote-revenue",
    ]);
  });

  it("does not mistake a method name for a same-named local reference", () => {
    const count = {
      ...references[0],
      id: "count",
      label: "count",
      token: "`count`",
    };
    expect(formulaReferenceDecorations("`Revenue`.count()", [references[0], count]))
      .toHaveLength(1);
  });

  it("does not paint a simple token inside a longer identifier", () => {
    const cost = { ...references[1], token: "Cost" };
    expect(formulaReferenceDecorations("CostCenter + Cost", [cost])).toEqual([
      expect.objectContaining({ start: 13, reference: cost }),
    ]);
  });

  it("keeps row offsets with their source reference", () => {
    const [previous, next] = formulaReferenceDecorations(
      "`Revenue`.shift(2) + `Cost`.shift(-1)",
      references
    );
    expect(previous.rowOffset).toBe(2);
    expect(next.rowOffset).toBe(-1);
  });

  it("keeps a scalar row bound with its column", () => {
    expect(rowRangeAfter("`Revenue`.head(4).last()", 9)).toEqual({
      start: 3,
      end: 3,
    });
    expect(
      formulaReferenceDecorations("`Revenue`.head(4).last()", references)[0].rowRange
    ).toEqual({ start: 3, end: 3 });
  });

  it("does not treat a declaration name as an input reference", () => {
    const decorated = formulaReferenceDecorations(
      "`Revenue` = `Revenue`.shift(1)",
      references
    );
    expect(decorated).toHaveLength(1);
    expect(decorated[0].start).toBe(12);
  });

  it("does not paint names inside strings or comments", () => {
    const decorated = formulaReferenceDecorations(
      '"Revenue" + `Revenue` # Cost\nCost',
      references
    );
    expect(decorated.map(({ reference }) => reference.id)).toEqual([
      "revenue",
      "cost",
    ]);
  });

  it("does not treat a hash inside an exact name as a comment", () => {
    const marked = {
      ...references[0],
      id: "marked",
      label: "Revenue #",
      token: "`Revenue #`",
    };
    expect(formulaReferenceDecorations("`Revenue #` + 1", [marked])).toHaveLength(1);
  });
});
