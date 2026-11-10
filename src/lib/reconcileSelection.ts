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
 *
 * A selection that survives whole is handed back as the *same object*. This
 * runs after every operation, and a fresh-but-equal object is a state change
 * to React: it re-renders the app and re-fires every effect keyed on the
 * selection, one tick after the document arrived. The effect that keeps grid
 * focus in step with the selection then runs holding the selection as it was
 * at that render, and a cell pressed inside the tick is judged against it —
 * focus and selection "disagree", grid focus is dropped, and the cell sits
 * highlighted while F2 and typing do nothing until it is clicked again.
 */
export function reconcileSelection(
  document: DocumentView,
  selection: Selection
): Selection | null {
  const kept = reconciled(document, selection);
  return kept &&
    kept.objectId === selection.objectId &&
    kept.viewId === selection.viewId &&
    kept.columnId === selection.columnId &&
    kept.rowId === selection.rowId
    ? selection
    : kept;
}

function reconciled(document: DocumentView, selection: Selection): Selection | null {
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
