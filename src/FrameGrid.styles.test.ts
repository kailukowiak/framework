import { describe, expect, it } from "vitest";
import {
  effectiveFrameCellStyle,
  matchedFrameStyles,
  styleMatchesFromFramePage,
  type FrameStyleMatches,
} from "./FrameGrid";
import { emptyStyle } from "./lib/conditionalFormatting";
import type { FramePage } from "./lib/api";
import type { FrameObject } from "./lib/types";

const frame = (): FrameObject =>
  ({
    kind: "frame",
    id: "frame-1",
    name: "Ledger",
    columns: [
      { id: "col-amount", name: "Amount", dataType: "number" },
      { id: "col-status", name: "Status", dataType: "string" },
    ],
    rows: [],
    display: {
      styles: [
        // A direct format on one cell, under everything a rule paints.
        {
          target: { kind: "cell", rowId: "row-1", columnId: "col-amount" },
          style: { ...emptyStyle(), bold: true, textColor: "#20221f" },
        },
      ],
      styleRules: [
        {
          id: "rule-row",
          formula: { expression: null },
          columnId: null,
          output: { kind: "condition", style: { ...emptyStyle(), fillColor: "#f8dfd0" } },
        },
        {
          id: "rule-amount",
          formula: { expression: null },
          columnId: "col-amount",
          output: { kind: "condition", style: { ...emptyStyle(), textColor: "#9a452b" } },
        },
      ],
    },
  }) as unknown as FrameObject;

const matched: FrameStyleMatches = {
  "row-1": [
    { ruleId: "rule-row", style: { ...emptyStyle(), fillColor: "#f8dfd0" } },
    { ruleId: "rule-amount", style: { ...emptyStyle(), textColor: "#9a452b" } },
  ],
};

describe("matchedFrameStyles", () => {
  it("lets a whole-row rule reach every column and the row gutter", () => {
    expect(
      matchedFrameStyles(frame(), "row-1", "col-status", matched).map((m) => m.ruleId)
    ).toEqual(["rule-row"]);
    expect(
      matchedFrameStyles(frame(), "row-1", undefined, matched).map((m) => m.ruleId)
    ).toEqual(["rule-row"]);
  });

  it("confines a scoped rule to its own column", () => {
    expect(
      matchedFrameStyles(frame(), "row-1", "col-amount", matched).map((m) => m.ruleId)
    ).toEqual(["rule-row", "rule-amount"]);
  });

  it("ignores an answer from a rule the frame no longer holds", () => {
    const stale: FrameStyleMatches = {
      "row-1": [{ ruleId: "deleted", style: { ...emptyStyle(), bold: true } }],
    };
    expect(matchedFrameStyles(frame(), "row-1", "col-amount", stale)).toEqual([]);
  });

  it("says nothing about a row no rule answered for", () => {
    expect(matchedFrameStyles(frame(), "row-2", "col-amount", matched)).toEqual([]);
    expect(matchedFrameStyles(frame(), "row-1", "col-amount", undefined)).toEqual([]);
  });
});

describe("effectiveFrameCellStyle", () => {
  it("paints rules over direct formatting, property by property", () => {
    const style = effectiveFrameCellStyle(frame(), "row-1", "col-amount", matched);
    // The cell's own bold survives: a rule replaces only what it sets.
    expect(style.bold).toBe(true);
    expect(style.fillColor).toBe("#f8dfd0");
    // The later rule wins the property both it and the direct format set.
    expect(style.textColor).toBe("#9a452b");
  });

  it("leaves direct formatting alone when no rule answered", () => {
    const style = effectiveFrameCellStyle(frame(), "row-1", "col-amount");
    expect(style.textColor).toBe("#20221f");
    expect(style.fillColor).toBeNull();
  });
});

describe("styleMatchesFromFramePage", () => {
  it("keys a page's answers the way its rows are keyed", () => {
    const page = {
      frameId: "frame-1",
      totalRows: 3,
      offset: 10,
      limit: 2,
      columns: [],
      rowIds: ["row-a"],
      rows: [["1"], ["2"]],
      styleMatches: [
        [{ ruleId: "rule-row", style: { ...emptyStyle(), bold: true } }],
        [],
      ],
    } as unknown as FramePage;
    expect(Object.keys(styleMatchesFromFramePage(frame(), page))).toEqual(["row-a"]);
  });

  it("is empty for a page from a core that sent no answers", () => {
    const page = {
      frameId: "frame-1",
      totalRows: 1,
      offset: 0,
      limit: 1,
      columns: [],
      rowIds: ["row-a"],
      rows: [["1"]],
    } as unknown as FramePage;
    expect(styleMatchesFromFramePage(frame(), page)).toEqual({});
  });
});
