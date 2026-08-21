import { ChevronRight, Sparkles, Workflow } from "lucide-react";
import { ColumnQuickActions } from "./ColumnQuickActions";
import type {
  RecurrenceState,
  RunningCalculationState,
  SequenceFillState,
} from "./ColumnAuthoringDialogs";
import type { GridContext } from "./FrameGrid";
import { formulaToken } from "./lib/formulaReferences";
import type { Column, FrameObject } from "./lib/types";

const numericTypes = new Set(["integer", "number", "currency", "percentage"]);

export function ColumnContextAuthoringActions({
  frame,
  column,
  grid,
  rowId,
  viewId,
  onTransform,
  onEdit,
  onRunning,
  onRecurrence,
  onSequence,
  compact = false,
}: {
  frame: FrameObject;
  column: Column | null;
  grid: GridContext | null;
  rowId?: string;
  viewId?: string;
  onTransform: (formula: string, focus?: boolean) => void;
  onEdit: () => void;
  onRunning: (state: RunningCalculationState) => void;
  onRecurrence: (state: RecurrenceState) => void;
  onSequence: (state: SequenceFillState) => void;
  /** A cell menu keeps column-wide operations behind one clear boundary. */
  compact?: boolean;
}) {
  if (!column) return null;
  const numeric = numericTypes.has(column.dataType);
  const startRaw =
    grid?.displayedRows
      .find((row) => row.id === rowId)
      ?.cells[column.id]?.raw?.trim() ?? "";
  const start = Number(startRaw);
  const actions = (
    <>
      <ColumnQuickActions
        column={column}
        onTransform={(formula) => onTransform(formula)}
      />
      <button
        onClick={() =>
          onRecurrence({
            frameId: frame.id,
            targetColumnId: column.id,
            viewId,
            initialSourceColumnId: numeric ? column.id : undefined,
          })
        }
      >
        <Workflow size={14} />
        <span>Calculate down rows…</span>
      </button>
      {numeric && (
        <button
          onClick={() =>
            onRunning({
              frameId: frame.id,
              targetColumnId: column.id,
              viewId,
              initialSourceColumnId: column.id,
            })
          }
        >
          <Workflow size={14} />
          <span>Running calculation…</span>
        </button>
      )}
      {numeric && (
        <button
          onClick={() =>
            onSequence({
              frameId: frame.id,
              columnId: column.id,
              columnName: column.name,
              viewId,
              initialStart: startRaw && Number.isInteger(start) ? start : 1,
              initialStep: 1,
              kind: "number",
            })
          }
        >
          <Sparkles size={14} />
          <span>Fill number series…</span>
        </button>
      )}
      <button
        onClick={() =>
          column.formula ? onEdit() : onTransform(formulaToken(column.name), true)
        }
      >
        <Workflow size={14} />
        <span>
          {column.formula ? "Edit column formula" : "Transform column in Wrangle"}
        </span>
      </button>
    </>
  );
  return compact ? (
    <details className="context-menu-submenu">
      <summary>
        <Workflow size={14} />
        <span>Column calculations</span>
        <ChevronRight className="submenu-chevron" size={14} />
      </summary>
      <div>{actions}</div>
    </details>
  ) : (
    actions
  );
}
