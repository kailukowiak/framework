import { Plus, X } from "lucide-react";
import { PipelineCommand } from "./PipelineCommand";
import {
  draftName,
  nextBlankColumnName,
  type NamedDraft,
  parseNamedTransformation,
} from "./PipelineColumnNames";
import type { FormulaReference } from "./lib/formulaReferences";
import {
  siblingColumnExplanation,
  siblingReferenceInFormula,
} from "./lib/formulaPicking";
import { BLANK_CALCULATION } from "./lib/pipelineChainEdits";
import {
  namedCommand,
  outputColumnIdForName,
} from "./lib/pipelineStepCommands";
import { namedDraft, type StepDraft, type VisibleColumn } from "./lib/pipelineSteps";
import type { Column, FrameObject, FrameStepInput } from "./lib/types";

/** The calculations of one Add or replace columns step. */
export function PipelineWithColumnsStep({
  step,
  visible,
  stepReferences,
  input,
  scope,
  editingFrame,
  commandId,
  commandFocus,
  patch,
  commitPatch,
  savePatch,
  rejectCommand,
  setPendingEditor,
}: {
  step: Extract<StepDraft, { kind: "withColumns" }>;
  visible: VisibleColumn[];
  stepReferences: FormulaReference[];
  input: { label: string; columns: Column[]; completionFrameId?: string };
  scope: { steps: FrameStepInput[]; stepIndex: number };
  editingFrame: FrameObject;
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
      {step.columns.map((column) => {
        const id = commandId(step, column.id);
        const update = (draft: string, saveNow: boolean) => {
          // Half a formula is not a calculation yet. The draft lives in the
          // editor and the bar until Return; it is deliberately not written
          // into the step on every keystroke, because a step is what the
          // *next* save writes to the document — so an abandoned draft used
          // to arrive in the document under some unrelated gesture's save,
          // and the schema preview kept announcing "unknown name" about a
          // word still being typed.
          if (!saveNow) return;
          const parsed = parseNamedTransformation(draft);
          if (!parsed) {
            rejectCommand("Write a column name, =, and a formula");
            return;
          }
          // Answered here rather than by the engine: see
          // siblingReferenceInFormula for why "Unknown name" is the wrong
          // sentence for a name that is one line further up this step.
          const sibling = siblingReferenceInFormula(
            parsed.formula,
            step.columns
              .filter((item) => item.id !== column.id)
              .map((item) => ({ name: draftName(item) })),
            visible.map((item) => item.name)
          );
          if (sibling) {
            rejectCommand(siblingColumnExplanation(sibling, parsed.name));
            return;
          }
          const change = (current: StepDraft): StepDraft =>
            current.kind === "withColumns"
              ? {
                  ...current,
                  columns: current.columns.map((item) =>
                    item.id === column.id
                      ? {
                          ...item,
                          // Naming an output exactly like a column
                          // visible above this step is an overwrite,
                          // not a request for "Name_2". Identity is
                          // what makes Polars replace it in place.
                          outputColumnId: outputColumnIdForName(
                            visible,
                            item.outputColumnId,
                            parsed.name
                          ),
                          name: parsed.name,
                          formula: parsed.formula,
                        }
                      : item
                  ),
                }
              : current;
          return commitPatch(step.id, change);
        };
        return (
          <div className="pipeline-command-row" key={column.id}>
            <PipelineCommand
              editorId={id}
              label={draftName(column)}
              initialDraft={namedCommand(draftName(column), column.formula)}
              references={stepReferences}
              frameId={input.completionFrameId}
              scope={scope}
              focusToken={commandFocus(id, column.focusToken)}
              targetColumnId={column.outputColumnId}
              anchorRowIndex={column.anchorRowIndex}
              anchorFrameId={editingFrame.id}
              focusSelection={
                column.focusToken === undefined
                  ? undefined
                  : {
                      // A fresh formula selects from the very
                      // start, name included: the placeholder
                      // `Column N` is nobody's chosen name, so
                      // typing should be able to replace the whole
                      // line — name, =, and expression — in one
                      // stroke. focusAtEnd flows keep the caret at
                      // the end for appending instead.
                      start: column.focusAtEnd
                        ? namedCommand(draftName(column), column.formula)
                            .length
                        : 0,
                      end: namedCommand(draftName(column), column.formula)
                        .length,
                    }
              }
              onChange={(draft) => update(draft, false)}
              onCommit={(draft) => update(draft, true)}
            />
            <button
              className="remove-derived-row"
              title="Remove column"
              onClick={() =>
                void savePatch(step.id, (current) =>
                  current.kind === "withColumns" && current.columns.length > 1
                    ? {
                        ...current,
                        columns: current.columns.filter(
                          (item) => item.id !== column.id
                        ),
                      }
                    : current
                )
              }
            >
              <X size={11} />
            </button>
          </div>
        );
      })}
      <AddColumnButton
        names={visible.map((item) => item.name)}
        onAdd={(column) => {
          patch(step.id, (current) =>
            current.kind === "withColumns"
              ? { ...current, columns: [...current.columns, column] }
              : current
          );
          setPendingEditor(commandId(step, column.id));
        }}
      />
    </div>
  );
}

/**
 * Wrangle's own doorway into a new column. It opens the same line the
 * header's *Add calculated column* opens — the placeholder formula and the
 * whole-line selection both — so the two ways in are one thing to learn.
 * `focusToken` is what asks for that selection (see `PipelineCommand`); a
 * freshly minted row only ever needs the one.
 */
function AddColumnButton({
  names,
  onAdd,
}: {
  names: string[];
  onAdd: (column: NamedDraft & { focusToken: number }) => void;
}) {
  return (
    <button
      className="pipeline-add-item"
      onClick={() =>
        onAdd({
          ...namedDraft(nextBlankColumnName(names), BLANK_CALCULATION),
          focusToken: 1,
        })
      }
    >
      <Plus size={11} /> Add or replace column
    </button>
  );
}
