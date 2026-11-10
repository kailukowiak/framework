import { useRef, useState } from "react";
import "./GhostColumnCell.css";

/**
 * One row's cell in the column that does not exist yet.
 *
 * Typing here is the whole gesture for widening a table: the value arrives as
 * a new column with this row's cell filled in. It commits on Return, Tab and
 * blur — never on a button, because nothing here needs announcing — and an
 * untouched cell does nothing at all, so walking the cursor through the edge
 * leaves the table exactly as it was.
 */
export function GhostColumnCell({
  frameId,
  rowId,
  rowNumber,
  nextRowId,
  onCommit,
  onLeave,
  onFocusRow,
}: {
  frameId: string;
  rowId: string;
  rowNumber: number;
  nextRowId: string | null;
  onCommit: (rowId: string, raw: string) => void;
  /** Give the keyboard back to the grid, at this row's last real cell. */
  onLeave: () => void;
  onFocusRow: (rowId: string) => void;
}) {
  const [value, setValue] = useState("");
  // Read at commit time rather than from the render closure: Escape and
  // Return clear the text and move focus in the same event, and the blur
  // that follows must see the clearing, not the text that was there.
  const typed = useRef("");
  const edit = (next: string) => {
    typed.current = next;
    setValue(next);
  };
  const commit = () => {
    const raw = typed.current;
    if (raw.trim() === "") return;
    edit("");
    onCommit(rowId, raw);
  };
  return (
    <td className="frame-edge-cell ghost-column-cell">
      <input
        data-ghost="column"
        data-frame-id={frameId}
        data-row-id={rowId}
        aria-label={`New column, row ${rowNumber}`}
        value={value}
        onChange={(event) => edit(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
            if (nextRowId) onFocusRow(nextRowId);
          } else if (event.key === "Escape") {
            event.preventDefault();
            edit("");
            onLeave();
          } else if (
            event.key === "ArrowLeft" &&
            event.currentTarget.selectionStart === 0 &&
            event.currentTarget.selectionEnd === 0
          ) {
            event.preventDefault();
            commit();
            onLeave();
          }
        }}
      />
    </td>
  );
}
