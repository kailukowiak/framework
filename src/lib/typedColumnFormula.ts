import type { Column, FrameObject } from "./types";

/**
 * A formula typed at a cell is a column declaration. `=` is how a spreadsheet
 * hand starts one, and a column is the slot FrameWork keeps declarations in:
 * Excel fills a range, FrameWork fills the column — once, visibly, as a
 * Wrangle step. Returns the formula text, or null for a literal value.
 */
export function typedColumnFormula(raw: string): string | null {
  const source = raw.trim();
  if (!source.startsWith("=")) return null;
  const formula = source.slice(1).trim();
  return formula || null;
}

/** A formula that reads row positions needs a declared order in front of it. */
export function needsDeclaredOrder(formula: string): boolean {
  return /^sequence\s*\(/i.test(formula);
}

/**
 * The new-row line, split into the row it adds and the columns it declares:
 * a formula typed there declares its column the same as one typed in any
 * cell, and whatever else was typed is still a row.
 */
export function splitDraftRow(
  frame: FrameObject,
  draftRow: Record<string, string>
): { values: Record<string, string>; formulas: Array<{ column: Column; formula: string }> } {
  const values: Record<string, string> = {};
  const formulas: Array<{ column: Column; formula: string }> = [];
  for (const [columnId, value] of Object.entries(draftRow)) {
    if (!value.length) continue;
    const formula = typedColumnFormula(value);
    const column = frame.columns.find((candidate) => candidate.id === columnId);
    if (formula && column) formulas.push({ column, formula });
    else values[columnId] = value;
  }
  return { values, formulas };
}
