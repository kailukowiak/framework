import type { MutableRefObject } from "react";
import type { CellFormulaRequest } from "./CellFormulaController";
import type { GridContext, GridFocus } from "./FrameGrid";
import { gridCellFormulaAction } from "./lib/gridCellFormula";
import { formulaToken } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import { isPrintableKey } from "./lib/gridNavigation";
import type { Column, Selection } from "./lib/types";

type ColumnFormulaRequest = {
  frameId: string;
  columnId: string;
  formula: string;
  focus: true;
  focusAtEnd: false;
  editExisting: boolean;
  token: number;
};

/** Route `=` by selection scope before ordinary grid editing sees the key. */
export function handleGridFormulaKey({
  event,
  context,
  focus,
  column,
  cellToken,
  columnToken,
  onCellRequest,
  onColumnRequest,
  onScratchworkRequest,
  onSelect,
  onOpenWrangle,
  onOperation,
}: {
  event: KeyboardEvent;
  context: GridContext;
  focus: GridFocus;
  column: Column | undefined;
  cellToken: MutableRefObject<number>;
  columnToken: MutableRefObject<number>;
  onCellRequest: (request: CellFormulaRequest | null) => void;
  onColumnRequest: (request: ColumnFormulaRequest) => void;
  onScratchworkRequest: () => void;
  onSelect: (selection: Selection) => void;
  onOpenWrangle: () => void;
  onOperation: OperationHandler;
}): boolean {
  const action = gridCellFormulaAction({
    key: event.key,
    modifier: event.metaKey || event.ctrlKey,
    printable: isPrintableKey(event),
    isOverride: Boolean(
      context.computed?.rows[focus.rowId]?.[focus.columnId]?.isOverride
    ),
    singleCell: !focus.anchor && !focus.span,
    wholeColumn: focus.span === "column",
  });
  if (!action || (action.kind === "column" && !column)) return false;
  event.preventDefault();
  if (action.kind === "clear") {
    void onOperation({
      type: "setCellOverride",
      frameId: context.frame.id,
      rowId: focus.rowId,
      columnId: focus.columnId,
      formula: null,
    });
  } else if (action.kind === "edit") {
    cellToken.current += 1;
    onCellRequest({
      key: `grid:${cellToken.current}`,
      cellId: `${context.frame.id}:${focus.rowId}:${focus.columnId}`,
      seed: action.seed,
    });
  } else if (action.kind === "scratchwork") {
    onCellRequest(null);
    onScratchworkRequest();
  } else {
    const target = column;
    if (!target) return false;
    const editExisting = Boolean(context.computed?.formulas[target.id]);
    columnToken.current += 1;
    onCellRequest(null);
    onSelect({
      objectId: context.frame.id,
      viewId: focus.viewId,
      columnId: target.id,
    });
    onOpenWrangle();
    onColumnRequest({
      frameId: context.frame.id,
      columnId: target.id,
      formula: editExisting ? "" : formulaToken(target.name),
      focus: true,
      focusAtEnd: false,
      editExisting,
      token: columnToken.current,
    });
  }
  return true;
}
