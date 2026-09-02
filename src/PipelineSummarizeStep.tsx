import { Plus } from "lucide-react";
import { PipelineCommand } from "./PipelineCommand";
import {
  draftName,
  parseNamedTransformation,
  type NamedDraft,
} from "./PipelineColumnNames";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import { namedCommand } from "./lib/pipelineStepCommands";
import { namedDraft, type StepDraft, type VisibleColumn } from "./lib/pipelineSteps";
import type { Column, FrameStepInput } from "./lib/types";

/** The group keys and aggregates of one Summarize step. */
export function PipelineSummarizeStep({
  step,
  visible,
  stepReferences,
  input,
  scope,
  commandId,
  commandFocus,
  patch,
  commitPatch,
  savePatch,
  rejectCommand,
  setPendingEditor,
}: {
  step: Extract<StepDraft, { kind: "summarize" }>;
  visible: VisibleColumn[];
  stepReferences: FormulaReference[];
  input: { label: string; columns: Column[]; completionFrameId?: string };
  scope: { steps: FrameStepInput[]; stepIndex: number };
  commandId: (step: StepDraft, itemId?: string) => string;
  commandFocus: (id: string, requested?: number) => number | undefined;
  patch: (stepId: string, update: (step: StepDraft) => StepDraft) => void;
  commitPatch: (
    stepId: string,
    update: (step: StepDraft) => StepDraft
  ) => Promise<void>;
  savePatch: (
    stepId: string,
    update: (step: StepDraft) => StepDraft
  ) => Promise<string | null>;
  rejectCommand: (message: string) => never;
  setPendingEditor: (editorId: string) => void;
}) {
  return (
    <div className="pipeline-command-list">
      {[
        ...step.groupKeys.map((item) => ({ item, detail: "group" })),
        ...step.aggregates.map((item) => ({ item, detail: "value" })),
      ].map(({ item, detail }) => {
        const id = commandId(step, item.id);
        const update = (draft: string, saveNow: boolean) => {
          const parsed = parseNamedTransformation(draft);
          if (!parsed) {
            if (saveNow)
              rejectCommand(
                "Write a backticked output name, =, and a formula"
              );
            return;
          }
          const change = (current: StepDraft): StepDraft => {
            if (current.kind !== "summarize") return current;
            const revise = (candidate: NamedDraft) =>
              candidate.id === item.id
                ? { ...candidate, name: parsed.name, formula: parsed.formula }
                : candidate;
            return {
              ...current,
              groupKeys: current.groupKeys.map(revise),
              aggregates: current.aggregates.map(revise),
            };
          };
          if (saveNow) return commitPatch(step.id, change);
          patch(step.id, change);
        };
        return (
          <PipelineCommand
            key={item.id}
            editorId={id}
            label={`${
              detail === "group" ? "Group" : "Aggregate"
            }: ${draftName(item)}`}
            detail={detail}
            initialDraft={namedCommand(draftName(item), item.formula)}
            references={stepReferences}
            frameId={input.completionFrameId}
            scope={scope}
            focusToken={commandFocus(id)}
            onChange={(draft) => update(draft, false)}
            onCommit={(draft) => update(draft, true)}
          />
        );
      })}
      <div className="pipeline-item-actions">
        <button
          onClick={() => {
            const item = namedDraft(
              visible[0]?.name ?? "Group",
              visible[0] ? formulaToken(visible[0].name) : ""
            );
            patch(step.id, (current) =>
              current.kind === "summarize"
                ? { ...current, groupKeys: [...current.groupKeys, item] }
                : current
            );
            setPendingEditor(commandId(step, item.id));
          }}
        >
          <Plus size={11} /> Group
        </button>
        <button
          onClick={() => {
            const item = namedDraft("Aggregate", "");
            patch(step.id, (current) =>
              current.kind === "summarize"
                ? { ...current, aggregates: [...current.aggregates, item] }
                : current
            );
            setPendingEditor(commandId(step, item.id));
          }}
        >
          <Plus size={11} /> Value
        </button>
        <label>
          <input
            type="checkbox"
            checked={step.maintainOrder}
            onChange={(event) =>
              void savePatch(step.id, (current) =>
                current.kind === "summarize"
                  ? { ...current, maintainOrder: event.target.checked }
                  : current
              )
            }
          />
          ordered
        </label>
      </div>
    </div>
  );
}
