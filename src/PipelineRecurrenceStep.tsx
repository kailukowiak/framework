import { PipelineCommand } from "./PipelineCommand";
import type { FormulaReference } from "./lib/formulaReferences";
import type { FrameStepInput } from "./lib/types";

export type PipelineRecurrenceDraft = {
  id: string;
  kind: "recurrence";
  outputColumnId: string;
  name: string;
  seed: string;
  formula: string;
  partitionName?: string;
  focusToken?: number;
  focusAtEnd?: boolean;
  anchorRowIndex?: number;
};

type RecurrenceUpdate = Partial<
  Pick<PipelineRecurrenceDraft, "seed" | "formula" | "partitionName">
>;

/** The compact visual face of a sequential formula saved as one Wrangle step. */
export function PipelineRecurrenceStep({
  step,
  references,
  columnNames,
  frameId,
  editingFrameId,
  scope,
  seedEditorId,
  nextEditorId,
  focusToken,
  onChange,
  onCommit,
  onReject,
}: {
  step: PipelineRecurrenceDraft;
  references: FormulaReference[];
  columnNames: string[];
  frameId?: string;
  editingFrameId: string;
  scope: { steps: FrameStepInput[]; stepIndex: number };
  seedEditorId: string;
  nextEditorId: string;
  focusToken?: number;
  onChange: (update: RecurrenceUpdate) => void;
  onCommit: (update: RecurrenceUpdate) => void | Promise<void>;
  onReject: (message: string) => void;
}) {
  const previousReference: FormulaReference = {
    id: `previous:${step.outputColumnId}`,
    label: `Previous ${step.name}`,
    token: "previous()",
    kind: "value",
    detail: "result from the preceding row",
  };
  const required = (
    draft: string,
    update: RecurrenceUpdate,
    message: string
  ) => (draft.trim() ? onCommit(update) : onReject(message));
  return (
    <div className="pipeline-recurrence">
      <label>
        <span>First row</span>
        <PipelineCommand
          editorId={seedEditorId}
          label={`First ${step.name}`}
          initialDraft={step.seed}
          references={references}
          frameId={frameId}
          scope={scope}
          onChange={(seed) => onChange({ seed })}
          onCommit={(seed) =>
            required(seed, { seed }, "Write the first row value")
          }
        />
      </label>
      <label>
        <span>Each next row</span>
        <PipelineCommand
          editorId={nextEditorId}
          label={`Each next ${step.name}`}
          initialDraft={step.formula}
          references={[previousReference, ...references]}
          frameId={frameId}
          scope={scope}
          focusToken={focusToken}
          focusSelection={
            step.focusToken === undefined
              ? undefined
              : {
                  start: step.focusAtEnd ? step.formula.length : 0,
                  end: step.formula.length,
                }
          }
          targetColumnId={step.outputColumnId}
          previousResultToken="previous()"
          anchorRowIndex={step.anchorRowIndex}
          anchorFrameId={editingFrameId}
          onChange={(formula) => onChange({ formula })}
          onCommit={(formula) =>
            required(
              formula,
              { formula },
              "Write how each next row is calculated"
            )
          }
        />
      </label>
      <label className="pipeline-recurrence-restart">
        <span>Restart for each</span>
        <select
          value={step.partitionName ?? ""}
          onChange={(event) =>
            void onCommit({ partitionName: event.target.value || undefined })
          }
        >
          <option value="">Never</option>
          {columnNames.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
