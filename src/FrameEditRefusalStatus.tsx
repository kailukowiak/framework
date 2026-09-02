import { isCalculatedFrameColumn } from "./FrameGrid";
import type { ComputedFrame, FrameObject } from "./lib/types";

/**
 * Why the last edit attempt at the focused cell was refused, said in the
 * card's own status line rather than a window-level toast — errors go where
 * the thing is. The engine's `editing.reason` is the frame-level answer; a
 * calculated column beside editable inputs needs the column-level one.
 */
export function frameEditRefusalStatus(
  frame: FrameObject,
  computed: ComputedFrame,
  focus: { editRefused?: boolean; columnId: string } | null
) {
  const refusedColumn = focus?.editRefused
    ? frame.columns.find((column) => column.id === focus.columnId)
    : undefined;
  if (!refusedColumn) return null;
  const text = isCalculatedFrameColumn(computed, refusedColumn)
    ? `${refusedColumn.name} is calculated. Edit its formula instead of typing over one result.`
    : computed.editing.reason ?? "This frame’s cells cannot be edited.";
  return (
    <div className="frame-page-controls frame-edit-refusal" role="status">
      <span>{text}</span>
    </div>
  );
}
