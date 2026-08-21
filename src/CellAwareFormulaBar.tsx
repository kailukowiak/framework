import { useEffect, useRef, useState } from "react";
import {
  CellFormulaController,
  type CellFormulaRequest,
} from "./CellFormulaController";
import {
  ScratchworkFormulaBar,
  type ScratchworkFormulaFeedback,
} from "./ScratchworkFormulaBar";
import { formulaBarCell } from "./lib/formulaBarCell";
import { columnFillFormula } from "./lib/columnFillFormula";
import type { FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type { GridContext, GridFocus } from "./FrameGrid";
import type { Column, Selection, FrameObject } from "./lib/types";

export function CellAwareFormulaBar({
  context,
  focus,
  references,
  cellReferences,
  cellFormulaRequest,
  expanded,
  onCommit,
  onOperation,
  onEditCalculated,
  onTransformColumn,
  onReadOnly,
  onToggle,
  onCellFormulaSaved,
}: {
  context: GridContext | null;
  focus: GridFocus | null;
  references: FormulaReference[];
  cellReferences: FormulaReference[];
  cellFormulaRequest: CellFormulaRequest | null;
  expanded: boolean;
  onCommit: (formula: string) => Promise<ScratchworkFormulaFeedback>;
  onOperation: OperationHandler;
  onEditCalculated: (
    frame: FrameObject,
    column: Column,
    rowIndex: number,
    viewId?: string
  ) => void;
  onTransformColumn: (
    frame: FrameObject,
    column: Column,
    formula: string,
    rowIndex: number,
    viewId?: string
  ) => void;
  onReadOnly: (selection: Selection, reason: string) => void;
  onToggle: () => void;
  onCellFormulaSaved: () => void;
}) {
  const cell = context && focus ? formulaBarCell(context, focus) : null;
  const [formulaRequest, setFormulaRequest] = useState<CellFormulaRequest | null>(
    null
  );
  const localRequest = useRef(0);

  useEffect(() => {
    setFormulaRequest(
      cellFormulaRequest?.cellId === cell?.id ? cellFormulaRequest : null
    );
  }, [cell?.id, cellFormulaRequest]);

  const editCellFormula = () => {
    if (!cell) return;
    localRequest.current += 1;
    setFormulaRequest({
      key: `bar:${localRequest.current}`,
      cellId: cell.id,
      seed: null,
    });
  };
  // New one-cell formulas deliberately go to Scratchwork. Keep the editor
  // mounted only for an override already stored by an older document, so it
  // can still be inspected, corrected, or removed without reviving that
  // scope as a creation path.
  const formulaCell = cell?.kind === "override" ? cell : null;
  return (
    <>
      {formulaCell && (
        <CellFormulaController
          cell={formulaCell}
          references={cellReferences}
          request={formulaRequest}
          onOperation={onOperation}
          onSaved={onCellFormulaSaved}
          orderingDeclared={Boolean(
            context?.computed?.steps?.some((step) => step.kind === "sort") ||
              context?.frame.display?.steps?.some((step) => step.kind === "sort")
          )}
        />
      )}
      <ScratchworkFormulaBar
        onCommit={onCommit}
        references={references}
        cell={cell}
        onCommitCell={(selected, raw) => {
          const formula = columnFillFormula(raw);
          const frame = context?.frame.id === selected.frameId ? context.frame : null;
          const column = frame?.columns.find(
            (candidate) => candidate.id === selected.columnId
          );
          if (formula && frame && column) {
            onTransformColumn(frame, column, formula, selected.rowIndex, focus?.viewId);
            return Promise.resolve(null);
          }
          return onOperation(
            {
              type: "setCells",
              frameId: selected.frameId,
              cells: [{ rowId: selected.rowId, columnId: selected.columnId, raw }],
            },
            { inlineError: true }
          );
        }}
        onEditCalculatedCell={(selected) => {
          const frame =
            context?.frame.id === selected.frameId ? context.frame : null;
          const column = frame?.columns.find(
            (candidate) => candidate.id === selected.columnId
          );
          if (frame && column)
            onEditCalculated(frame, column, selected.rowIndex, focus?.viewId);
        }}
        onEditOverrideCell={editCellFormula}
        onRequestReadOnlyCell={(selected) =>
          onReadOnly(
            {
              objectId: selected.frameId,
              viewId: focus?.viewId,
              rowId: selected.rowId,
              columnId: selected.columnId,
            },
            selected.reason ?? "This value is read-only."
          )
        }
        expanded={expanded}
        onToggle={onToggle}
      />
    </>
  );
}
