import { useEffect, useRef, useState } from "react";
import { useFormulaEditorRegistration } from "./ActiveFormulaEditor";
import type { FormulaReference } from "./lib/formulaReferences";
import type { FrameStepInput } from "./lib/types";

type PipelineCommandProps = {
  editorId: string;
  label: string;
  initialDraft: string;
  detail?: string;
  references: FormulaReference[];
  frameId?: string;
  scope?: { steps: FrameStepInput[]; stepIndex: number };
  focusToken?: number;
  focusSelection?: { start: number; end: number };
  targetColumnId?: string;
  appliesToAllRows?: boolean;
  previousResultToken?: string;
  anchorRowIndex?: number;
  anchorFrameId?: string;
  onChange: (draft: string) => void;
  onCommit: (draft: string) => void | Promise<void>;
};

/**
 * A transformation row is an address for the one shared formula editor.
 * Clicking it moves that editor to the formula bar; it does not create a
 * second draft hidden in the inspector.
 */
export function PipelineCommand({
  editorId,
  label,
  initialDraft,
  detail,
  references,
  frameId,
  scope,
  focusToken,
  focusSelection,
  targetColumnId,
  appliesToAllRows,
  previousResultToken,
  anchorRowIndex,
  anchorFrameId,
  onChange,
  onCommit,
}: PipelineCommandProps) {
  const [draft, setDraft] = useState(initialDraft);
  const previousInitial = useRef(initialDraft);
  useEffect(() => {
    if (draft === previousInitial.current) setDraft(initialDraft);
    previousInitial.current = initialDraft;
  }, [draft, initialDraft]);
  const registration = useFormulaEditorRegistration({
    id: editorId,
    label,
    kind: "formula",
    draft,
    completion: {
      references,
      frameId,
      scope,
      targetColumnId,
      previousResultToken,
      anchorRowIndex,
      anchorFrameId,
      appliesToAllRows: appliesToAllRows ?? targetColumnId !== undefined,
    },
    onChange: (next) => {
      setDraft(next);
      onChange(next);
    },
    onCommit,
    onFocus: (selection) => {
      // Deferred on a timer, never requestAnimationFrame: a hidden or
      // occluded WKWebView schedules no animation frames at all, so an
      // rAF-deferred focus silently never runs there — which is how "Add
      // calculated column" could open its formula unfocused. The retry is
      // for the other half of the race: the bar's textarea may not exist
      // yet when activation and the bar's own render land in one commit.
      const attempt = (remaining: number) => {
        const input = window.document.querySelector<HTMLTextAreaElement>(
          ".scratchwork-formula-bar textarea"
        );
        if (input) {
          input.focus();
          input.setSelectionRange(selection.start, selection.end);
          return;
        }
        if (remaining > 0) window.setTimeout(() => attempt(remaining - 1), 16);
      };
      window.setTimeout(() => attempt(20), 0);
    },
  });
  const focusRequest = useRef({
    activate: registration.activateAt,
    selection: focusSelection ?? { start: draft.length, end: draft.length },
  });
  focusRequest.current = {
    activate: registration.activateAt,
    selection: focusSelection ?? { start: draft.length, end: draft.length },
  };
  useEffect(() => {
    if (focusToken !== undefined)
      focusRequest.current.activate(focusRequest.current.selection);
  }, [focusToken]);
  const needsOrder =
    /\.shift\s*\(/.test(draft) &&
    scope !== undefined &&
    !scope.steps
      .slice(0, scope.stepIndex)
      .some((step) => step.kind === "sort");
  return (
    <div className="pipeline-command-address">
      <button
        type="button"
        className="pipeline-command"
        onClick={() =>
          registration.activateAt({ start: draft.length, end: draft.length })
        }
      >
        {detail && <span>{detail}</span>}
        <code>{draft || "…"}</code>
      </button>
      {needsOrder && (
        <small className="pipeline-command-guidance">
          Previous or next row needs a Sort step above this calculation.
        </small>
      )}
    </div>
  );
}
