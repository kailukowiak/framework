import {
  Braces,
  FunctionSquare,
  KeyRound,
  Table2 as FrameIcon,
} from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { ColumnContextAuthoringActions } from "./ColumnContextAuthoringActions";
import { InferredSeriesMenuAction } from "./InferredSeriesMenuAction";
import { PinColumnsMenuItem } from "./PinColumnsMenuItem";
import type { ContextMenuState, GridContext } from "./FrameGrid";
import { nextEntryColumnName } from "./hooks/useCanvasObjectCreation";
import type { ContextGeneratorInference } from "./lib/contextGeneratorInference";
import { formulaToken } from "./lib/formulaReferences";
import type {
  RecurrenceState,
  RunningCalculationState,
  SequenceFillState,
} from "./ColumnAuthoringDialogs";
import type {
  Column,
  DocumentView,
  FrameObject,
  Operation,
} from "./lib/types";

export type ContextMenuColumnItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  contextColumn: Column | null;
  contextGrid: GridContext | null;
  contextGenerator: ContextGeneratorInference | null;
  contextEntryKey: string[] | null;
  requestAddCalculatedColumn: (
    frameId: string,
    afterColumnId: string | undefined,
    anchorRowIndex: number | undefined,
    viewId?: string
  ) => void;
  requestCalculatedColumnEdit: (
    frame: FrameObject,
    column: Column,
    rowIndex?: number,
    viewId?: string
  ) => void;
  requestColumnTransformation: (
    frame: FrameObject,
    column: Column,
    formula: string,
    focus?: boolean,
    viewId?: string,
    orderByColumnId?: string
  ) => void;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  setRecurrence: Dispatch<SetStateAction<RecurrenceState | null>>;
  setRunningCalculation: Dispatch<SetStateAction<RunningCalculationState | null>>;
  setSequenceFill: Dispatch<SetStateAction<SequenceFillState | null>>;
};

/**
 * Writing and filling a column: the authoring actions for the column under
 * the pointer, a new calculated column, the series a selected run implies,
 * a keyed entry column, and which columns stay pinned.
 */
export function ContextMenuColumnItems({
  contextMenu,
  contextFrame,
  contextColumn,
  contextGrid,
  contextGenerator,
  contextEntryKey,
  requestAddCalculatedColumn,
  requestCalculatedColumnEdit,
  requestColumnTransformation,
  run,
  setContextMenu,
  setRecurrence,
  setRunningCalculation,
  setSequenceFill,
}: ContextMenuColumnItemsProps) {
  return (
    <>
      <ColumnContextAuthoringActions
        frame={contextFrame}
        column={contextColumn}
        grid={contextGrid}
        rowId={contextMenu.rowId}
        viewId={contextMenu.viewId}
        onTransform={(formula, focus) =>
          contextColumn &&
          requestColumnTransformation(
            contextFrame,
            contextColumn,
            formula,
            focus,
            contextMenu.viewId
          )
        }
        onEdit={() =>
          contextColumn &&
          requestCalculatedColumnEdit(
            contextFrame,
            contextColumn,
            contextMenu.rowIndex,
            contextMenu.viewId
          )
        }
        onRunning={(state) => {
          setContextMenu(null);
          setRunningCalculation(state);
        }}
        onRecurrence={(state) => {
          setContextMenu(null);
          setRecurrence(state);
        }}
        onSequence={(state) => {
          setContextMenu(null);
          setSequenceFill(state);
        }}
        compact={contextMenu.rowId !== undefined}
      />
      <button
        onClick={() =>
          requestAddCalculatedColumn(
            contextFrame.id,
            contextColumn?.id,
            contextMenu.rowIndex,
            contextMenu.viewId
          )
        }
      >
        <FunctionSquare size={14} />
        <span>
          {contextMenu.rowIndex === undefined
            ? "Add calculated column"
            : "Formula here"}
        </span>
      </button>
      {contextGenerator && contextColumn && (
        <InferredSeriesMenuAction
          frame={contextFrame}
          column={contextColumn}
          inference={contextGenerator}
          viewId={contextMenu.viewId}
          x={contextMenu.canvasX}
          y={contextMenu.canvasY}
          onClose={() => setContextMenu(null)}
          onFill={setSequenceFill}
          onOperation={run}
        />
      )}
      {contextEntryKey && (
        <button
          onClick={() => {
            setContextMenu(null);
            void run({
              type: "addEntryColumn",
              frameId: contextFrame.id,
              name: nextEntryColumnName(contextFrame),
              dataType: "number",
              keyColumnIds: contextEntryKey,
            });
          }}
        >
          <KeyRound size={14} />
          <span>Add entry column (keyed)</span>
        </button>
      )}
      <PinColumnsMenuItem
        frame={contextFrame}
        column={contextColumn}
        onPin={(pinnedColumns) => {
          setContextMenu(null);
          void run({
            type: "setFrameDisplayPinnedColumns",
            frameId: contextFrame.id,
            pinnedColumns,
          });
        }}
      />
    </>
  );
}

export type ContextMenuColumnDisplayItemsProps = {
  contextMenu: ContextMenuState;
  contextFrame: FrameObject;
  contextColumn: Column | null;
  contextCrosstabValues: string | null;
  contextIsMaterialized: boolean;
  document: DocumentView;
  copyColumnReference: (
    frame: FrameObject,
    column: Column,
    alreadyMaterialized: boolean
  ) => Promise<void>;
  requestColumnTransformation: (
    frame: FrameObject,
    column: Column,
    formula: string,
    focus?: boolean,
    viewId?: string,
    orderByColumnId?: string
  ) => void;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
};

/**
 * How a column reads rather than what it holds: spread across columns or
 * back to rows, converted to another type, or quoted as a reference another
 * frame can read.
 */
export function ContextMenuColumnDisplayItems({
  contextMenu,
  contextFrame,
  contextColumn,
  contextCrosstabValues,
  contextIsMaterialized,
  document,
  copyColumnReference,
  requestColumnTransformation,
  run,
  setContextMenu,
}: ContextMenuColumnDisplayItemsProps) {
  return (
    <>
      {contextColumn && contextCrosstabValues && (
        <button
          onClick={() => {
            setContextMenu(null);
            void run({
              type: "setFrameDisplayCrosstab",
              frameId: contextFrame.id,
              crosstab: {
                namesColumnId: contextColumn.id,
                valuesColumnId: contextCrosstabValues,
              },
            });
          }}
        >
          <FrameIcon size={14} />
          <span>Spread across columns</span>
        </button>
      )}
      {contextFrame.display?.crosstab && (
        <button
          onClick={() => {
            setContextMenu(null);
            void run({
              type: "setFrameDisplayCrosstab",
              frameId: contextFrame.id,
              crosstab: null,
            });
          }}
        >
          <FrameIcon size={14} />
          <span>Back to rows</span>
        </button>
      )}
      {contextColumn &&
        !document.computedFrames[contextFrame.id]?.editing.rows && (
          <label className="context-menu-field">
            <span>Convert column type</span>
            <select
              value={contextColumn.dataType}
              onChange={(event) =>
                requestColumnTransformation(
                  contextFrame,
                  contextColumn,
                  `${formulaToken(contextColumn.name)}.cast("${event.target.value}")`,
                  false,
                  contextMenu.viewId
                )
              }
            >
              <option value="string">Text</option>
              <option value="categorical" disabled>
                Categorical
              </option>
              <option value="integer">Integer</option>
              <option value="number">Number</option>
              <option value="currency" disabled>
                Currency
              </option>
              <option value="percentage" disabled>
                Percentage
              </option>
              <option value="boolean">Boolean</option>
              <option value="date">Date</option>
            </select>
          </label>
        )}
      {/* Reading a column from another frame needs that frame to
          hold a snapshot, which is one action away — so this is one
          action, not two, and it says which one it is doing. */}
      {contextColumn && (contextFrame.derivation || contextIsMaterialized) && (
        <button
          onClick={() => {
            setContextMenu(null);
            void copyColumnReference(
              contextFrame,
              contextColumn,
              contextIsMaterialized
            );
          }}
        >
          <Braces size={14} />
          <span>
            {contextIsMaterialized
              ? "Copy reference to this column"
              : "Materialize and copy reference"}
          </span>
        </button>
      )}
    </>
  );
}
