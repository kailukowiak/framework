import { useCallback, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { CellFormulaRequest } from "../CellFormulaController";
import type { GridFocus, RenderedGrid } from "../FrameGrid";
import {
  ARROW_DIRECTIONS,
  gridBoundsFor,
  gridRangeForFocus,
  isEditableGridColumn,
  isVisualCellEmpty,
  resolveGridContext,
  visualGridPosition,
} from "../FrameGrid";
import { handleGridFormulaKey } from "../GridFormulaKeyboard";
import type { TransformColumnRequest } from "./usePipelineColumnRequests";
import { clearGridRangeUpdates, fillGridRangeUpdates } from "../lib/gridEditing";
import { movedGridFocus, rangedGridFocus, type GridFocusDestination } from "../lib/gridFocusNavigation";
import {
  contiguousDataRange,
  documentEdgePosition,
  enterPosition,
  fullColumnRange,
  fullGridRange,
  fullRowRange,
  isPrintableKey,
  jumpPosition,
  pagePosition,
  rowEdgePosition,
  sameRange,
  stepPosition,
  tabPosition,
  type GridPosition,
  type GridRange,
} from "../lib/gridNavigation";
import type { OperationHandler } from "../lib/handlers";
import type { DocumentView, Selection } from "../lib/types";
import type { InspectorSection } from "../App";

/**
 * Keyboard handling for a focused grid cell: arrow/tab/enter movement,
 * range selection, fill-down/right, delete/backspace, and starting an
 * edit. Sits alongside (not inside) the window-level shortcut dispatcher —
 * it only runs once a cell already has keyboard focus.
 */
export function useGridKeyboardNavigation({
  document,
  gridFocus,
  renderedRows,
  cellFormulaToken,
  transformColumnToken,
  setCellFormulaRequest,
  setTransformColumnRequest,
  clearActiveFormulaEditor,
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
  clearActiveFormulaEditor: () => void;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  setInspectorSection: Dispatch<SetStateAction<InspectorSection>>;
  run: OperationHandler;
}) {
  return useCallback(
    (event: KeyboardEvent) => {
      if (!document || !gridFocus) return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      if (!context) return;
      const bounds = gridBoundsFor(context);
      const position = visualGridPosition(context, gridFocus.rowId, gridFocus.columnId);
      if (!position) return;
      const column = context.frame.columns.find(
        (candidate) => candidate.id === gridFocus.columnId
      );
      const editable = Boolean(
        column && isEditableGridColumn(context.computed, column, context.frame)
      );
      if (
        handleGridFormulaKey({
          event,
          context,
          focus: gridFocus,
          column,
          cellToken: cellFormulaToken,
          columnToken: transformColumnToken,
          onCellRequest: setCellFormulaRequest,
          onColumnRequest: setTransformColumnRequest,
          onScratchworkRequest: () => {
            clearActiveFormulaEditor();
            setGridFocus(null);
            setSelection(null);
            window.requestAnimationFrame(() => {
              window.document
                .querySelector<HTMLTextAreaElement>(".scratchwork-formula-bar textarea")
                ?.focus();
            });
          },
          onSelect: setSelection,
          onOpenWrangle: () => setInspectorSection("wrangle"),
          onOperation: run,
        })
      )
        return;
      const modifier = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();

      const arrive = (destination: GridFocusDestination | null) => {
        if (!destination) return;
        setGridFocus(destination.focus);
        setSelection(destination.selection);
      };
      const moveTo = (next: GridPosition, extend: boolean) =>
        arrive(movedGridFocus(context, gridFocus, next, extend));
      const selectRange = (range: GridRange, span: GridFocus["span"] = null) =>
        arrive(rangedGridFocus(context, gridFocus, range, span));

      const setCells = (
        updates: Array<{ rowId: string; columnId: string; raw: string }>
      ) => {
        if (updates.length)
          void run({ type: "setCells", frameId: context.frame.id, cells: updates });
      };

      const direction = ARROW_DIRECTIONS[event.key];
      if (modifier && key === "a") {
        event.preventDefault();
        const region = contiguousDataRange(position, bounds, (candidate) =>
          isVisualCellEmpty(context, candidate)
        );
        const current = gridRangeForFocus(context, gridFocus);
        selectRange(
          gridFocus.anchor && current && sameRange(current, region)
            ? fullGridRange(bounds)
            : region
        );
      } else if (event.shiftKey && event.key === " ") {
        event.preventDefault();
        selectRange(fullRowRange(position.row, bounds), "row");
      } else if (modifier && event.key === " ") {
        event.preventDefault();
        selectRange(fullColumnRange(position.col, bounds), "column");
      } else if (modifier && key === "d") {
        event.preventDefault();
        setCells(fillGridRangeUpdates(context, gridFocus, "down"));
      } else if (modifier && key === "r") {
        event.preventDefault();
        setCells(fillGridRangeUpdates(context, gridFocus, "right"));
      } else if (direction) {
        event.preventDefault();
        const next = modifier
          ? jumpPosition(position, direction, bounds, (candidate) =>
              isVisualCellEmpty(context, candidate)
            )
          : stepPosition(position, direction, bounds);
        moveTo(next, event.shiftKey);
      } else if (event.key === "Tab") {
        event.preventDefault();
        moveTo(tabPosition(position, event.shiftKey, bounds), false);
      } else if (event.key === "Enter") {
        event.preventDefault();
        moveTo(enterPosition(position, event.shiftKey, bounds), false);
      } else if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        const edge = event.key === "Home" ? "home" : "end";
        moveTo(
          modifier
            ? documentEdgePosition(edge, bounds, (candidate) =>
                isVisualCellEmpty(context, candidate)
              )
            : rowEdgePosition(position, edge, bounds),
          event.shiftKey
        );
      } else if (event.key === "PageUp" || event.key === "PageDown") {
        event.preventDefault();
        moveTo(
          pagePosition(
            position,
            event.key === "PageUp" ? "up" : "down",
            context.viewportRows,
            bounds
          ),
          event.shiftKey
        );
      } else if (event.key === "Escape") {
        event.preventDefault();
        setGridFocus(null);
        setSelection({ objectId: gridFocus.objectId, viewId: gridFocus.viewId });
      } else if (event.key === "F2") {
        if (!editable) return;
        event.preventDefault();
        setGridFocus({ ...gridFocus, mode: "edit", editSeed: null, anchor: null });
      } else if (event.key === "Delete") {
        event.preventDefault();
        setCells(clearGridRangeUpdates(context, gridFocus));
      } else if (event.key === "Backspace") {
        event.preventDefault();
        setCells(clearGridRangeUpdates(context, gridFocus));
      } else if (isPrintableKey(event)) {
        if (!editable) return;
        event.preventDefault();
        setGridFocus({
          ...gridFocus,
          mode: "edit",
          editSeed: column!.dataType === "categorical" ? null : event.key,
          anchor: null,
        });
      }
    },
    [
      clearActiveFormulaEditor,
      document,
      gridFocus,
      run,
      setTransformColumnRequest,
      transformColumnToken,
    ]
  );
}
