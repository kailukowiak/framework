import { useCallback, useEffect, useRef, useState } from "react";
import { useActiveFormulaEditorWatcher } from "../ActiveFormulaEditor";
import { evaluateWithStepSample, type EvaluateLiteralCell } from "../lib/evaluateLiteralCell";
import { announceGridReturn } from "./useGridEdgeGrowth";
import type { OperationHandler } from "../lib/handlers";
import { referenceFreeArithmetic } from "../lib/literalArithmetic";
import {
  appendOrderedColumnTransformation,
  focusExistingCalculatedColumn,
} from "../lib/pipelineChainEdits";
import type { StepDraft } from "../lib/pipelineSteps";
import { transformationForTarget } from "../lib/targetedTransformation";
import type { FrameObject } from "../lib/types";
import type { TransformColumnEditorRequest } from "./usePipelineColumnRequests";

/**
 * The line a cell's `=` opened, while it is still only a draft.
 *
 * `editorId` is the shared formula session's address for that line, which
 * is how an abandoned doorway is noticed: the session ends without a
 * commit.
 */
type PendingCellDoorway = {
  stepId: string;
  editorId: string;
  column: { id: string; name: string };
  rowId: string;
  literal: boolean;
};

/**
 * What the withColumns step needs to know about a line opened from a cell:
 * whether committing a draft would write that one cell rather than the
 * column, and the commit that decides it.
 */
export type CellDoorwayFold = {
  stepId: string;
  foldsDraft: (draft: string) => boolean;
  commit: (
    parsed: { name: string; formula: string },
    saveAsColumn: () => Promise<void>
  ) => Promise<void>;
};

/**
 * Handles a request to transform an existing column -- the context menu's
 * transforms, a formula begun in one of its cells, and reopening a
 * calculation the column already has.
 *
 * The cell doorway (`fromCell`) differs from the others in one respect: it
 * does not save on opening. The others append a real transformation
 * (`.cast(...)`, a fill) that is worth saving as it appears; the cell's `=`
 * appends only the column restating itself, `` `Revenue` = `Revenue` ``,
 * which means nothing yet. Saving it anyway put a step and an undo entry in
 * the document for a gesture that had not said anything, and left both
 * behind when Escape abandoned it. So the line stays a local draft -- the
 * chain sync keeps an unsaved draft through unrelated renders -- until
 * Return. Return then either saves it as the column's formula, through the
 * ordinary commit, or folds it: reference-free arithmetic in a hand-entered
 * cell becomes that cell's value, written with the ordinary `setCell`, and
 * the draft line is dropped, so the chain is exactly as it was. An ended
 * session that never committed drops the draft too.
 */
export function useColumnTransformRequest({
  request,
  onHandled,
  editingFrame,
  steps,
  setSteps,
  persist,
  setFormulaError,
  onOperation,
  evaluate = evaluateWithStepSample,
}: {
  request: TransformColumnEditorRequest | undefined;
  onHandled?: () => void;
  editingFrame: FrameObject;
  steps: StepDraft[];
  setSteps: (update: (current: StepDraft[]) => StepDraft[]) => void;
  persist: (next: StepDraft[]) => Promise<string | null>;
  setFormulaError: (message: string | null) => void;
  onOperation: OperationHandler;
  evaluate?: EvaluateLiteralCell;
}): CellDoorwayFold | undefined {
  const handled = useRef<number | null>(null);
  // State for rendering, a ref for the session watcher and the commit,
  // which both run outside render and must see a settled doorway at once.
  const [pending, setPending] = useState<PendingCellDoorway | null>(null);
  const pendingRef = useRef<PendingCellDoorway | null>(null);
  const sessionSeen = useRef(false);
  const settle = useCallback((next: PendingCellDoorway | null) => {
    pendingRef.current = next;
    sessionSeen.current = false;
    setPending(next);
  }, []);
  const dropDraft = useCallback(
    (stepId: string) => setSteps((current) => current.filter((step) => step.id !== stepId)),
    [setSteps]
  );

  useEffect(() => {
    if (request === undefined || handled.current === request.token) return;
    handled.current = request.token;
    onHandled?.();
    const column = editingFrame.columns.find(
      (candidate) => candidate.id === request.columnId
    );
    if (!column) return;
    if (request.editExisting) {
      const focused = focusExistingCalculatedColumn(
        steps,
        column.id,
        request.token,
        request.anchorRowIndex
      );
      if (focused) setSteps(() => focused);
      else setFormulaError(`The calculation for ${column.name} is not in this chain.`);
      return;
    }
    // A second `=` before the first was committed replaces that draft
    // rather than stacking a second unsaved line under it.
    const earlier = pendingRef.current;
    const base = earlier ? steps.filter((step) => step.id !== earlier.stepId) : steps;
    if (earlier) settle(null);
    const next = appendOrderedColumnTransformation(
      base,
      column,
      request.formula,
      request.orderByColumnId,
      {
        focusToken: request.focus ? request.token : undefined,
        focusAtEnd: request.focusAtEnd,
        anchorRowIndex: request.anchorRowIndex,
      }
    );
    const added = next[next.length - 1];
    if (!request.fromCell || added?.kind !== "withColumns") {
      // Through `persist` for the identity-preserving reason given beside
      // the add-calculated-column request in PipelineEditor.
      void persist(next);
      return;
    }
    setSteps(() => next);
    settle({
      stepId: added.id,
      editorId: `pipeline:${editingFrame.id}:${added.id}:${added.columns[0].id}`,
      column: { id: column.id, name: column.name },
      rowId: request.fromCell.rowId,
      literal: request.fromCell.literal,
    });
  }, [
    request,
    editingFrame.columns,
    editingFrame.id,
    onHandled,
    persist,
    steps,
    setSteps,
    setFormulaError,
    dropDraft,
    settle,
  ]);

  useActiveFormulaEditorWatcher((active) => {
    const doorway = pendingRef.current;
    if (!doorway) return;
    if (active?.id === doorway.editorId) {
      sessionSeen.current = true;
      return;
    }
    // Only a session that began and then ended is an abandonment; before
    // the line's editor has activated, some other session being current
    // says nothing about this one.
    if (!sessionSeen.current) return;
    settle(null);
    dropDraft(doorway.stepId);
  });

  const folding = (parsed: { name: string; formula: string } | null) => {
    if (!pending?.literal || !parsed || parsed.name !== pending.column.name) return null;
    return referenceFreeArithmetic(parsed.formula);
  };

  if (!pending) return undefined;
  return {
    stepId: pending.stepId,
    foldsDraft: (draft) =>
      folding(transformationForTarget(draft, pending.column.name)) !== null,
    commit: async (parsed, saveAsColumn) => {
      const doorway = pendingRef.current;
      const expression = doorway ? folding(parsed) : null;
      if (!doorway || expression === null) {
        await saveAsColumn();
        if (pendingRef.current === doorway) settle(null);
        return;
      }
      let raw: string;
      try {
        raw = await evaluate(editingFrame.id, doorway.column, expression);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setFormulaError(message);
        throw new Error(message, { cause: error });
      }
      // The cell first: a refused write keeps the session, and the line it
      // is typed in, so the person can see the refusal and fix the text.
      const failure = await onOperation(
        {
          type: "setCell",
          frameId: editingFrame.id,
          rowId: doorway.rowId,
          columnId: doorway.column.id,
          raw,
        },
        { inlineError: true }
      );
      if (failure) {
        setFormulaError(failure);
        throw new Error(failure);
      }
      setFormulaError(null);
      settle(null);
      dropDraft(doorway.stepId);
      // The value is in the cell, so the edit is finished: the keyboard goes
      // back to the grid one row down, like any committed cell edit, rather
      // than staying in a formula bar that now holds nothing.
      announceGridReturn();
    },
  };
}
