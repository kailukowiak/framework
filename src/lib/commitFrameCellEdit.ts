import type { GridDirection } from "./gridNavigation";
import type { OperationHandler } from "./handlers";
import type { Column, FrameObject, Row } from "./types";
import { splitDraftRow, typedColumnFormula } from "./typedColumnFormula";

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
  // `=…` is not a value: it belongs to the column, and the column's formula
  // editor opens with it. The keystroke itself is caught earlier, so this
  // is reached only by text pasted or seeded into the editor.
  const formula = typedColumnFormula(raw);
  if (formula) {
    onSettle(row, column);
    onTransformColumn(frame, column, formula);
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

/**
 * Route the new-row line the same way: a formula typed there declares its
 * column, whatever else was typed is still a row, and an untouched line is
 * left alone unless an empty row was asked for outright.
 */
export function commitDraftRowEdit({
  frame,
  draftRow,
  allowEmpty,
  onOperation,
  onTransformColumn,
  onReset,
}: {
  frame: FrameObject;
  draftRow: Record<string, string>;
  allowEmpty: boolean;
  onOperation: OperationHandler;
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void;
  onReset: () => void;
}): Promise<void> {
  const { values, formulas } = splitDraftRow(frame, draftRow);
  const addsRow = allowEmpty || Object.keys(values).length > 0;
  if (!addsRow && !formulas.length) return Promise.resolve();
  for (const { column, formula } of formulas) onTransformColumn(frame, column, formula);
  if (!addsRow) {
    onReset();
    return Promise.resolve();
  }
  // The line is cleared only once the row is in: a typed column can refuse
  // a value, and the refusal should leave what was typed there to fix.
  return onOperation({ type: "addRow", frameId: frame.id, values }).then((failure) => {
    if (failure === null) onReset();
  });
}
