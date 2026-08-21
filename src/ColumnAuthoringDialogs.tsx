import { RunningCalculationDialog } from "./RunningCalculationDialog";
import { RecurrenceDialog } from "./RecurrenceDialog";
import { SequenceFillDialog } from "./SequenceFillDialog";
import type { Column, DocumentView, FrameObject } from "./lib/types";

export type SequenceFillState = {
  frameId: string;
  columnId: string;
  columnName: string;
  viewId?: string;
  initialStart: number | string;
  initialStep?: number;
  kind?: "number" | "date";
  dateUnit?: "d" | "mo";
};

export type RunningCalculationState = {
  frameId: string;
  targetColumnId: string;
  viewId?: string;
  initialSourceColumnId?: string;
};

export type RecurrenceState = {
  frameId: string;
  targetColumnId: string;
  viewId?: string;
  initialSourceColumnId?: string;
};

type Transform = (
  frame: FrameObject,
  column: Column,
  formula: string,
  viewId?: string,
  orderByColumnId?: string
) => void;

export function ColumnAuthoringDialogs({
  document,
  sequence,
  running,
  recurrence,
  onCloseSequence,
  onCloseRunning,
  onCloseRecurrence,
  onTransform,
}: {
  document: DocumentView;
  sequence: SequenceFillState | null;
  running: RunningCalculationState | null;
  recurrence: RecurrenceState | null;
  onCloseSequence: () => void;
  onCloseRunning: () => void;
  onCloseRecurrence: () => void;
  onTransform: Transform;
}) {
  const frame = (id: string | undefined) =>
    document.objects.find(
      (object): object is FrameObject => object.kind === "frame" && object.id === id
    );
  const sequenceFrame = frame(sequence?.frameId);
  const sequenceColumn = sequenceFrame?.columns.find(
    (column) => column.id === sequence?.columnId
  );
  const runningFrame = frame(running?.frameId);
  const runningTarget = runningFrame?.columns.find(
    (column) => column.id === running?.targetColumnId
  );
  const recurrenceFrame = frame(recurrence?.frameId);
  const recurrenceTarget = recurrenceFrame?.columns.find(
    (column) => column.id === recurrence?.targetColumnId
  );
  const ordered = (frameId: string) =>
    Boolean(
      document.computedFrames[frameId]?.steps?.some((step) => step.kind === "sort")
    );
  return (
    <>
      {sequence && sequenceFrame && sequenceColumn && (
        <SequenceFillDialog
          columnName={sequence.columnName}
          orderColumns={sequenceFrame.columns.map((column) => ({
            id: column.id,
            name: column.name,
          }))}
          alreadyOrdered={ordered(sequenceFrame.id)}
          initialStart={sequence.initialStart}
          initialStep={sequence.initialStep}
          kind={sequence.kind}
          dateUnit={sequence.dateUnit}
          onCancel={onCloseSequence}
          onApply={(formula, orderByColumnId) => {
            onTransform(
              sequenceFrame,
              sequenceColumn,
              formula,
              sequence.viewId,
              orderByColumnId
            );
            onCloseSequence();
          }}
        />
      )}
      {running && runningFrame && runningTarget && (
        <RunningCalculationDialog
          targetName={runningTarget.name}
          columns={runningFrame.columns}
          initialSourceColumnId={running.initialSourceColumnId ?? runningTarget.id}
          alreadyOrdered={ordered(runningFrame.id)}
          onCancel={onCloseRunning}
          onApply={(formula, orderByColumnId) => {
            onTransform(
              runningFrame,
              runningTarget,
              formula,
              running.viewId,
              orderByColumnId
            );
            onCloseRunning();
          }}
        />
      )}
      {recurrence && recurrenceFrame && recurrenceTarget && (
        <RecurrenceDialog
          target={recurrenceTarget}
          columns={recurrenceFrame.columns}
          initialSourceColumnId={recurrence.initialSourceColumnId}
          alreadyOrdered={ordered(recurrenceFrame.id)}
          onCancel={onCloseRecurrence}
          onApply={(formula, orderByColumnId) => {
            onTransform(
              recurrenceFrame,
              recurrenceTarget,
              formula,
              recurrence.viewId,
              orderByColumnId
            );
            onCloseRecurrence();
          }}
        />
      )}
    </>
  );
}
