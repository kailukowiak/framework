import type { DocumentView, Selection } from "./types";

/**
 * Keep the most specific part of a selection that still exists after an edit.
 *
 * Imported and paged frames do not store their rows in `document.objects`;
 * those rows arrive from `get_frame_page`. Treating an absent literal row as
 * an absent frame used to close the entire inspector after every formatting
 * operation. A row that cannot be confirmed is therefore shed back to its
 * column (or frame), which keeps the panel and its active rule available
 * without leaving later operations aimed at an id that may have disappeared.
 */
export function reconcileSelection(
  document: DocumentView,
  selection: Selection
): Selection | null {
  const object = document.objects.find(
    (candidate) => candidate.id === selection.objectId
  );
  if (!object) return null;
  if (object.kind !== "frame") return { objectId: object.id, viewId: selection.viewId };

  const columnId = object.columns.some(
    (column) => column.id === selection.columnId
  )
    ? selection.columnId
    : undefined;
  if (!selection.rowId) return { objectId: object.id, viewId: selection.viewId, columnId };

  const computedRows = document.computedFrames[object.id]?.rows ?? {};
  const rowExists =
    object.rows.some((row) => row.id === selection.rowId) ||
    Object.hasOwn(computedRows, selection.rowId);
  return rowExists
    ? { objectId: object.id, viewId: selection.viewId, columnId, rowId: selection.rowId }
    : { objectId: object.id, viewId: selection.viewId, columnId };
}
