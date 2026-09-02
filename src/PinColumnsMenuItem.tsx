import { Pin } from "lucide-react";
import { pinnedThrough } from "./lib/pinnedColumns";
import type { Column, FrameObject } from "./lib/types";

/**
 * Freezing the leading columns, from the column menu, where the other
 * column-shaped decisions are. One entry rather than two: on a column outside
 * the frozen region it extends the freeze to here, and on one already inside
 * it there is only one thing a second press can mean.
 */
export function PinColumnsMenuItem({
  frame,
  column,
  onPin,
}: {
  frame: FrameObject;
  /** Null when the menu was opened somewhere other than a column. */
  column: Column | null | undefined;
  onPin: (pinnedColumns: number) => void;
}) {
  if (!column) return null;
  const through = pinnedThrough(frame, column);
  return (
    <button onClick={() => onPin(through)}>
      <Pin size={14} />
      <span>{through === 0 ? "Unpin columns" : "Pin columns through here"}</span>
    </button>
  );
}
