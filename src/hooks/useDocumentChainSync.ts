import { useEffect, useMemo, useRef, useState } from "react";
import { useActiveFormulaEditorCommands } from "../ActiveFormulaEditor";
import { stepInput, type StepDraft } from "../lib/pipelineSteps";
import { stepsFromRendered } from "../lib/pipelineChainEdits";
import type { Column, FrameObject, RenderedFrameStep } from "../lib/types";

/**
 * Keeps the step draft honest against the document's saved chain.
 *
 * The document is the authority; the draft is only the editing surface over
 * it. Seeding once at mount and never looking back is how the step list
 * kept showing a step an undo had removed — and worse, how the next saved
 * gesture (hide a column, append a calculation) wrote that stale draft back
 * over the document, silently reversing the undo. So the saved chain is
 * watched: when it moves under the editor — an undo, a redo, a peer,
 * another editing surface — the draft is re-derived from it. When the draft
 * already says the same thing (the ordinary case right after this editor's
 * own save round-trips), it is kept, so open formula editors and their
 * focus survive saving; an unsaved local draft (a filter still being typed)
 * survives unrelated document renders because the reseed fires only when
 * the saved chain itself changed. State is adjusted during render — the
 * sanctioned shape for derived state — so the request-handling effects
 * below never act on a stale draft.
 */
export function useDocumentChainSync(
  renderedSteps: RenderedFrameStep[],
  editingFrame: FrameObject,
  inputColumns: Column[],
  steps: StepDraft[],
  setSteps: (next: StepDraft[]) => void,
  lastSavedSignature: { current: string | null }
) {
  const formulaEditors = useActiveFormulaEditorCommands();
  const authoritative = useMemo(
    () => stepsFromRendered(renderedSteps, editingFrame, inputColumns),
    [renderedSteps, editingFrame, inputColumns]
  );
  const signature = useMemo(
    () => JSON.stringify(authoritative.map(stepInput)),
    [authoritative]
  );
  const [reconciled, setReconciled] = useState(signature);
  const reseeded = useRef(false);
  if (reconciled !== signature) {
    setReconciled(signature);
    // A chain this editor itself just saved is not the document moving
    // *under* it, even when the local draft no longer matches: typing can
    // legitimately run ahead of the save's echo. The everyday case is the
    // creation gesture — its placeholder save is still round-tripping while
    // the person is already replacing the formula, and reseeding here would
    // discard their typing and end the session they are typing in. The echo
    // is recognized by value, not by timing, because the engine's answer
    // arrives whenever it arrives.
    const ownEcho = signature === lastSavedSignature.current;
    if (!ownEcho && JSON.stringify(steps.map(stepInput)) !== signature) {
      setSteps(authoritative);
      reseeded.current = true;
    }
  }
  // A reseed replaces the drafts a session was addressing, so the session
  // ends with them: leaving it armed keeps a formula bar that says "Edit
  // Column 1" wired to callbacks over the pre-reseed chain, and committing
  // those would write the superseded chain back over the document — the
  // same silent undo-reversal the reseed exists to prevent. This never
  // fires on this editor's own saves: every save path here persists the
  // formatted draft, which reconciles as unchanged. Cleared in an effect,
  // not during render, because ending the session notifies subscribers.
  // The effect deliberately runs after every render: the flag decides.
  useEffect(() => {
    if (!reseeded.current) return;
    reseeded.current = false;
    const active = formulaEditors.getActive();
    if (active?.id.startsWith(`pipeline:${editingFrame.id}:`))
      formulaEditors.clear(active.id);
  });
}
