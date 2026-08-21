import {
  gridRangeForFocus,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";
import type { Column, FrameObject } from "./types";
import {
  inferGeneratorPattern,
  inferGeneratorRule,
  type GeneratorPattern,
} from "./generatorInference";

/** Read the selected run, or the whole clicked column, as an Excel fill. */
export type ContextGeneratorInference = {
  pattern: GeneratorPattern | null;
  rule: string | null;
  actionLabel: string;
};

export function inferContextGenerator(
  frame: FrameObject | null,
  column: Column | null,
  focus: GridFocus | null,
  grid: GridContext | null,
  clickedRowIndex?: number
): ContextGeneratorInference | null {
  if (!frame || !column) return null;
  let rows = frame.rows;
  if (focus?.objectId === frame.id && grid) {
    const range = gridRangeForFocus(grid, focus);
    const columnIndex = frame.columns.findIndex((candidate) => candidate.id === column.id);
    if (
      range &&
      columnIndex >= range.left &&
      columnIndex <= range.right &&
      range.bottom > range.top
    ) {
      rows = grid.displayedRows.slice(range.top, range.bottom + 1);
    }
  } else if (clickedRowIndex !== undefined) {
    return null;
  }
  const raws = rows.slice(0, 100).map((row) => row.cells[column.id]?.raw ?? "");
  const pattern = inferGeneratorPattern(raws);
  return {
    pattern,
    rule: inferGeneratorRule(raws),
    actionLabel: pattern ? "Fill series down frame…" : "Make generator frame",
  };
}
