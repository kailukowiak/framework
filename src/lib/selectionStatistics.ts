import {
  gridCellAt,
  gridRangeForFocus,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";
import { positionsInRange, type GridPosition } from "./gridNavigation";

export type SelectionStatistics = {
  selectedCells: number;
  count: number | null;
  numericCount: number | null;
  sum: number | null;
  average: number | null;
  partial: boolean;
};

type StatisticValue = { numeric?: number };

function statisticValue(
  context: GridContext,
  position: GridPosition
): StatisticValue | null {
  const target = gridCellAt(context, position);
  if (!target) return null;
  const cell = context.computed?.rows[target.row.id]?.[target.column.id];
  if (cell) {
    if (cell.typedValue.type === "null") return null;
    return cell.typedValue.type === "number"
      ? { numeric: cell.typedValue.value }
      : {};
  }
  const raw = target.row.cells[target.column.id]?.raw?.trim() ?? "";
  if (!raw) return null;
  if (
    !["integer", "number", "currency", "percentage"].includes(
      target.column.dataType
    )
  )
    return {};
  const number = Number(raw.replace(/[$,%\s]/g, ""));
  if (!Number.isFinite(number)) return {};
  return {
    numeric: target.column.dataType === "percentage" ? number / 100 : number,
  };
}

/** Instant, ephemeral statistics for a selected grid range. */
export function selectionStatistics(
  context: GridContext,
  focus: GridFocus
): SelectionStatistics | null {
  const range = gridRangeForFocus(context, focus);
  if (!range || (!focus.anchor && !focus.span)) return null;
  const width = range.right - range.left + 1;
  const height = range.bottom - range.top + 1;
  const partial =
    focus.span === "column" && context.totalRows > context.displayedRows.length;
  const selectedColumns = context.orientation === "fieldsAsRows" ? height : width;
  const selectedCells = partial
    ? context.totalRows * selectedColumns
    : width * height;
  if (partial) {
    return {
      selectedCells,
      count: null,
      numericCount: null,
      sum: null,
      average: null,
      partial: true,
    };
  }

  let count = 0;
  let numericCount = 0;
  let sum = 0;
  for (const position of positionsInRange(range)) {
    const value = statisticValue(context, position);
    if (!value) continue;
    count += 1;
    if (value.numeric !== undefined) {
      numericCount += 1;
      sum += value.numeric;
    }
  }
  return {
    selectedCells,
    count,
    numericCount,
    sum: numericCount ? sum : null,
    average: numericCount ? sum / numericCount : null,
    partial: false,
  };
}
