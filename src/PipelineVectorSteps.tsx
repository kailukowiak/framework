import { useEffect, useRef } from "react";
import { PipelineCommand } from "./PipelineCommand";
import { meltedColumnIds } from "./lib/columnList";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import type {
  BroadcastOperator,
  Column,
  FrameStepInput,
  RenderedFrameStep,
} from "./lib/types";
import { formattedFormula } from "./PipelineFormulaFormatting";

export type BroadcastStepDraft = {
  id: string;
  kind: "broadcast";
  columns: string;
  vector: string;
  operator: BroadcastOperator;
  expectedLength?: number;
};

export type ZipVectorStepDraft = {
  id: string;
  kind: "zipVector";
  outputColumnId: string;
  name: string;
  vector: string;
  expectedLength?: number;
};

export type VectorStepDraft = BroadcastStepDraft | ZipVectorStepDraft;

export function blankVectorStep(
  kind: string,
  id: string,
  mintColumnId: (name: string) => string
): VectorStepDraft | null {
  if (kind === "broadcast")
    return { id, kind, columns: "", vector: "", operator: "multiply" };
  return kind === "zipVector"
    ? {
        id,
        kind,
        outputColumnId: mintColumnId("Column"),
        name: "Column",
        vector: "",
      }
    : null;
}

export function vectorDraftFromRendered(
  step: RenderedFrameStep,
  id: string,
  visible: Array<{ id: string; name: string }>
): VectorStepDraft | null {
  if (step.kind === "broadcast")
    return {
      id,
      kind: "broadcast",
      columns: step.columnIds
        .map((columnId) =>
          formulaToken(
            visible.find((candidate) => candidate.id === columnId)?.name ?? columnId
          )
        )
        .join(", "),
      vector: formattedFormula(step.vector),
      operator: step.operator,
      expectedLength: step.expectedLength,
    };
  return step.kind === "zipVector"
    ? {
        id,
        kind: "zipVector",
        outputColumnId: step.outputColumnId,
        name: step.outputColumnName,
        vector: formattedFormula(step.vector),
        expectedLength: step.expectedLength,
      }
    : null;
}

export function vectorStepInput(step: VectorStepDraft): FrameStepInput {
  return step.kind === "broadcast"
    ? {
        kind: "broadcast",
        columns: step.columns,
        vector: step.vector,
        operator: step.operator,
      }
    : {
        kind: "zipVector",
        outputColumnId: step.outputColumnId,
        name: step.name,
        vector: step.vector,
      };
}

export function vectorStepIsIncomplete(step: VectorStepDraft): boolean {
  return step.kind === "broadcast"
    ? !step.columns.trim() || !step.vector.trim()
    : !step.name.trim() || !step.vector.trim();
}

export function vectorStepFormulas(step: VectorStepDraft): string[] {
  return step.kind === "broadcast" ? [step.columns, step.vector] : [step.vector];
}

export function BroadcastStepRow({
  step,
  visible,
  columnReferences,
  references,
  columnsEditorId,
  vectorEditorId,
  columnsFocusToken,
  vectorFocusToken,
  onUpdate,
}: {
  step: BroadcastStepDraft;
  visible: Array<{ id: string; name: string }>;
  columnReferences: FormulaReference[];
  references: FormulaReference[];
  columnsEditorId: string;
  vectorEditorId: string;
  columnsFocusToken?: number;
  vectorFocusToken?: number;
  onUpdate: (change: Partial<BroadcastStepDraft>, saveNow: boolean) => void;
}) {
  const count = meltedColumnIds(step.columns, visible).length;
  const repeats =
    step.expectedLength && count % step.expectedLength === 0
      ? count / step.expectedLength
      : undefined;
  return (
    <div className="pipeline-broadcast">
      <PipelineCommand
        editorId={columnsEditorId}
        label="Columns"
        initialDraft={step.columns}
        references={columnReferences}
        focusToken={columnsFocusToken}
        onChange={(columns) => onUpdate({ columns }, false)}
        onCommit={(columns) => onUpdate({ columns }, true)}
      />
      <label className="pipeline-broadcast-operator">
        <span>Apply</span>
        <select
          aria-label="Vector operation"
          value={step.operator}
          onChange={(event) =>
            onUpdate({ operator: event.target.value as BroadcastOperator }, true)
          }
        >
          <option value="multiply">×</option>
          <option value="divide">÷</option>
          <option value="add">+</option>
          <option value="subtract">−</option>
        </select>
      </label>
      <PipelineCommand
        editorId={vectorEditorId}
        label="Vector"
        initialDraft={step.vector}
        references={references}
        focusToken={vectorFocusToken}
        onChange={(vector) => onUpdate({ vector }, false)}
        onCommit={(vector) => onUpdate({ vector }, true)}
      />
      {step.expectedLength !== undefined && (
        <small>
          {step.expectedLength} value{step.expectedLength === 1 ? "" : "s"}
          {repeats && repeats > 1 ? ` × ${repeats}` : ""}
        </small>
      )}
    </div>
  );
}

export function ZipVectorStepRow({
  step,
  references,
  editorId,
  focusToken,
  onDraft,
}: {
  step: ZipVectorStepDraft;
  references: FormulaReference[];
  editorId: string;
  focusToken?: number;
  onDraft: (draft: string, saveNow: boolean) => void;
}) {
  return (
    <div className="pipeline-zip-vector">
      <PipelineCommand
        editorId={editorId}
        label="Column"
        initialDraft={`${formulaToken(step.name)} = ${step.vector}`}
        references={references}
        focusToken={focusToken}
        onChange={(draft) => onDraft(draft, false)}
        onCommit={(draft) => onDraft(draft, true)}
      />
      {step.expectedLength !== undefined && (
        <small>{step.expectedLength} rows, paired by position</small>
      )}
    </div>
  );
}

type VectorRequest = {
  token: number;
  vector: string;
  expectedLength: number;
};

export function useVectorStepRequests<T>({
  applyRequest,
  pairRequest,
  columns,
  steps,
  persist,
  onApplyHandled,
  onPairHandled,
  mintColumnId,
  uniqueColumnName,
}: {
  applyRequest?: VectorRequest & { columnIds: string[] };
  pairRequest?: VectorRequest & { name: string };
  columns: Column[];
  steps: T[];
  persist: (steps: Array<T | VectorStepDraft>) => void;
  onApplyHandled?: () => void;
  onPairHandled?: () => void;
  mintColumnId: (name: string) => string;
  uniqueColumnName: (name: string, used: string[]) => string;
}) {
  const handledApply = useRef<number | null>(null);
  const handledPair = useRef<number | null>(null);
  useEffect(() => {
    if (!applyRequest || handledApply.current === applyRequest.token) return;
    handledApply.current = applyRequest.token;
    onApplyHandled?.();
    const selected = applyRequest.columnIds
      .map((id) => columns.find((column) => column.id === id)?.name)
      .filter((name): name is string => Boolean(name))
      .map(formulaToken)
      .join(", ");
    if (selected)
      persist([
        ...steps,
        {
          id: crypto.randomUUID(),
          kind: "broadcast",
          columns: selected,
          vector: applyRequest.vector,
          operator: "multiply",
          expectedLength: applyRequest.expectedLength,
        },
      ]);
  }, [applyRequest, columns, onApplyHandled, persist, steps]);
  useEffect(() => {
    if (!pairRequest || handledPair.current === pairRequest.token) return;
    handledPair.current = pairRequest.token;
    onPairHandled?.();
    const name = uniqueColumnName(
      pairRequest.name || "Column",
      columns.map((column) => column.name)
    );
    persist([
      ...steps,
      {
        id: crypto.randomUUID(),
        kind: "zipVector",
        outputColumnId: mintColumnId(name),
        name,
        vector: pairRequest.vector,
        expectedLength: pairRequest.expectedLength,
      },
    ]);
  }, [
    columns,
    mintColumnId,
    onPairHandled,
    pairRequest,
    persist,
    steps,
    uniqueColumnName,
  ]);
}
