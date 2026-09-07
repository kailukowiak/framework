import { describe, expect, it } from "vitest";
import type { ActiveFormulaEditor } from "./activeFormulaEditor";
import {
  formulaCellRangePick,
  formulaColumnPick,
  formulaSiblingPick,
  formulaSummaryPick,
  siblingColumnExplanation,
  siblingColumnInStep,
  siblingReferenceInFormula,
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

  // The tutorial gesture: the formula was opened on the second row, so the
  // first row — index 0 — is one row back. A falsy row index is a real row.
  it("reads the row above row two as one period back", () => {
    expect(
      formulaColumnPick(
        { ...active, completion: { ...active.completion, anchorRowIndex: 1 } },
        "revenue",
        "sales",
        0,
        true
      )
    ).toEqual({ kind: "insert", token: "`Revenue`.shift(1)" });
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

describe("a calculation written beside another", () => {
  const step = {
    kind: "withColumns" as const,
    columns: [
      { outputColumnId: "previous", name: "Previous revenue", formula: "`Revenue`.shift(1)" },
      { outputColumnId: "change", name: "Change", formula: "" },
    ],
  };
  const writingChange: ActiveFormulaEditor = {
    ...active,
    label: "Change",
    completion: {
      ...active.completion,
      targetColumnId: "change",
      scope: { steps: [step], stepIndex: 0 },
    },
  };

  it("names the sibling a click cannot read yet", () => {
    expect(siblingColumnInStep(writingChange, "previous")).toEqual({
      name: "Previous revenue",
    });
    expect(siblingColumnInStep(writingChange, "change")).toBeNull();
    expect(siblingColumnInStep(writingChange, "revenue")).toBeNull();
  });

  // Pointing at it is an ordinary insertion now — the commit is what moves
  // the reader into its own step — so the sentence is only for a chain that
  // arrived already broken, and it names the key that repairs it.
  it("says which key finishes the repair on a chain that arrived broken", () => {
    expect(siblingColumnExplanation("Previous revenue", "Change")).toBe(
      "Previous revenue is added in this same step, so Change cannot read it yet. Press Return on Change to move it into its own step."
    );
  });

  it("inserts a pointed sibling the same way as any other column", () => {
    // The anchor is row 4, so row 3 is one period back: a sibling is not a
    // lesser kind of reference, it is the same reference in the wrong step.
    expect(formulaSiblingPick(writingChange, "previous", "sales", 3)).toEqual({
      kind: "insert",
      token: "`Previous revenue`.shift(1)",
    });
    // The header, which carries no row, is the whole column.
    expect(formulaSiblingPick(writingChange, "previous", "sales", undefined)).toEqual({
      kind: "insert",
      token: "`Previous revenue`",
    });
    expect(formulaSiblingPick(writingChange, "revenue", "sales", 3)).toBeNull();
  });

  it("catches the typed spelling of the same mistake", () => {
    expect(
      siblingReferenceInFormula(
        "`Revenue` - `Previous revenue`",
        [{ name: "Previous revenue" }],
        ["Month", "Revenue"]
      )
    ).toBe("Previous revenue");
  });

  // Replacing a column while another calculation in the same step reads it
  // is reading the value from above, which is exactly what with_columns
  // means. Refusing it would forbid an ordinary chain.
  it("leaves a name that also arrives from above alone", () => {
    expect(
      siblingReferenceInFormula(
        "`Amount` * 2",
        [{ name: "Amount" }],
        ["Amount", "Revenue"]
      )
    ).toBeNull();
  });
});
