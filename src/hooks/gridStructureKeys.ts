import type { Dispatch, SetStateAction } from "react";
import {
  gridRangeForFocus,
  nextColumnName,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";
import { structureChord, type EdgeKey } from "../lib/gridEdgeGrowth";
import {
  appendRowOperation,
  deleteRowsOperation,
  insertColumnOperation,
  insertRowOperation,
  rowPositionIsMeaningful,
} from "../lib/gridStructure";
import type { GridRange } from "../lib/gridNavigation";
import type { OperationHandler } from "../lib/handlers";
import type { Operation, Selection } from "../lib/types";

type Deps = {
  context: GridContext;
  focus: GridFocus;
  run: OperationHandler;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
};

/**
 * Keys that act at the grid's edges: the structure chords, and the three
 * keystrokes that would otherwise stop dead at the last row or column
 * (Return, Tab, Right-arrow). One entry point so the navigation handler,
 * which is already past its complexity budget, gains a single branch.
 */
export function handleGridEdgeKey({
  event,
  growAtEdge,
  ...deps
}: Deps & {
  event: KeyboardEvent;
  growAtEdge: (key: EdgeKey) => boolean;
}): boolean {
  if (handleGridStructureKey({ event, ...deps })) return true;
  const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
  const key: EdgeKey | null = !plain
    ? null
    : event.key === "Tab"
    ? "tab"
    : event.key === "Enter"
    ? "enter"
    : event.key === "ArrowRight"
    ? "arrowRight"
    : null;
  if (!key || !growAtEdge(key)) return false;
  event.preventDefault();
  return true;
}

/**
 * Insert and delete rows and columns from the keyboard (see `structureChord`
 * for the chords and why they are not Excel's).
 *
 * The selection decides the axis, as it does in a spreadsheet: a whole-row
 * selection (⇧Space) is rows, a whole-column selection (⌃Space) is columns.
 * Insert alone also works from a plain cell, as a row above it — it destroys
 * nothing, and "add a line here" is by far the commonest reading. Delete
 * never guesses: with only cells selected it does nothing, because removing
 * the rows under a rectangle is not what anyone typing a delete chord over a
 * few cells expects.
 *
 * Returns true when the keystroke was a structure chord, handled or refused,
 * so it never falls through to a browser default.
 */
export function handleGridStructureKey({
  event,
  ...deps
}: Deps & { event: KeyboardEvent }): boolean {
  const chord = structureChord(event);
  if (!chord) return false;
  event.preventDefault();
  const { context, focus } = deps;
  // A fields-as-rows card draws a column as a row, so "row" would mean the
  // opposite axis there. Keeping the chords out of it is safer than
  // translating them.
  if (context.orientation === "fieldsAsRows") return true;
  const range = gridRangeForFocus(context, focus);
  if (!range) return true;
  if (focus.span === "column") structureColumns(chord, range, deps);
  else structureRows(chord, range, deps);
  return true;
}

function structureColumns(chord: "insert" | "delete", range: GridRange, deps: Deps) {
  const { context, focus, run } = deps;
  const { frame, computed } = context;
  if (!computed?.editing.columns) return;
  if (chord === "insert") {
    // The shared builder, not a hand-made pair of anchors: the engine
    // refuses an add that names both a column to follow and one to precede.
    const anchor = frame.columns[range.left];
    if (anchor)
      void run(insertColumnOperation(frame, anchor.id, "left", nextColumnName(frame)));
    return;
  }
  // One undo entry however many columns: the selection is deleted as a
  // batch, all or none, so a refusal on one column leaves the rest alone
  // instead of stopping half way through a run of separate edits.
  const doomed = frame.columns.slice(range.left, range.right + 1);
  if (doomed.length === 0) return;
  const deletes: Operation[] = doomed.map((column) => ({
    type: "deleteColumn",
    frameId: frame.id,
    columnId: column.id,
  }));
  void run(
    deletes.length === 1 ? deletes[0] : { type: "batch", operations: deletes }
  ).then((failure) => {
    if (failure) return;
    const survivor =
      frame.columns[range.right + 1] ?? frame.columns[range.left - 1] ?? null;
    settleAfterDelete(
      deps,
      survivor ? { rowId: focus.rowId, columnId: survivor.id } : null
    );
  });
}

function structureRows(chord: "insert" | "delete", range: GridRange, deps: Deps) {
  const { context, focus, run } = deps;
  const { frame, computed, displayedRows } = context;
  if (!computed?.editing.rows) return;
  if (chord === "insert") {
    const above = displayedRows[range.top];
    // Rows read out of a file or a connector have no slot to insert into —
    // the engine refuses a positioned insert there — and under a sort or a
    // filter "above this row" cannot be honoured on screen, so in both the
    // key appends instead of promising a position it cannot keep.
    void run(
      above && rowPositionIsMeaningful(frame, computed, displayedRows)
        ? insertRowOperation(frame, above.id, "above")
        : appendRowOperation(frame.id)
    );
    return;
  }
  const doomed = displayedRows.slice(range.top, range.bottom + 1);
  if (focus.span !== "row" || doomed.length === 0) return;
  const survivor =
    displayedRows[range.bottom + 1] ?? displayedRows[range.top - 1] ?? null;
  const columnId = frame.columns.some((column) => column.id === focus.columnId)
    ? focus.columnId
    : null;
  void run(
    deleteRowsOperation(
      frame.id,
      doomed.map((row) => row.id)
    )
  ).then((failure) => {
    if (!failure)
      settleAfterDelete(
        deps,
        survivor && columnId ? { rowId: survivor.id, columnId } : null
      );
  });
}

/**
 * The cursor stood on something that no longer exists, which would leave
 * every later key with nowhere to act. It moves to what took its place, or
 * back to the card when nothing is left.
 */
function settleAfterDelete(
  { focus, setGridFocus, setSelection }: Deps,
  target: { rowId: string; columnId: string } | null
) {
  if (!target) {
    setGridFocus(null);
    setSelection({ objectId: focus.objectId, viewId: focus.viewId });
    return;
  }
  setGridFocus({ ...focus, ...target, mode: "navigate", anchor: null, span: null });
  setSelection({ objectId: focus.objectId, viewId: focus.viewId, ...target });
}
