import { Plus, X } from "lucide-react";
import { PipelineCommand } from "./PipelineCommand";
import type { FormulaReference } from "./lib/formulaReferences";
import type { StepDraft } from "./lib/pipelineSteps";
import type { Column, FrameStepInput } from "./lib/types";

/** The conditions of one Filter rows step, each its own command line. */
export function PipelineFilterStep({
  step,
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
  step: Extract<StepDraft, { kind: "filter" }>;
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
    <div className="pipeline-filter-list">
      {step.predicates.map((predicate, predicateIndex) => {
        const id = commandId(step, predicate.id);
        const update = (draft: string, saveNow: boolean) => {
          if (!draft.trim()) {
            if (saveNow)
              rejectCommand("Write a condition before applying this filter");
            return;
          }
          const change = (current: StepDraft): StepDraft =>
            current.kind === "filter"
              ? {
                  ...current,
                  predicates: current.predicates.map((item) =>
                    item.id === predicate.id
                      ? { ...item, formula: draft }
                      : item
                  ),
                }
              : current;
          if (saveNow) return commitPatch(step.id, change);
          patch(step.id, change);
        };
        return (
          <div className="pipeline-filter-row" key={predicate.id}>
            {predicateIndex > 0 && (
              <button
                type="button"
                className="pipeline-filter-join"
                title={`Match ${step.matchAll ? "any" : "all"} conditions instead`}
                onClick={() =>
                  void savePatch(step.id, (current) =>
                    current.kind === "filter"
                      ? { ...current, matchAll: !current.matchAll }
                      : current
                  )
                }
              >
                {step.matchAll ? "AND" : "OR"}
              </button>
            )}
            <div className="pipeline-filter-condition">
              <PipelineCommand
                editorId={id}
                label={`Filter condition ${predicateIndex + 1}`}
                initialDraft={predicate.formula}
                references={stepReferences}
                frameId={input.completionFrameId}
                scope={scope}
                focusToken={commandFocus(id, predicate.focusToken)}
                focusSelection={predicate.focusSelection}
                appliesToAllRows
                onChange={(draft) => update(draft, false)}
                onCommit={(draft) => update(draft, true)}
              />
              {step.predicates.length > 1 && (
                <button
                  type="button"
                  className="remove-derived-row"
                  title="Remove condition"
                  onClick={() =>
                    void savePatch(step.id, (current) =>
                      current.kind === "filter"
                        ? {
                            ...current,
                            predicates: current.predicates.filter(
                              (item) => item.id !== predicate.id
                            ),
                          }
                        : current
                    )
                  }
                >
                  <X size={11} />
                </button>
              )}
            </div>
          </div>
        );
      })}
      <button
        type="button"
        className="pipeline-add-item"
        onClick={() => {
          const predicate = { id: crypto.randomUUID(), formula: "" };
          patch(step.id, (current) =>
            current.kind === "filter"
              ? {
                  ...current,
                  predicates: [...current.predicates, predicate],
                }
              : current
          );
          setPendingEditor(commandId(step, predicate.id));
        }}
      >
        <Plus size={11} /> condition
      </button>
    </div>
  );
}
