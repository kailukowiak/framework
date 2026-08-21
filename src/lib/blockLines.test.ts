import { describe, expect, it } from "vitest";
import {
  continueFormula,
  logicalGroups,
  logicalLineIndexAt,
  logicalLineSpans,
  physicalToLogical,
  scratchworkStripeRows,
} from "./blockLines";

// These cases mirror the core tests in crates/framework-core/tests/blocks.rs
// — the two rules must never drift apart, or the gutter answers land beside
// the wrong lines.
describe("the continuation rule", () => {
  it("joins an indented line onto the one above", () => {
    expect(physicalToLogical("revenue = 10\n  + 5\nrevenue * 2")).toEqual([0, 0, 1]);
    expect(logicalLineSpans("revenue = 10\n  + 5\nrevenue * 2")).toEqual([2, 1]);
  });

  it("never joins onto a blank line", () => {
    expect(physicalToLogical("x = 10\n\n  5")).toEqual([0, 1, 2]);
  });

  it("keeps a chain of continuations on one line", () => {
    expect(logicalLineSpans("total = a\n  + b\n  + c")).toEqual([3]);
  });

  it("treats an indented blank as a blank, not a continuation", () => {
    expect(physicalToLogical("x = 1\n   \ny = 2")).toEqual([0, 1, 2]);
  });

  it("maps a cursor to its logical line", () => {
    const source = "x = 1\n  + 2\ny = 3";
    expect(logicalLineIndexAt(source, source.indexOf("+ 2"))).toBe(0);
    expect(logicalLineIndexAt(source, source.indexOf("y ="))).toBe(1);
  });

  it("regroups text without losing a byte", () => {
    const source = "x = 1\n  + 2\n\ny = 3";
    expect(logicalGroups(source)).toEqual(["x = 1\n  + 2", "", "y = 3"]);
    expect(logicalGroups(source).join("\n")).toBe(source);
  });

  it("keeps a margin close and terminal method in the open calculation", () => {
    const source = "total = (\n  [1, 2, 3]\n).sum()\ntotal / 2";
    expect(logicalGroups(source)).toEqual([
      "total = (\n  [1, 2, 3]\n).sum()",
      "total / 2",
    ]);
  });

  it("does not count delimiters printed inside values", () => {
    expect(logicalGroups('text = "("\nname = `a[b`\n# note (\n3')).toEqual([
      'text = "("',
      "name = `a[b`",
      "# note (",
      "3",
    ]);
  });
});

describe("Alt+Return expansion", () => {
  it("keeps a scratchwork name outside the visible grouping", () => {
    const source = "total = values";
    const expanded = continueFormula(source, source.length, source.length);
    expect(expanded.source).toBe("total = (\n  values\n  \n)");
    expect(expanded.source.slice(expanded.selection.start)).toBe("\n)");
  });

  it("places the rest of a formula on the new inner line", () => {
    const source = "total = values.sum()";
    const at = source.indexOf(".sum");
    const expanded = continueFormula(source, at, at);
    expect(expanded.source).toBe("total = (\n  values\n  .sum()\n)");
  });

  it("adds an ordinary continuation after the expression is expanded", () => {
    const source = "total = (\n  values\n)";
    const at = source.indexOf("values") + "values".length;
    expect(continueFormula(source, at, at).source).toBe(
      "total = (\n  values\n  \n)"
    );
  });
});

describe("scratchwork bands", () => {
  it("alternates live logical lines while blanks stay unbanded", () => {
    expect(scratchworkStripeRows("one\n\ntwo\nthree").map((row) => row.on)).toEqual([
      false,
      false,
      true,
      false,
    ]);
  });

  it("keeps an expanded formula on one band", () => {
    const rows = scratchworkStripeRows(
      "one\ntwo = (\n  values\n).sum()\nthree"
    );
    expect(rows.map(({ span, on }) => ({ span, on }))).toEqual([
      { span: 1, on: false },
      { span: 3, on: true },
      { span: 1, on: false },
    ]);
  });
});
