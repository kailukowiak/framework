import type { GridContext, GridFocus } from "../FrameGrid";
import { gridRangeForFocus, pipelineSortKeys } from "../FrameGrid";
import type {
  ComputedFrame,
  DataType,
  DocumentView,
  FrameObject,
  Operation,
  Row,
} from "./types";

/**
 * Whether literal columns can be added, deleted and retyped on this frame.
 *
 * The engine reports it as `editing.columns` and that, not `editing.rows`, is
 * the rule it enforces: a frame whose chain ends in a Select owns its rows and
 * still refuses a structural column edit. Menus ask here so the question has
 * one spelling; a missing flag reads as "no", the safe answer for a menu.
 */
export function framePermitsColumnEdits(
  document: DocumentView,
  frameId: string
): boolean {
  return Boolean(document.computedFrames[frameId]?.editing.columns);
}

/**
 * Whether a row can be put *between* two others. A frame read from a file
 * only grows past the file's end, and the engine refuses a position there, so
 * the menu does not offer the gesture it would have to refuse.
 */
export function framePermitsRowInsertion(frame: FrameObject): boolean {
  return !frame.sourceFile && !frame.artifact && !frame.connector;
}

/**
 * Whether "above" and "below" mean the same thing on screen and in the
 * engine. A positioned insert lands in *stored* order, which is also the order
 * the grid draws only while nothing reorders or hides rows. Under a sort the
 * new row would appear wherever the sort puts it, and under a filter the row
 * "below" the one you see may be separated from it by rows you cannot, so a
 * promise of a visual position cannot be kept. In that case the honest verb
 * is the one that promises nothing about position — "Add row", at the end —
 * and the menu, the palette and the chord all ask here so they agree.
 *
 * Comparing the displayed ids with the stored ones catches a column-header
 * sort, a pipeline sort and a filter alike without naming any of them.
 * `displayedRows` is omitted when no grid is rendered, in which case only the
 * pipeline's own sort can be known.
 */
export function rowPositionIsMeaningful(
  frame: FrameObject,
  computed: ComputedFrame | undefined,
  displayedRows?: Row[]
): boolean {
  if (!framePermitsRowInsertion(frame)) return false;
  if (pipelineSortKeys(computed).length > 0) return false;
  if (!displayedRows) return true;
  return (
    displayedRows.length === frame.rows.length &&
    displayedRows.every((row, index) => row.id === frame.rows[index]?.id)
  );
}

/** `rowPositionIsMeaningful` for a menu, which may or may not have a rendered grid. */
export function gridPositionIsMeaningful(
  frame: FrameObject,
  computed: ComputedFrame | undefined,
  grid: GridContext | null
): boolean {
  return rowPositionIsMeaningful(
    frame,
    computed,
    grid?.frame.id === frame.id ? grid.displayedRows : undefined
  );
}

/** A row at the end: the one insertion every owned frame can honour. */
export function appendRowOperation(frameId: string): Operation {
  return { type: "addRow", frameId, values: {} };
}

/** A column after the last one, optionally typed and seeded with a value. */
export function appendColumnOperation(
  frame: FrameObject,
  name: string,
  extra: { dataType?: DataType; values?: { [rowId: string]: string } } = {}
): Operation {
  return {
    type: "addColumn",
    frameId: frame.id,
    name,
    dataType: extra.dataType ?? "string",
    afterColumnId: frame.columns.at(-1)?.id ?? null,
    ...(extra.values ? { values: extra.values } : {}),
  };
}

/**
 * The rows a row-level command acts on: every row the grid selection spans if
 * the clicked row is among them, otherwise just the clicked row. Right-clicking
 * outside a range means that row, the way a spreadsheet reads it.
 *
 * Only rows the grid has loaded can be named. A selection reaching past a
 * paged window deletes what the person can see highlighted, never rows the
 * client has not been shown.
 */
export function selectedRowIds(
  context: GridContext | null,
  focus: GridFocus | null,
  clickedRowId: string | undefined
): string[] {
  if (!clickedRowId) return [];
  if (!context || !focus || focus.objectId !== context.frame.id)
    return [clickedRowId];
  // With fields as rows the displayed rows are the frame's columns, so a row
  // range is not a range of records and a multi-delete would be wrong.
  if (context.orientation === "fieldsAsRows") return [clickedRowId];
  const range = gridRangeForFocus(context, focus);
  if (!range) return [clickedRowId];
  const ids = context.displayedRows
    .slice(range.top, range.bottom + 1)
    .map((row) => row.id);
  return ids.includes(clickedRowId) ? ids : [clickedRowId];
}

/** Above puts the row where the clicked one is; below, before the next. */
export function insertRowOperation(
  frame: FrameObject,
  rowId: string,
  where: "above" | "below"
): Operation {
  if (where === "above")
    return { type: "addRow", frameId: frame.id, values: {}, beforeRowId: rowId };
  const index = frame.rows.findIndex((row) => row.id === rowId);
  const next = index >= 0 ? frame.rows[index + 1] : undefined;
  return {
    type: "addRow",
    frameId: frame.id,
    values: {},
    beforeRowId: next?.id ?? null,
  };
}

/** One row keeps the one-row operation; several are one `deleteRows`. */
export function deleteRowsOperation(frameId: string, rowIds: string[]): Operation {
  return rowIds.length === 1
    ? { type: "deleteRow", frameId, rowId: rowIds[0] }
    : { type: "deleteRows", frameId, rowIds };
}

export function deleteRowsLabel(count: number): string {
  return count > 1 ? `Delete ${count} rows` : "Delete row";
}

/** A new text column beside an existing one. "Left" needs `beforeColumnId`
 *  because "after" has no way to say "first". */
export function insertColumnOperation(
  frame: FrameObject,
  columnId: string,
  side: "left" | "right",
  name: string
): Operation {
  const base = { type: "addColumn" as const, frameId: frame.id, name, dataType: "string" as const };
  return side === "right"
    ? { ...base, afterColumnId: columnId }
    : { ...base, afterColumnId: null, beforeColumnId: columnId };
}

/**
 * What Quick Commands needs to know about the selected rows, or undefined when
 * the frame does not let rows be added or deleted by hand. `below` is the row
 * after the last selected one, so "Insert row below" lands under the whole
 * selection rather than inside it.
 */
export function selectedRowsTarget(
  document: DocumentView,
  frame: FrameObject,
  context: GridContext | null,
  focus: GridFocus | null,
  rowId: string | undefined
):
  | { ids: string[]; canInsertBetween: boolean; below: string | null }
  | undefined {
  if (!document.computedFrames[frame.id]?.editing.rows) return undefined;
  const ids = selectedRowIds(context, focus, rowId);
  if (!ids.length) return undefined;
  const last = frame.rows.findIndex((row) => row.id === ids[ids.length - 1]);
  return {
    ids,
    canInsertBetween: rowPositionIsMeaningful(
      frame,
      document.computedFrames[frame.id],
      context?.frame.id === frame.id ? context.displayedRows : undefined
    ),
    below: (last >= 0 ? frame.rows[last + 1]?.id : undefined) ?? null,
  };
}

/**
 * The palette's structural handlers, sharing the operations the right-click
 * menu builds so the two cannot disagree about what "Insert row below" sends.
 */
export function gridStructureActions(
  document: DocumentView | null,
  run: (operation: Operation) => Promise<string | null>,
  nextName: (frame: FrameObject) => string
) {
  const frameOf = (frameId: string) =>
    document?.objects.find(
      (object): object is FrameObject => object.id === frameId && object.kind === "frame"
    );
  return {
    insertColumn: (frameId: string, columnId: string, side: "left" | "right") => {
      const frame = frameOf(frameId);
      if (frame) void run(insertColumnOperation(frame, columnId, side, nextName(frame)));
    },
    insertRow: (frameId: string, beforeRowId: string | null) =>
      void run(
        beforeRowId === null
          ? appendRowOperation(frameId)
          : { type: "addRow", frameId, values: {}, beforeRowId }
      ),
    deleteRows: (frameId: string, rowIds: string[]) =>
      void run(deleteRowsOperation(frameId, rowIds)),
  };
}
