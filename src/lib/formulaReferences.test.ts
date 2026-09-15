import { describe, expect, it } from "vitest";
import {
  columnReferenceForPick,
  columnTokenForCellPick,
  contextualFormulaReferenceCompletion,
  contextualFormulaReferenceToken,
  filterFormulaReferences,
  formulaToken,
  getFormulaReferenceQuery,
  insertFormulaReference,
  insertionResumesAt,
  isFormulaExecuteShortcut,
  referenceInsertionRange,
  type FormulaReference,
} from "./formulaReferences";

const references: FormulaReference[] = [
  {
    id: "amount",
    label: "Amount",
    token: "`Amount`",
    kind: "column",
    detail: "number column",
  },
  {
    id: "safety",
    label: "Safety Factor",
    token: "`Safety Factor`",
    kind: "value",
    detail: "Canvas value · 1.7",
  },
  {
    id: "expr.round",
    label: ".round",
    token: ".round(",
    kind: "function",
    detail: ".round(decimals=0)",
    searchTerms: ["rounded"],
  },
];

describe("formula references", () => {
  it("uses backticks for exact names and escapes embedded backticks", () => {
    expect(formulaToken("Amount")).toBe("`Amount`");
    expect(formulaToken("Safety Factor")).toBe("`Safety Factor`");
    expect(formulaToken("Net amount (%)")).toBe("`Net amount (%)`");
    expect(formulaToken("Cost ` cap")).toBe("`Cost `` cap`");
  });

  it("matches compact user input to a spaced reference", () => {
    expect(filterFormulaReferences(references, "SafetyFactor")[0]?.id).toBe("safety");
  });

  it("scopes an accepted frame to its columns", () => {
    const frame: FormulaReference = {
      id: "ledger",
      label: "Ledger",
      token: "`Ledger`.",
      kind: "frame",
      detail: "2 columns",
    };
    const amount: FormulaReference = {
      id: "amount",
      frameId: "ledger",
      label: "Ledger.Amount",
      token: "`Ledger`.`Amount`",
      kind: "column",
      detail: "Number column",
    };
    const completion = contextualFormulaReferenceCompletion(
      [frame, amount],
      "`Ledger`.Am",
      11,
      ".Am"
    );

    expect(completion.qualifier?.id).toBe("ledger");
    expect(completion.suggestions.map((reference) => reference.id)).toEqual([
      "amount",
    ]);
    expect(contextualFormulaReferenceToken(amount, completion.qualifier)).toBe(
      ".`Amount`"
    );
    expect(
      insertFormulaReference(
        "`Ledger`.Am",
        11,
        contextualFormulaReferenceToken(amount, completion.qualifier)
      )
    ).toEqual({ source: "`Ledger`.`Amount`", cursor: 17 });
  });

  it("finds functions by friendly aliases", () => {
    expect(filterFormulaReferences(references, "rounded")[0]?.id).toBe("expr.round");
  });

  it("uses the editor's exact token for a picked column", () => {
    expect(columnReferenceForPick(references, "amount")?.token).toBe("`Amount`");
    expect(columnReferenceForPick(references, "safety")).toBeNull();
  });

  it("expresses a picked cell relative to the row that began the formula", () => {
    expect(columnTokenForCellPick("`Amount`", 8, 5)).toBe("`Amount`.shift(3)");
    expect(columnTokenForCellPick("`Amount`", 8, 10)).toBe("`Amount`.shift(-2)");
    expect(columnTokenForCellPick("`Amount`", 8, 8)).toBe("`Amount`");
    expect(columnTokenForCellPick("`Amount`", undefined, 5)).toBe("`Amount`");
  });

  it("replaces the active token at the cursor", () => {
    const source = "Amount * `Safe";
    const result = insertFormulaReference(source, source.length, "`Safety Factor`");
    expect(result).toEqual({ source: "Amount * `Safety Factor`", cursor: 24 });
    expect(getFormulaReferenceQuery(result.source, result.cursor)).toBe("");
  });

  it("keeps an unfinished backtick in the active query", () => {
    expect(getFormulaReferenceQuery("Amount * `Safety", 16)).toBe("`Safety");
  });

  it("inserts a function with the cursor ready for its arguments", () => {
    expect(insertFormulaReference("`Amount`.rou", 12, ".round(")).toEqual({
      source: "`Amount`.round(",
      cursor: 15,
    });
  });

  it("recognizes Command/Ctrl+Return without consuming ordinary or Option+Return", () => {
    expect(
      isFormulaExecuteShortcut({ key: "Enter", metaKey: true, ctrlKey: false })
    ).toBe(true);
    expect(
      isFormulaExecuteShortcut({ key: "Enter", metaKey: false, ctrlKey: true })
    ).toBe(true);
    expect(
      isFormulaExecuteShortcut({ key: "Enter", metaKey: false, ctrlKey: false })
    ).toBe(false);
  });
});

describe("insertFormulaReference", () => {
  it("inserts a reference where the query started", () => {
    const result = insertFormulaReference("= deb", 5, "`debit`");
    expect(result.source).toBe("= `debit`");
    expect(result.cursor).toBe(9);
  });

  // Typing over an existing reference leaves its closing backtick after the
  // cursor. The token brings its own, and two of them is a parse error the
  // user did not make.
  it("consumes a closing backtick already waiting after the cursor", () => {
    const result = insertFormulaReference("= `deb`", 6, "`debit`");
    expect(result.source).toBe("= `debit`");
    expect(result.cursor).toBe(9);
  });

  it("keeps a following backtick that opens the next reference", () => {
    const result = insertFormulaReference("= deb + `credit`", 5, "`debit`");
    expect(result.source).toBe("= `debit` + `credit`");
  });

  it("leaves the text after an unquoted insertion alone", () => {
    const result = insertFormulaReference("= su + 1", 4, "sum(");
    expect(result.source).toBe("= sum( + 1");
  });
});

describe("insertionResumesAt", () => {
  it("steps over a closing backtick the token brings its own of", () => {
    expect(insertionResumesAt("= `deb`", 6, "`debit`")).toBe(7);
  });

  it("leaves a backtick that opens the next reference", () => {
    expect(insertionResumesAt("= `deb + `credit`", 6, "`debit`")).toBe(6);
  });

  it("leaves everything alone for a token that closes nothing", () => {
    expect(insertionResumesAt("= su`", 4, "sum(")).toBe(4);
  });
});

describe("referenceInsertionRange", () => {
  const command = "`Column 1` = None.cast(\"number\")";

  // A new wrangle row opens with its whole line selected so typing can
  // replace it. A pointed reference must not take that as licence to delete
  // the column's name: what is left cannot be saved as a named command at
  // all.
  it("keeps a pick out of the name when the whole command is selected", () => {
    expect(referenceInsertionRange(command, { start: 0, end: command.length })).toEqual(
      { start: 13, end: command.length }
    );
  });

  it("moves a caret resting in the name to the start of the formula", () => {
    expect(referenceInsertionRange(command, { start: 3, end: 3 })).toEqual({
      start: 13,
      end: 13,
    });
  });

  it("leaves a selection already inside the formula alone", () => {
    expect(referenceInsertionRange(command, { start: 14, end: 18 })).toEqual({
      start: 14,
      end: 18,
    });
  });

  it("leaves an ordinary formula surface alone", () => {
    expect(referenceInsertionRange("`Debit` - `Credit`", { start: 0, end: 18 })).toEqual(
      { start: 0, end: 18 }
    );
  });

  // A predicate is a formula surface with an operator in it, not a command
  // with a name in front: nothing about it is protected from a pick.
  it("leaves a comparison alone", () => {
    for (const predicate of [
      '`Region` == "East"',
      "`Amount` <= 5",
      "`Amount` != 5",
    ])
      expect(
        referenceInsertionRange(predicate, { start: 0, end: predicate.length })
      ).toEqual({ start: 0, end: predicate.length });
  });
});

describe("picking a column that two cards share", () => {
  const references = [
    {
      id: "input",
      label: "Input",
      token: "`Input`",
      kind: "column" as const,
      detail: "column",
      frameId: "source",
    },
    {
      id: "input",
      label: "Predictions.Input",
      token: "`Predictions`.`Input`",
      kind: "column" as const,
      detail: "column of Predictions",
      frameId: "predictions",
    },
  ];

  it("prefers the clicked card's own reference", () => {
    expect(columnReferenceForPick(references, "input", "predictions")?.token).toBe(
      "`Predictions`.`Input`"
    );
    expect(columnReferenceForPick(references, "input", "source")?.token).toBe("`Input`");
  });

  it("falls back to the first declared reference, as it always did", () => {
    expect(columnReferenceForPick(references, "input")?.token).toBe("`Input`");
    expect(columnReferenceForPick(references, "input", "elsewhere")?.token).toBe("`Input`");
  });
});
