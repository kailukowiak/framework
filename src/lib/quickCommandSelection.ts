import {
  gridRangeForFocus,
  pipelineSortKeys,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";
import type { SelectionTarget } from "../QuickCommands";
import { selectedCanvasView } from "./canvasNavigation";
import { framePermitsColumnEdits, selectedRowsTarget } from "./gridStructure";
import { pinnedThrough } from "./pinnedColumns";
import type { DocumentView, Selection } from "./types";

/**
 * The selected object as Quick Commands names it: the card, the column when
 * one is picked, and the few facts its verbs need to read correctly — whether
 * the card is collapsed, how the column sorts today, how far a pin would
 * reach, and whether the frame owns its rows (which decides whether a column
 * is deleted or hidden). A plain description rather than the objects
 * themselves, so `quickCommandItems` stays testable without a document.
 */
export function quickCommandSelection(
  document: DocumentView | null,
  selection: Selection | null,
  grid: { context: GridContext | null; focus: GridFocus | null } = {
    context: null,
    focus: null,
  }
): SelectionTarget | null {
  if (!document || !selection) return null;
  const object = document.objects.find(
    (candidate) => candidate.id === selection.objectId
  );
  if (!object) return null;
  const frame = object.kind === "frame" ? object : null;
  const column =
    frame?.columns.find((candidate) => candidate.id === selection.columnId) ?? null;
  const sortKey =
    frame && column
      ? pipelineSortKeys(document.computedFrames[frame.id]).find(
          (key) => key.columnId === column.id
        )
      : undefined;
  return {
    name: column ? `${object.name} · ${column.name}` : object.name,
    objectId: object.id,
    kind: object.kind,
    collapsed: selectedCanvasView(document, selection)?.collapsed ?? false,
    frameId: frame?.id,
    column:
      frame && column
        ? {
            id: column.id,
            name: column.name,
            descending: sortKey ? sortKey.descending : null,
            pinThrough: pinnedThrough(frame, column),
            editsColumns: framePermitsColumnEdits(document, frame.id),
          }
        : undefined,
    rows: frame
      ? selectedRowsTarget(document, frame, grid.context, grid.focus, selection.rowId)
      : undefined,
  };
}

/** Ids of every column the grid selection covers, in frame order. */
export function selectedGridColumnIds(context: GridContext, focus: GridFocus): string[] {
  const range = gridRangeForFocus(context, focus);
  if (!range) return [];
  const [from, to] =
    context.orientation === "fieldsAsRows"
      ? [range.top, range.bottom]
      : [range.left, range.right];
  return context.frame.columns.slice(from, to + 1).map((column) => column.id);
}
