import { useCallback, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { CellFormulaRequest } from "../CellFormulaController";
import type { GridContext, GridFocus, RenderedGrid } from "../FrameGrid";
import { handleGridFormulaKey } from "../GridFormulaKeyboard";
import type { InspectorSection } from "../Inspector";
import type { OperationHandler } from "../lib/handlers";
import type { DocumentView, Selection } from "../lib/types";
import { handleGridEdgeKey } from "./gridStructureKeys";
import { useGridEdgeGrowth } from "./useGridEdgeGrowth";
import type { TransformColumnRequest } from "./usePipelineColumnRequests";

/**
 * The keys that act on the grid rather than move through it: the formula
 * chords (open a cell or column formula) and the edge keys (structure chords,
 * and Return, Tab and Right-arrow growing the grid at its last row or column).
 *
 * Gathered into one handler so the navigation hook, which is well past its
 * complexity budget, asks a single question — "was this a chord?" — before it
 * falls through to plain movement and editing. Returns whether the key was
 * taken.
 */
export function useGridChordKeys({
  document,
  gridFocus,
  renderedRows,
  cellFormulaToken,
  transformColumnToken,
  setCellFormulaRequest,
  setTransformColumnRequest,
  setGridFocus,
  setSelection,
  setInspectorSection,
  run,
}: {
  document: DocumentView | null;
  gridFocus: GridFocus | null;
  renderedRows: MutableRefObject<Map<string, RenderedGrid>>;
  cellFormulaToken: MutableRefObject<number>;
  transformColumnToken: MutableRefObject<number>;
  setCellFormulaRequest: Dispatch<SetStateAction<CellFormulaRequest | null>>;
  setTransformColumnRequest: Dispatch<SetStateAction<TransformColumnRequest>>;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  setInspectorSection: Dispatch<SetStateAction<InspectorSection>>;
  run: OperationHandler;
}) {
  const { growAtEdge } = useGridEdgeGrowth({
    document,
    gridFocus,
    renderedRows,
    setGridFocus,
    setSelection,
    run,
  });
  return useCallback(
    (event: KeyboardEvent, context: GridContext, focus: GridFocus): boolean => {
      const column = context.frame.columns.find(
        (candidate) => candidate.id === focus.columnId
      );
      return (
        handleGridFormulaKey({
          event,
          context,
          focus,
          column,
          cellToken: cellFormulaToken,
          columnToken: transformColumnToken,
          onCellRequest: setCellFormulaRequest,
          onColumnRequest: setTransformColumnRequest,
          onSelect: setSelection,
          onOpenWrangle: () => setInspectorSection("wrangle"),
          onOperation: run,
        }) ||
        handleGridEdgeKey({
          event,
          context,
          focus,
          run,
          setGridFocus,
          setSelection,
          growAtEdge,
        })
      );
    },
    [
      cellFormulaToken,
      transformColumnToken,
      setCellFormulaRequest,
      setTransformColumnRequest,
      setSelection,
      setInspectorSection,
      run,
      setGridFocus,
      growAtEdge,
    ]
  );
}
