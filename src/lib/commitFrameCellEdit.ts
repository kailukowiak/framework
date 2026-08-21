import type { GridDirection } from "./gridNavigation";
import type { OperationHandler } from "./handlers";
import type { Column, FrameObject, Row } from "./types";
import { columnFillFormula } from "./columnFillFormula";

/** Route one grid edit to its actual storage or declaration surface. */
export function commitFrameCellEdit({
  frame,
  row,
  column,
  raw,
  move,
  onOperation,
  onTransformColumn,
  onGridStep,
  onSettle,
}: {
  frame: FrameObject;
  row: Row;
  column: Column;
  raw: string;
  move: GridDirection | null;
  onOperation: OperationHandler;
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void;
  onGridStep: (move: GridDirection) => void;
  onSettle: (row: Row, column: Column) => void;
}) {
  const fillFormula = columnFillFormula(raw);
  if (fillFormula) {
    onSettle(row, column);
    onTransformColumn(frame, column, fillFormula);
    return;
  }
  if (raw !== (row.cells[column.id]?.raw ?? "")) {
    // Computed rows store a person's input against its durable key. Literal
    // rows still take the ordinary positional cell write.
    const entryColumn = frame.entryColumns?.find(
      (candidate) => candidate.columnId === column.id
    );
    if (entryColumn) {
      void onOperation({
        type: "setEntryValue",
        frameId: frame.id,
        columnId: column.id,
        key: entryColumn.keyColumnIds.map(
          (keyColumnId) => row.cells[keyColumnId]?.raw ?? ""
        ),
        raw,
      });
    } else {
      void onOperation({
        type: "setCell",
        frameId: frame.id,
        rowId: row.id,
        columnId: column.id,
        raw,
      });
    }
  }
  if (move) onGridStep(move);
  else onSettle(row, column);
}
