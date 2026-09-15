import { X } from "lucide-react";
import { useState } from "react";
import {
  CombineExpandForm,
  CombinePairForm,
  CombineStackForm,
} from "./CombineChainForms";
import { CombineMatchForm } from "./CombineMatchForm";
import { existingChainSteps } from "./lib/combineChain";
import type { OperationHandler } from "./lib/handlers";
import type { JoinState } from "./lib/joinState";
import type { DocumentView, FrameObject, FrameStepInput } from "./lib/types";

/**
 * How this table's rows relate to that table's rows.
 *
 * Four answers to one question, so one surface asks it. Three of them append
 * a step to this frame's own chain; *Match on a key* creates a new frame from
 * both. The gestures on the canvas remain shortcuts that preselect an answer
 * — they are not separate features with separate dialogs.
 */
export type CombineRelationship = "match" | "pair" | "stack" | "expand";

const RELATIONSHIPS: Array<{ value: CombineRelationship; label: string }> = [
  { value: "match", label: "Match on a key" },
  { value: "pair", label: "Same rows in order" },
  { value: "stack", label: "Rows under rows" },
  { value: "expand", label: "Every row with every row" },
];

export function CombineDialog({
  state,
  document,
  relationship: initialRelationship = "match",
  onClose,
  onOperation,
  onCreated,
  onChained,
}: {
  state: NonNullable<JoinState>;
  document: DocumentView;
  relationship?: CombineRelationship;
  onClose: () => void;
  onOperation: OperationHandler;
  /** A new frame was created and should be found and selected. */
  onCreated: () => void;
  /** A step was appended to this frame's chain; show it in Wrangle. */
  onChained?: (frameId: string) => void;
}) {
  const [relationship, setRelationship] = useState(initialRelationship);
  const frames = document.objects.filter(
    (object): object is FrameObject => object.kind === "frame"
  );
  const primary = frames.find((frame) => frame.id === state.primaryFrameId)!;
  const candidates = frames.filter((frame) => frame.id !== primary.id);

  /**
   * `setFramePipeline` replaces the chain whole, so a chain relationship
   * resends everything already there and appends its own step. The existing
   * steps are rebuilt through the Wrangle editor's own reader so their
   * formulas go back in the spelling the engine echoes.
   */
  const saveChain = (steps: FrameStepInput[]) =>
    onOperation(
      {
        type: "setFramePipeline",
        frameId: primary.id,
        steps: [
          ...existingChainSteps(
            primary,
            document.computedFrames[primary.id],
            document.objects
          ),
          ...steps,
        ],
      },
      { inlineError: true }
    ).then((failure) => {
      if (!failure) onChained?.(primary.id);
      return failure;
    });

  if (candidates.length === 0) return <NoOtherTable onClose={onClose} />;

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog join-dialog combine-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">COMBINE</span>
            <h2>{primary.name} with another table</h2>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close combine">
            <X size={16} />
          </button>
        </div>
        <label className="combine-line combine-relationship">
          Relationship
          <select
            aria-label="Relationship"
            value={relationship}
            onChange={(event) =>
              setRelationship(event.target.value as CombineRelationship)
            }
          >
            {RELATIONSHIPS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        {relationship === "match" && (
          <CombineMatchForm
            state={state}
            primary={primary}
            candidates={candidates}
            onOperation={onOperation}
            onCreated={onCreated}
          />
        )}
        {relationship === "pair" && (
          <CombinePairForm
            primary={primary}
            primaryComputed={document.computedFrames[primary.id]}
            candidates={candidates}
            computedFrames={document.computedFrames}
            onSave={saveChain}
          />
        )}
        {relationship === "stack" && (
          <CombineStackForm
            primary={primary}
            candidates={candidates}
            onSave={saveChain}
          />
        )}
        {relationship === "expand" && (
          <CombineExpandForm
            primary={primary}
            primaryComputed={document.computedFrames[primary.id]}
            candidates={candidates}
            computedFrames={document.computedFrames}
            onSave={saveChain}
          />
        )}
      </div>
    </div>
  );
}

function NoOtherTable({ onClose }: { onClose: () => void }) {
  return (
    <div className="dialog-backdrop">
      <div className="insert-dialog join-dialog combine-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">COMBINE</span>
            <h2>Add another table first</h2>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close combine">
            <X size={16} />
          </button>
        </div>
        <p className="empty-transform-note">
          Combining needs two tables on the canvas.
        </p>
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
