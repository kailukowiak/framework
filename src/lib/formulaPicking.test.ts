import { describe, expect, it } from "vitest";
import type { ActiveFormulaEditor } from "./activeFormulaEditor";
import {
  formulaCellRangePick,
  formulaColumnPick,
  formulaSummaryPick,
  summaryFormulaToken,
} from "./formulaPicking";

const active: ActiveFormulaEditor = {
  id: "column:previous",
  label: "Previous",
  kind: "formula",
  draft: "",
  selection: { start: 0, end: 0 },
  focused: true,
  canCommit: true,
  completion: {
    references: [
      {
        id: "revenue",
        label: "Revenue",
        token: "`Revenue`",
        kind: "column",
        detail: "number column",
      },
    ],
    targetColumnId: "previous",
    anchorFrameId: "sales",
    anchorRowIndex: 4,
  },
};

describe("formula cell picking", () => {
  it("turns a previous-row pick into shift syntax", () => {
    expect(formulaColumnPick(active, "revenue", "sales", 3, true)).toEqual({
      kind: "insert",
      token: "`Revenue`.shift(1)",
    });
  });

  it("routes the previous value of the target to recurrence authoring", () => {
    expect(formulaColumnPick(active, "previous", "sales", 3, true)).toEqual({
      kind: "recurrence",
    });
  });

  it("inserts previous() while an existing recurrence is being edited", () => {
    expect(
      formulaColumnPick(
        {
          ...active,
          completion: { ...active.completion, previousResultToken: "previous()" },
        },
        "previous",
        "sales",
        3,
        true
      )
    ).toEqual({ kind: "insert", token: "previous()" });
  });

  it("still refuses a same-row self-reference", () => {
    expect(formulaColumnPick(active, "previous", "sales", 4, true)).toMatchObject({
      kind: "refuse",
      message: expect.stringContaining("same row"),
    });
  });

  it("explains why a later target value cannot seed recurrence", () => {
    expect(formulaColumnPick(active, "previous", "sales", 5, true)).toMatchObject({
      kind: "refuse",
      message: expect.stringContaining("earlier row"),
    });
  });

  it("turns a cell clicked from Scratchwork into one scalar value", () => {
    expect(
      formulaColumnPick(
        {
          ...active,
          kind: "scratchwork",
          completion: {
            references: active.completion.references,
          },
        },
        "revenue",
        "sales",
        3,
        true
      )
    ).toEqual({
      kind: "insert",
      token: "`Revenue`.head(4).last()",
    });
  });

  it("refuses an ordinal cell address when the frame can change its rows", () => {
    expect(
      formulaColumnPick(
        {
          ...active,
          kind: "scratchwork",
          completion: {
            references: active.completion.references,
          },
        },
        "revenue",
        "sales",
        3,
        false
      )
    ).toEqual({
      kind: "refuse",
      message: expect.stringContaining("internal dataset"),
    });
  });

  it("keeps a Scratchwork drag from degrading to its first cell", () => {
    const scratchwork = {
      ...active,
      kind: "scratchwork" as const,
      completion: { references: active.completion.references },
    };
    expect(formulaCellRangePick(scratchwork, "revenue", "sales", 4, 1, true))
      .toMatchObject({ kind: "refuse", message: expect.stringContaining("Wrangle") });
    expect(formulaCellRangePick(scratchwork, "revenue", "sales", 2, 2, true))
      .toEqual({ kind: "insert", token: "`Revenue`.head(3).last()" });
  });

});

describe("formula summary picking", () => {
  it("inserts the aggregate represented by the clicked cell", () => {
    expect(formulaSummaryPick(active, "sum", "revenue")).toEqual({
      kind: "insert",
      token: "`Revenue`.sum()",
    });
    expect(formulaSummaryPick(active, "quartile25", "revenue")).toEqual({
      kind: "insert",
      token: "`Revenue`.quantile(0.25)",
    });
  });

  it("spells null-safe distinct and stable mode exactly as the profile does", () => {
    expect(summaryFormulaToken("countDistinct", "`Name`")).toBe(
      "`Name`.drop_nulls().n_unique()"
    );
    expect(summaryFormulaToken("mode", "`Name`")).toBe(
      "`Name`.drop_nulls().mode(True).first()"
    );
  });

  it("refuses a column the active editor deliberately cannot read", () => {
    expect(formulaSummaryPick(active, "sum", "previous")).toMatchObject({
      kind: "refuse",
    });
  });
});
