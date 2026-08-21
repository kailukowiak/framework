import {
  gridCellAt,
  gridRangeForFocus,
  isEditableGridColumn,
  rawGridValue,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";
import { fillPairs, positionsInRange } from "./gridNavigation";

export type GridCellUpdate = {
  rowId: string;
  columnId: string;
  raw: string;
};

/** Clear only cells whose frame and column explicitly allow literal edits. */
export function clearGridRangeUpdates(
  context: GridContext,
  focus: GridFocus
): GridCellUpdate[] {
  const range = gridRangeForFocus(context, focus);
  if (!range) return [];
  return positionsInRange(range).flatMap((position) => {
    const target = gridCellAt(context, position);
    return target && isEditableGridColumn(context.computed, target.column)
      ? [{ rowId: target.row.id, columnId: target.column.id, raw: "" }]
      : [];
  });
}

/**
 * Copy the visible edge of a selection through its editable literal cells.
 *
 * This is deliberately value fill, not formula fill. A calculated column is
 * one declaration and is edited at that declaration; this helper is for the
 * owned data cells that really do hold individual values.
 */
export function fillGridRangeUpdates(
  context: GridContext,
  focus: GridFocus,
  direction: "down" | "right"
): GridCellUpdate[] {
  const range = gridRangeForFocus(context, focus);
  if (!range) return [];
  return fillPairs(range, direction).flatMap(({ source, target: position }) => {
    const target = gridCellAt(context, position);
    return target && isEditableGridColumn(context.computed, target.column)
      ? [
          {
            rowId: target.row.id,
            columnId: target.column.id,
            raw: rawGridValue(context, source),
          },
        ]
      : [];
  });
}
