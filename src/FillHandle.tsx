import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";

/** The previewed rows receive copies or an inferred literal series. */
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
      role="button"
      aria-label={`Fill ${column.name}`}
      title={`Drag to fill ${column.name}`}
      onPointerDown={beginFillDrag}
    />
  );
}
