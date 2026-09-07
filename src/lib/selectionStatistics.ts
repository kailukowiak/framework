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
  // The evaluated answer first, and the row on screen when there isn't one.
  //
  // A frame carrying a transformation chain is read through pages, so its
  // rows arrive already evaluated while `computed.rows` still describes that
  // frame's *stored* input — where a column the chain calculated has no
  // literal at all and reads as null. Taking that null for an empty cell is
  // what made a calculated column sit the aggregate out: Revenue and Cost
  // summed while Profit was skipped, and Forecast selected on its own
  // reported "Count 0" over six numbers plainly on screen. A null cell over
  // a row that still holds a value means the cache is not describing these
  // rows, so the value on screen is the only value there is.
  const cell = context.computed?.rows[target.row.id]?.[target.column.id];
  if (cell && cell.typedValue.type !== "null") {
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
