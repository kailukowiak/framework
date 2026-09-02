import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";

/**
 * The small square at the bottom-right of a single-column selection. In a
 * spreadsheet dragging it fills cells; the unit here is the column, so
 * dragging it down opens the column's formula instead of copying values.
 * It renders only in the cell it belongs to and nowhere else.
 */
export function FillHandle({
  model,
  column,
  rowIndex,
}: {
  model: RecordsAsRowsFrameCardProps;
  column: RecordsAsRowsFrameCardProps["frame"]["columns"][number];
  rowIndex: number;
}) {
  const { target, beginFillDrag } = model.fillHandle;
  if (!target || target.column.id !== column.id || target.rowIndex !== rowIndex)
    return null;
  return (
    <span
      className="fill-handle"
      title={`Drag down to write ${column.name}'s formula`}
      onPointerDown={beginFillDrag}
    />
  );
}
