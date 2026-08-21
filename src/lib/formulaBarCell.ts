import {
  isEditableGridColumn,
  isCalculatedFrameColumn,
  rawGridValue,
  visualGridPosition,
  type GridContext,
  type GridFocus,
} from "../FrameGrid";

export type FormulaBarCell = {
  id: string;
  address: string;
  label: string;
  kind: "literal" | "calculated" | "override" | "readOnly";
  value: string;
  reason?: string;
  frameId: string;
  rowId: string;
  columnId: string;
  rowIndex: number;
};

/** Spreadsheet letters remain the quickest compact way to identify a cell. */
export function columnLetters(index: number): string {
  let current = index + 1;
  let result = "";
  while (current > 0) {
    current -= 1;
    result = String.fromCharCode(65 + (current % 26)) + result;
    current = Math.floor(current / 26);
  }
  return result;
}

/** The one selected grid cell as the top formula bar should present it. */
export function formulaBarCell(
  context: GridContext,
  focus: GridFocus
): FormulaBarCell | null {
  const position = visualGridPosition(context, focus.rowId, focus.columnId);
  const rowIndex = context.displayedRows.findIndex((row) => row.id === focus.rowId);
  const columnIndex = context.frame.columns.findIndex(
    (column) => column.id === focus.columnId
  );
  const column = context.frame.columns[columnIndex];
  if (!position || rowIndex < 0 || !column) return null;

  const logicalRow = context.rowOffset + rowIndex;
  const address = `${columnLetters(columnIndex)}${logicalRow + 1}`;
  const common = {
    id: `${context.frame.id}:${focus.rowId}:${column.id}`,
    address,
    label: `${column.name} · row ${logicalRow + 1}`,
    frameId: context.frame.id,
    rowId: focus.rowId,
    columnId: column.id,
    rowIndex: logicalRow,
  };
  const override = context.computed?.overrideFormulas[focus.rowId]?.[column.id];
  if (override !== undefined) {
    return {
      ...common,
      kind: "override",
      value: override,
    };
  }
  const calculatedFormula = context.computed?.formulas[column.id];
  if (isCalculatedFrameColumn(context.computed, column)) {
    return {
      ...common,
      kind: "calculated",
      value: calculatedFormula ?? "",
    };
  }
  if (isEditableGridColumn(context.computed, column)) {
    return {
      ...common,
      kind: "literal",
      value: context.displayedRows[rowIndex].cells[column.id]?.raw ?? "",
    };
  }
  return {
    ...common,
    kind: "readOnly",
    value: rawGridValue(context, position),
    reason: context.computed?.editing.reason ?? "This value is read-only.",
  };
}
