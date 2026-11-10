import { frameCardSize, grownFrameCardHeight } from "./cardPlacement";
import type { DocumentView, Operation } from "./types";

/**
 * A paste can make the table bigger in both directions — past the last row,
 * and now past the last column, where a document-owned frame adds "New
 * column" headers for the overflow — and a card still at the size the app
 * gave it should grow to show what arrived rather than hide it behind a
 * scrollbar. A card someone dragged to a size of their own is left alone.
 *
 * Width has no "hand-sized" flag any more than height does, so the same
 * judgement is made from geometry: a width equal to what `frameCardSize`
 * gives for the old column count is the app's own. The engine decides whether
 * the table widened at all; this only asks whether the paste *reached* past
 * the old columns, which is the same fact read off the clipboard.
 *
 * The resize rides in one `batch` with the paste, so one undo takes both
 * back. That is possible because the new size never needed the engine's
 * answer: it is a function of the anchor and the clipboard's shape (the
 * target counts are `anchor + pasted extent`), both known before sending.
 * The engine's own growth of the frame is the same arithmetic, and a paste it
 * refuses refuses the whole batch, so the card is never resized for nothing.
 */
export function cardGrowthForPaste(
  document: DocumentView,
  target: { viewId: string; rowCount: number; columnCount: number }
): Operation | null {
  const view = document.views.find((candidate) => candidate.id === target.viewId);
  const frame = document.objects.find(
    (object) => object.id === view?.objectId && object.kind === "frame"
  );
  if (!view || !frame || frame.kind !== "frame") return null;
  const height = grownFrameCardHeight(view.height, target.rowCount) ?? view.height;
  const oldColumns = frame.columns.length;
  const wanted = frameCardSize(target.columnCount, 0).width;
  const automaticWidth = view.width === frameCardSize(oldColumns, 0).width;
  const width =
    target.columnCount > oldColumns && automaticWidth && wanted > view.width
      ? wanted
      : view.width;
  return width === view.width && height === view.height
    ? null
    : { type: "resizeView", viewId: view.id, width, height };
}

/**
 * The one operation a block paste sends: `pasteCells`, with the card's growth
 * riding in a `batch` beside it when the paste reaches past the card. Fields
 * as rows transposes the block back (its clipboard rows are this frame's
 * columns) and leaves the card alone, since the extent is then not the
 * clipboard's. A refusal of the batch is the engine's own sentence, shown like
 * any single refusal.
 */
export function pasteOperation(
  document: DocumentView,
  paste: {
    frameId: string;
    viewId: string;
    anchor: { rowId: string; columnId: string };
    at: { row: number; col: number };
    matrix: string[][];
    fieldsAsRows: boolean;
  }
): Operation {
  const { matrix, fieldsAsRows } = paste;
  const pasteCells: Operation = {
    type: "pasteCells",
    frameId: paste.frameId,
    rowId: paste.anchor.rowId,
    columnId: paste.anchor.columnId,
    grid: fieldsAsRows
      ? (matrix[0] ?? []).map((_, index) => matrix.map((line) => line[index] ?? ""))
      : matrix,
  };
  const growth = fieldsAsRows
    ? null
    : cardGrowthForPaste(document, {
        viewId: paste.viewId,
        rowCount: paste.at.row + matrix.length,
        columnCount: paste.at.col + (matrix[0]?.length ?? 1),
      });
  return growth ? { type: "batch", operations: [pasteCells, growth] } : pasteCells;
}
