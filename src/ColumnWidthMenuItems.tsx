import { Check, MoveHorizontal } from "lucide-react";
import { columnLayoutOperation, isColumnWrapped, storedColumnWidth } from "./lib/columnLayout";
import type { Column, FrameObject, Operation } from "./lib/types";

/**
 * The two column-menu entries that have no gesture on the grid itself:
 * putting a column back to automatic width (dragging an edge only ever sets
 * one, and double-clicking fits rather than clears) and wrapping its text.
 * Sizing is done on the header edge, so there is no "set width" here.
 *
 * Each sends the column's whole layout through `columnLayoutOperation`, which
 * is what carries the half that is not changing.
 */
export function ColumnWidthMenuItems({
  frame,
  column,
  onOperation,
}: {
  frame: FrameObject;
  column: Column | null;
  onOperation: (operation: Operation) => void;
}) {
  if (!column) return null;
  const wrapped = isColumnWrapped(frame, column.id);
  // A resize stores every column's width at once, so resetting them one at a
  // time would leave a fixed table with a 150px hole where each one was.
  const stored = frame.columns.filter((candidate) => storedColumnWidth(frame, candidate.id) !== null);
  return (
    <>
      {storedColumnWidth(frame, column.id) !== null && (
        <button
          onClick={() =>
            onOperation(columnLayoutOperation(frame, [{ columnId: column.id, width: null }]))
          }
        >
          <MoveHorizontal size={14} />
          <span>Reset column width</span>
        </button>
      )}
      {stored.length > 1 && (
        <button
          onClick={() =>
            onOperation(
              columnLayoutOperation(
                frame,
                stored.map((candidate) => ({ columnId: candidate.id, width: null }))
              )
            )
          }
        >
          <MoveHorizontal size={14} />
          <span>Reset all column widths</span>
        </button>
      )}
      <button
        role="menuitemcheckbox"
        aria-checked={wrapped}
        onClick={() =>
          onOperation(columnLayoutOperation(frame, [{ columnId: column.id, wrap: !wrapped }]))
        }
      >
        {wrapped ? <Check size={14} /> : <span style={{ width: 14 }} />}
        <span>Wrap text</span>
      </button>
    </>
  );
}
