import type { ColumnResize } from "./hooks/useColumnResize";
import { COLUMN_WIDTH_KEY_STEP, storedColumnWidth } from "./lib/columnLayout";
import type { Column, FrameObject } from "./lib/types";

/**
 * The strip on a header's right edge that resizes the column. Invisible at
 * rest: the column border it sits on is the affordance, and the cursor says
 * the rest. It is a sibling of the name button, not inside it, so pressing it
 * can neither start a rearrange drag nor select the column, and it is not
 * the sort button, so releasing it sorts nothing.
 *
 * Exposed as a separator so it can be reached and driven without a pointer:
 * arrows step the width, and the write happens on key up or blur — the same
 * single-write rule as dragging.
 */
export function ColumnResizeHandle({
  frame,
  column,
  resize,
}: {
  frame: FrameObject;
  column: Column;
  /** Absent where the header is shown without a grid to resize: no handle. */
  resize?: ColumnResize;
}) {
  if (!resize) return null;
  const columnId = column.id;
  const columnName = column.name;
  const width = resize.preview?.[columnId] ?? storedColumnWidth(frame, columnId);
  return (
    <div
      className="column-resize-handle"
      role="separator"
      aria-orientation="vertical"
      aria-label={`Resize ${columnName}`}
      aria-valuenow={width ?? undefined}
      tabIndex={0}
      title="Drag to resize · double-click to fit"
      onPointerDown={(event) => resize.beginDrag(event, columnId)}
      onClick={(event) => event.stopPropagation()}
      onDoubleClick={(event) => {
        event.stopPropagation();
        resize.fit(columnId);
      }}
      onKeyDown={(event) => {
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
        event.preventDefault();
        event.stopPropagation();
        resize.nudge(
          columnId,
          event.key === "ArrowRight" ? COLUMN_WIDTH_KEY_STEP : -COLUMN_WIDTH_KEY_STEP,
          event.currentTarget.closest("th")
        );
      }}
      onKeyUp={(event) => {
        if (event.key === "ArrowLeft" || event.key === "ArrowRight")
          resize.commitNudge();
      }}
      onBlur={() => resize.commitNudge()}
    />
  );
}
