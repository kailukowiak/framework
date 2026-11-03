import type { GridFocus } from "../FrameGrid";
import type { FindHit } from "./documentSearch";
import type { Selection } from "./types";

export type FindJumpTargets = {
  jumpToObject: (objectId: string) => void;
  setSelection: (selection: Selection | null) => void;
  setGridFocus: (focus: GridFocus | null) => void;
};

/**
 * Go to a document match. Where a hit takes you depends on how specific it
 * is: a cell knows its row and column, a column knows its column, and
 * everything else is a thing on the canvas. A jump is two facts — bring the
 * card forward, then put the cursor in it — and both are already
 * implemented; this only says which hit needs which.
 */
export function jumpToFindHit(hit: FindHit, targets: FindJumpTargets): void {
  targets.jumpToObject(hit.objectId);
  if (hit.kind !== "cell" && hit.kind !== "column") return;
  if (!hit.columnId) return;
  // A cell hit knows a row, so the grid gets a cursor; a column hit knows
  // only the column, and putting a cursor on an arbitrary row of it would be
  // inventing an answer the search never gave.
  if (hit.viewId && hit.rowId) {
    targets.setGridFocus({
      viewId: hit.viewId,
      objectId: hit.objectId,
      rowId: hit.rowId,
      columnId: hit.columnId,
      mode: "navigate",
      editSeed: null,
      anchor: null,
      span: null,
      rowIndex: hit.rowIndex,
    });
  }
  targets.setSelection({
    objectId: hit.objectId,
    viewId: hit.viewId,
    rowId: hit.rowId,
    columnId: hit.columnId,
  });
}
