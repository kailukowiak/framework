import { Plus, X } from "lucide-react";
import { useState } from "react";
import { PipelineCommand } from "./PipelineCommand";
import {
  draftName,
  nextBlankColumnName,
  type NamedDraft,
} from "./PipelineColumnNames";
import type { CellDoorwayFold } from "./hooks/useColumnTransformRequest";
import type { FormulaReference } from "./lib/formulaReferences";
import { siblingReferenceInFormula } from "./lib/formulaPicking";
import {
  BLANK_CALCULATION,
  splitSiblingReferenceStep,
} from "./lib/pipelineChainEdits";
import {
  namedCommand,
  outputColumnIdForName,
} from "./lib/pipelineStepCommands";
import { namedDraft, type StepDraft, type VisibleColumn } from "./lib/pipelineSteps";
import { transformationForTarget } from "./lib/targetedTransformation";
import type { Column, FrameObject, FrameStepInput } from "./lib/types";

/**
 * The step with one calculation's committed name and expression written in.
 *
 * Naming an output exactly like a column visible above this step is an
 * overwrite, not a request for "Name_2": identity is what makes Polars
 * replace it in place, so the output id follows the name.
 */
function rewrittenColumn(
  current: StepDraft,
  columnId: string,
  parsed: { name: string; formula: string },
  visible: VisibleColumn[]
): StepDraft {
  if (current.kind !== "withColumns") return current;
  return {
    ...current,
    columns: current.columns.map((item) =>
      item.id === columnId
        ? {
            ...item,
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
  };
}

/**
 * Where the caret lands when a calculation line is opened for typing.
 *
 * `focusAtEnd` flows put it at the end, for appending a method. Otherwise
 * typing should replace what has not been chosen yet, and nothing more. A
 * fresh placeholder -- `Column N` = the blank calculation -- has had
 * nothing chosen at all, so the whole line is selected and one stroke can
 * replace name, `=` and expression together. A line on a column that
 * already has a name -- the cell's `=` doorway, reopening a calculation --
 * selects only the expression after `name = `: selecting the name too was
 * how `=10*2` typed at a cell replaced `Revenue` along with it and ended in
 * "Write a column name, =, and a formula".
 */
function focusSelectionFor(
  column: NamedDraft,
  line: string
): { start: number; end: number } {
  if (column.focusAtEnd) return { start: line.length, end: line.length };
  // Compared without whitespace: a saved chain comes back formatted, which
  // puts the blank's `.cast` on a line of its own (formatFormulaChains only
  // ever moves whitespace).
  if (column.formula.replace(/\s+/g, "") === BLANK_CALCULATION)
    return { start: 0, end: line.length };
  return { start: line.length - column.formula.length, end: line.length };
}

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
  commitChain,
  savePatch,
  rejectCommand,
  setPendingEditor,
  cellDoorway,
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
  commitChain: (update: (current: StepDraft[]) => StepDraft[]) => Promise<void>;
  savePatch: (
    stepId: string,
    update: (step: StepDraft) => StepDraft
  ) => Promise<string | null>;
  rejectCommand: (message: string) => never;
  setPendingEditor: (editorId: string) => void;
  /** Set while a line opened by a cell's `=` is still a draft. */
  cellDoorway?: CellDoorwayFold;
}) {
  // The doorway line's live draft, kept only so the bar can say whether
  // Return will write "this cell" or the column's "all rows" -- the answer
  // turns on what has been typed, and it changes as it is typed.
  const [doorwayDraft, setDoorwayDraft] = useState<string | null>(null);
  const doorway = cellDoorway?.stepId === step.id ? cellDoorway : undefined;
  return (
    <div className="pipeline-command-list">
      {step.columns.map((column) => {
        const id = commandId(step, column.id);
        const line = namedCommand(draftName(column), column.formula);
        const update = (draft: string, saveNow: boolean) => {
          // Half a formula is not a calculation yet. The draft lives in the
          // editor and the bar until Return; it is deliberately not written
          // into the step on every keystroke, because a step is what the
          // *next* save writes to the document — so an abandoned draft used
          // to arrive in the document under some unrelated gesture's save,
          // and the schema preview kept announcing "unknown name" about a
          // word still being typed.
          if (doorway) setDoorwayDraft(draft);
          if (!saveNow) return;
          const parsed = transformationForTarget(draft, draftName(column));
          if (!parsed) {
            rejectCommand("Write a column name, =, and a formula");
            return;
          }
          if (doorway) return doorway.commit(parsed, () => saveAsColumn(parsed));
          return saveAsColumn(parsed);
        };
        const saveAsColumn = (parsed: { name: string; formula: string }) => {
          // A name written one line further up this step is a reference the
          // engine cannot serve — every formula in a `with_columns` reads
          // the frame as it stood before the step — but the sentence is not
          // wrong, only the arrangement is. So the commit rearranges: this
          // calculation moves into a step of its own, directly below, where
          // the sibling it reads exists. See splitSiblingReferenceStep.
          const sibling = siblingReferenceInFormula(
            parsed.formula,
            step.columns
              .filter((item) => item.id !== column.id)
              .map((item) => ({ name: draftName(item) })),
            visible.map((item) => item.name)
          );
          const change = (current: StepDraft): StepDraft =>
            rewrittenColumn(current, column.id, parsed, visible);
          if (!sibling) return commitPatch(step.id, change);
          return commitChain((current) => {
            const edited = current.map((item) =>
              item.id === step.id ? change(item) : item
            );
            return (
              splitSiblingReferenceStep(
                edited,
                step.id,
                column.id,
                visible.map((item) => item.name)
              ) ?? edited
            );
          });
        };
        return (
          <div className="pipeline-command-row" key={column.id}>
            <PipelineCommand
              editorId={id}
              label={draftName(column)}
              initialDraft={line}
              references={stepReferences}
              frameId={input.completionFrameId}
              scope={scope}
              focusToken={commandFocus(id, column.focusToken)}
              targetColumnId={column.outputColumnId}
              appliesToAllRows={
                doorway?.foldsDraft(doorwayDraft ?? line) ? false : undefined
              }
              anchorRowIndex={column.anchorRowIndex}
              anchorFrameId={editingFrame.id}
              focusSelection={
                column.focusToken === undefined
                  ? undefined
                  : focusSelectionFor(column, line)
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
