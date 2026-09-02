import { CircleAlert, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CommentStepRow } from "./PipelineCommentStepRow";
import { FormulaErrorDetails } from "./FormulaEditor";
import { PipelineCommand } from "./PipelineCommand";
import { PipelineFilterStep } from "./PipelineFilterStep";
import { PipelineRearrangeStep } from "./PipelineRearrangeStep";
import { PipelineSummarizeStep } from "./PipelineSummarizeStep";
import { PipelineWithColumnsStep } from "./PipelineWithColumnsStep";
import { useDocumentChainSync } from "./hooks/useDocumentChainSync";
import { usePipelineSchemaPreview } from "./hooks/usePipelineSchemaPreview";
import { appendColumnFilter } from "./PipelineColumnFilters";
import { PipelineFrameStepCommand } from "./PipelineFrameStepCommand";
import { PipelineHeading } from "./PipelineHeading";
import { PipelineJoinStep } from "./PipelineJoinStep";
import { PipelineRecurrenceStep } from "./PipelineRecurrenceStep";
import {
  BroadcastStepRow,
  ZipVectorStepRow,
  useVectorStepRequests,
  type BroadcastStepDraft,
} from "./PipelineVectorSteps";
import { meltedColumnIds } from "./lib/columnList";
import { type FormulaReference } from "./lib/formulaReferences";
import { exactName, parseNamedTransformation, uniqueColumnName } from "./PipelineColumnNames";
import { formatPipelineFormulas } from "./PipelineFormulaFormatting";
import {
  appendBlankCalculatedColumn,
  appendOrderedColumnTransformation,
  focusExistingCalculatedColumn,
  hidePipelineColumn,
  normalizeCalculatedColumnNames,
  rearrangePipelineColumns,
  stepsFromRendered,
} from "./lib/pipelineChainEdits";
import {
  columnListCommand,
  parsePivotCommand,
  parseSortCommand,
  parseUnpivotCommand,
  pivotCommand,
  sortCommand,
  unpivotCommand,
} from "./lib/pipelineStepCommands";
import {
  STEP_LABELS,
  blankStep,
  columnsBeforeStep,
  isOrderingOnlySelect,
  mintColumnId,
  referencesForStep,
  selectStepLabel,
  stepFormulas,
  stepInput,
  stepIsIncomplete,
  type AddStepKind,
  type StepDraft,
  type VisibleColumn,
} from "./lib/pipelineSteps";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  FrameObject,
  RenderedFrameStep,
} from "./lib/types";

function fixedJoinStep(
  frame: FrameObject,
  frames: FrameObject[],
  onOperation: OperationHandler
) {
  return frame.derivation?.join
    ? <PipelineJoinStep frame={frame} frames={frames} onOperation={onOperation} />
    : null;
}

/**
 * The transformation chain: an ordered list of steps, each editable, all
 * reorderable by dragging. Saving replaces the whole chain, which is also
 * how the core validates it -- a step naming a column no earlier step
 * produces is refused by name rather than failing later at read time.
 */
export function DerivedFrameCreator({
  input,
  editingFrame,
  renderedSteps,
  passThroughSteps,
  references,
  frames,
  joinFrames = [],
  addCalculatedColumnRequest,
  onAddCalculatedColumnRequestHandled,
  transformColumnRequest,
  onTransformColumnRequestHandled,
  filterColumnRequest,
  onFilterColumnRequestHandled,
  hidePipelineColumnRequest,
  onHidePipelineColumnRequestHandled,
  rearrangeColumnsRequest,
  onRearrangeColumnsRequestHandled,
  applyVectorRequest,
  onApplyVectorRequestHandled,
  pairVectorRequest,
  onPairVectorRequestHandled,
  onOperation,
}: {
  /** Where the chain starts: the frame it derives from, or its own data. */
  input: { label: string; columns: Column[]; completionFrameId?: string };
  editingFrame: FrameObject;
  renderedSteps: RenderedFrameStep[];
  /**
   * Leading steps that exist only so a linked frame owns its column ids.
   * They are held in state and saved back untouched, but never drawn —
   * nobody wrote them, and dropping them would strand every column id this
   * frame has published.
   */
  passThroughSteps: number;
  references: FormulaReference[];
  /** Every other frame in the document, for two-input wrangle steps. */
  frames: Array<{ id: string; name: string }>;
  joinFrames?: FrameObject[];
  /** Appends and saves one blank Number column at the bottom of the chain. */
  addCalculatedColumnRequest?: {
    token: number;
    afterColumnId?: string;
    anchorRowIndex?: number;
  };
  onAddCalculatedColumnRequestHandled?: () => void;
  /** Appends a same-id expression, optionally focusing it for another method call. */
  transformColumnRequest?: {
    token: number;
    columnId: string;
    formula: string;
    focus?: boolean;
    editExisting?: boolean;
    anchorRowIndex?: number;
    orderByColumnId?: string;
    focusAtEnd?: boolean;
  };
  onTransformColumnRequestHandled?: () => void;
  /** Opens a single-column condition in the canonical Wrangle filter step. */
  filterColumnRequest?: { token: number; columnId: string };
  onFilterColumnRequestHandled?: () => void;
  hidePipelineColumnRequest?: { token: number; columnId: string };
  onHidePipelineColumnRequestHandled?: () => void;
  rearrangeColumnsRequest?: { token: number; columnIds: string[] };
  onRearrangeColumnsRequestHandled?: () => void;
  applyVectorRequest?: {
    token: number;
    columnIds: string[];
    vector: string;
    expectedLength: number;
  };
  onApplyVectorRequestHandled?: () => void;
  pairVectorRequest?: {
    token: number;
    name: string;
    vector: string;
    expectedLength: number;
  };
  onPairVectorRequestHandled?: () => void;
  onOperation: OperationHandler;
}) {
  const [steps, setSteps] = useState<StepDraft[]>(() =>
    stepsFromRendered(renderedSteps, editingFrame, input.columns)
  );
  /** What this editor last wrote — how the sync tells its own echo apart. */
  const lastSavedSignature = useRef<string | null>(null);
  useDocumentChainSync(
    renderedSteps,
    editingFrame,
    input.columns,
    steps,
    setSteps,
    lastSavedSignature
  );
  const visibleAuthored = steps
    .map((step, index) => ({ step, index }))
    .slice(passThroughSteps)
    .filter(({ index }) => !isOrderingOnlySelect(input.columns, steps, index));
  const authoredStepNumberOffset = editingFrame.derivation?.join ? 2 : 1;
  const [formulaError, setFormulaError] = useState<string | null>(null);
  const [refreshingGeneratedColumns, setRefreshingGeneratedColumns] = useState(false);
  const [dragging, setDragging] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);
  const [draggingColumn, setDraggingColumn] = useState<string | null>(null);
  const [columnDrop, setColumnDrop] = useState<{
    columnId: string;
    after: boolean;
  } | null>(null);
  const [pendingEditor, setPendingEditor] = useState<string | null>(null);
  const handledAddRequest = useRef<number | null>(null);
  const handledTransformRequest = useRef<number | null>(null);
  const handledFilterRequest = useRef<number | null>(null);
  const handledHideRequest = useRef<number | null>(null);
  const handledRearrangeRequest = useRef<number | null>(null);

  // Declared ahead of the request-handling effects below: their dependency
  // arrays reference it directly (not just from inside the effect body), so
  // it has to exist by the time those arrays are built, not just by the
  // time the effects run.
  const persist = useCallback(
    async (next: StepDraft[]) => {
      const normalized = normalizeCalculatedColumnNames(
        next,
        input.columns,
        passThroughSteps
      ).map(formatPipelineFormulas);
      lastSavedSignature.current = JSON.stringify(normalized.map(stepInput));
      setSteps(normalized);
      const failure = await onOperation(
        {
          type: "setFramePipeline",
          frameId: editingFrame.id,
          steps: normalized.map(stepInput),
        },
        { inlineError: true }
      );
      setFormulaError(failure);
      return failure;
    },
    [input.columns, passThroughSteps, editingFrame.id, onOperation]
  );

  useVectorStepRequests({
    applyRequest: applyVectorRequest,
    pairRequest: pairVectorRequest,
    columns: editingFrame.columns,
    steps,
    persist: (next) => void persist(next as StepDraft[]),
    onApplyHandled: onApplyVectorRequestHandled,
    onPairHandled: onPairVectorRequestHandled,
    mintColumnId,
    uniqueColumnName,
  });

  // Saving through `persist`, not a bare setFramePipeline, is load-bearing:
  // persist formats the draft into the same deterministic form the document
  // sync re-derives from the saved chain, so this save's own round trip
  // reconciles as "the draft already says this" and the step list keeps its
  // identity. An unformatted save came back looking different, the sync
  // reseeded the list with fresh row identities, and the formula session the
  // request had just focused was left addressing an editor that no longer
  // existed — the bar could edit its draft but Enter saved nothing.
  useEffect(() => {
    if (
      addCalculatedColumnRequest === undefined ||
      handledAddRequest.current === addCalculatedColumnRequest.token
    )
      return;
    handledAddRequest.current = addCalculatedColumnRequest.token;
    onAddCalculatedColumnRequestHandled?.();
    void persist(
      appendBlankCalculatedColumn(
        steps,
        addCalculatedColumnRequest.token,
        addCalculatedColumnRequest.afterColumnId,
        editingFrame.columns,
        passThroughSteps,
        addCalculatedColumnRequest.anchorRowIndex
      )
    );
  }, [
    addCalculatedColumnRequest,
    editingFrame.columns,
    onAddCalculatedColumnRequestHandled,
    passThroughSteps,
    persist,
    steps,
  ]);

  useEffect(() => {
    if (
      transformColumnRequest === undefined ||
      handledTransformRequest.current === transformColumnRequest.token
    )
      return;
    handledTransformRequest.current = transformColumnRequest.token;
    onTransformColumnRequestHandled?.();
    const column = editingFrame.columns.find(
      (candidate) => candidate.id === transformColumnRequest.columnId
    );
    if (!column) return;
    if (transformColumnRequest.editExisting) {
      const focused = focusExistingCalculatedColumn(
        steps,
        column.id,
        transformColumnRequest.token,
        transformColumnRequest.anchorRowIndex
      );
      if (focused) setSteps(focused);
      else setFormulaError(`The calculation for ${column.name} is not in this chain.`);
      return;
    }
    // Through `persist` for the same identity-preserving reason as the
    // add-calculated-column request above.
    void persist(
      appendOrderedColumnTransformation(
        steps,
        column,
        transformColumnRequest.formula,
        transformColumnRequest.orderByColumnId,
        transformColumnRequest.focus ? transformColumnRequest.token : undefined,
        transformColumnRequest.focusAtEnd
      )
    );
  }, [
    transformColumnRequest,
    editingFrame.columns,
    onTransformColumnRequestHandled,
    persist,
    steps,
  ]);

  useEffect(() => {
    if (
      filterColumnRequest === undefined ||
      handledFilterRequest.current === filterColumnRequest.token
    )
      return;
    handledFilterRequest.current = filterColumnRequest.token;
    onFilterColumnRequestHandled?.();
    const column = editingFrame.columns.find(
      (candidate) => candidate.id === filterColumnRequest.columnId
    );
    if (!column) return;
    // This remains a local draft until Enter. Merely opening a header filter
    // must not silently remove rows from the frame.
    setSteps((current) => appendColumnFilter(current, column, filterColumnRequest.token));
  }, [
    editingFrame.columns,
    filterColumnRequest,
    onFilterColumnRequestHandled,
  ]);

  useEffect(() => {
    if (
      hidePipelineColumnRequest === undefined ||
      handledHideRequest.current === hidePipelineColumnRequest.token
    )
      return;
    handledHideRequest.current = hidePipelineColumnRequest.token;
    onHidePipelineColumnRequestHandled?.();
    const next = hidePipelineColumn(
      steps,
      hidePipelineColumnRequest.columnId,
      editingFrame.columns.map((column) => column.id)
    );
    if (!next) return;
    // Formatted for the identity-preserving reason the add request explains;
    // not through `persist`, because a refused hide deliberately leaves the
    // draft unchanged rather than showing a chain the engine rejected.
    const formatted = next.map(formatPipelineFormulas);
    lastSavedSignature.current = JSON.stringify(formatted.map(stepInput));
    void onOperation(
      {
        type: "setFramePipeline",
        frameId: editingFrame.id,
        steps: formatted.map(stepInput),
      },
      { inlineError: true }
    ).then((failure) => {
      setFormulaError(failure);
      if (!failure) setSteps(formatted);
    });
  }, [
    hidePipelineColumnRequest,
    editingFrame.columns,
    editingFrame.id,
    onHidePipelineColumnRequestHandled,
    onOperation,
    steps,
  ]);

  useEffect(() => {
    if (
      rearrangeColumnsRequest === undefined ||
      handledRearrangeRequest.current === rearrangeColumnsRequest.token
    )
      return;
    handledRearrangeRequest.current = rearrangeColumnsRequest.token;
    onRearrangeColumnsRequestHandled?.();
    const next = rearrangePipelineColumns(steps, rearrangeColumnsRequest.columnIds);
    void persist(next);
  }, [rearrangeColumnsRequest, onRearrangeColumnsRequestHandled, persist, steps]);

  // A save error describes the exact draft that was submitted. Once any
  // step changes it is historical, and leaving it visible makes a corrected
  // formula look as though it is still being rejected before Save is tried.
  useEffect(() => setFormulaError(null), [steps]);

  // The draft as the core reads it. Memoized so a scope object handed to a
  // formula editor is stable between keystrokes in *other* steps, which is
  // what keeps its completion effect from refiring on every edit anywhere.
  const stepScopeInputs = useMemo(() => steps.map(stepInput), [steps]);

  const { preview, previewOf } = usePipelineSchemaPreview(
    editingFrame.id,
    stepScopeInputs
  );

  /**
   * The columns a step can read.
   *
   * The core's answer when it reaches that far, because it carries real
   * types and agrees with what will actually run. The local walk is the
   * fallback for a draft the core could not get through — typically the
   * step being typed into right now, which is exactly when a stale answer
   * would be worse than a guessed one.
   */
  const visibleBeforeStep = (index: number): VisibleColumn[] => {
    const columns =
      index === 0 ? preview?.inputColumns : preview?.steps[index - 1]?.columns;
    return (
      columns?.map((column) => ({
        id: column.id,
        name: column.name,
        dataType: column.dataType,
      })) ?? columnsBeforeStep(input.columns, steps, index)
    );
  };

  const patch = (stepId: string, update: (step: StepDraft) => StepDraft) =>
    setSteps((current) =>
      current.map((step) => (step.id === stepId ? update(step) : step))
    );
  const removeStep = (stepId: string) => {
    const index = steps.findIndex((step) => step.id === stepId);
    if (index < 0) return;
    const count = isOrderingOnlySelect(input.columns, steps, index + 1) ? 2 : 1;
    void persist(
      steps.filter(
        (_, candidateIndex) => candidateIndex < index || candidateIndex >= index + count
      )
    );
  };

  const moveStep = (fromId: string, toId: string) => {
    const from = steps.findIndex((step) => step.id === fromId);
    const to = steps.findIndex((step) => step.id === toId);
    if (from < 0 || to < 0 || from === to) return;
    const next = [...steps];
    const count = isOrderingOnlySelect(input.columns, steps, from + 1) ? 2 : 1;
    const moved = next.splice(from, count);
    const target = next.findIndex((step) => step.id === toId);
    if (target < 0) return;
    const targetCount = isOrderingOnlySelect(input.columns, next, target + 1) ? 2 : 1;
    const insertion = from < to ? target + targetCount : target;
    next.splice(insertion, 0, ...moved);
    void persist(next);
  };

  const savePatch = async (stepId: string, update: (step: StepDraft) => StepDraft) =>
    persist(steps.map((step) => (step.id === stepId ? update(step) : step)));
  /**
   * savePatch for the commit gesture — Return in a formula surface. A save
   * the engine refuses rejects instead of resolving, because the session
   * committing it may be hosted by the formula bar with this panel closed:
   * `formulaError` renders only here, the registry keeps a refused session
   * alive only when its commit throws, and the bar shows the refusal it
   * catches. Button- and drag-driven saves stay on savePatch — those
   * gestures exist only inside this panel, where the inline error is
   * already in view.
   */
  const commitPatch = async (
    stepId: string,
    update: (step: StepDraft) => StepDraft
  ) => {
    const failure = await savePatch(stepId, update);
    if (failure) throw new Error(failure);
  };

  const commandId = (step: StepDraft, itemId?: string) =>
    `pipeline:${editingFrame.id}:${step.id}:${itemId ?? step.kind}`;
  const commandFocus = (id: string, requested?: number) =>
    pendingEditor === id ? requested ?? 1 : requested;
  // The parse-refusal half of commitPatch's contract: every caller is a
  // commit gesture, so the throw reaches the surface hosting the session.
  const rejectCommand = (message: string): never => {
    setFormulaError(message);
    throw new Error(message);
  };

  return (
    <div className="derived-creator pipeline-outline">
      <PipelineHeading
        hasGeneratedColumns={visibleAuthored.some(({ step }) => step.kind === "pivot")}
        refreshing={refreshingGeneratedColumns}
        onRefresh={() => {
          setRefreshingGeneratedColumns(true);
          void persist(steps).finally(() => setRefreshingGeneratedColumns(false));
        }}
      />
      {fixedJoinStep(editingFrame, joinFrames, onOperation)}
      {visibleAuthored.map(({ step, index }, displayIndex) => {
        const visible = visibleBeforeStep(index);
        const stepReferences = referencesForStep(references, visible);
        const columnReferences = stepReferences.filter(
          (reference) => reference.kind === "column"
        );
        const columnListReferences: FormulaReference[] = [
          ...columnReferences,
          ...[
            ["starts_with", 'starts_with("', "Columns whose names start with text"],
            ["ends_with", 'ends_with("', "Columns whose names end with text"],
            ["contains", 'contains("', "Columns whose names contain text"],
            ["except", "except(", "Every column except those named"],
          ].map(([label, token, detail]) => ({
            id: `selector.${label}`,
            label,
            token,
            kind: "function" as const,
            detail,
          })),
        ];
        const scope = { steps: stepScopeInputs, stepIndex: index };
        const stepFailure =
          previewOf === stepScopeInputs && preview?.failedStep === index
            ? preview.error
            : null;
        return (
          <section
            key={step.id}
            className={`pipeline-step ${dragging === step.id ? "dragging" : ""} ${
              dragOver === step.id ? "drag-target" : ""
            }`}
            onDragOver={(event) => {
              event.preventDefault();
              setDragOver(step.id);
            }}
            onDragLeave={() =>
              setDragOver((current) => (current === step.id ? null : current))
            }
            onDrop={(event) => {
              event.preventDefault();
              if (dragging) moveStep(dragging, step.id);
              setDragging(null);
              setDragOver(null);
            }}
          >
            <div className="pipeline-step-heading">
              <span
                className="pipeline-step-index"
                draggable
                title="Drag to reorder"
                onDragStart={() => setDragging(step.id)}
                onDragEnd={() => {
                  setDragging(null);
                  setDragOver(null);
                }}
              >
                {displayIndex + authoredStepNumberOffset}
              </span>
              <strong>
                {step.kind === "select"
                  ? selectStepLabel(step)
                  : STEP_LABELS[step.kind]}
              </strong>
              <button
                className="remove-derived-row"
                title="Remove step"
                onClick={() => removeStep(step.id)}
              >
                <X size={12} />
              </button>
            </div>

            {step.kind === "comment" && (
              <CommentStepRow
                text={step.text}
                startEditing={pendingEditor === commandId(step) || !step.text}
                onCommit={(draft) => {
                  const trimmed = draft.trim();
                  if (!trimmed) return removeStep(step.id);
                  if (trimmed === step.text) return;
                  void savePatch(step.id, (current) =>
                    current.kind === "comment" ? { ...current, text: trimmed } : current
                  );
                }}
              />
            )}

            {step.kind === "filter" && (
              <PipelineFilterStep
                step={step}
                stepReferences={stepReferences}
                input={input}
                scope={scope}
                commandId={commandId}
                commandFocus={commandFocus}
                patch={patch}
                commitPatch={commitPatch}
                savePatch={savePatch}
                rejectCommand={rejectCommand}
                setPendingEditor={setPendingEditor}
              />
            )}

            {step.kind === "withColumns" && (
              <PipelineWithColumnsStep
                step={step}
                visible={visible}
                stepReferences={stepReferences}
                input={input}
                scope={scope}
                editingFrame={editingFrame}
                commandId={commandId}
                commandFocus={commandFocus}
                patch={patch}
                commitPatch={commitPatch}
                savePatch={savePatch}
                rejectCommand={rejectCommand}
                setPendingEditor={setPendingEditor}
              />
            )}

            {step.kind === "recurrence" && (
              <PipelineRecurrenceStep
                step={step}
                references={stepReferences}
                columnNames={visible.map((column) => column.name)}
                frameId={input.completionFrameId}
                editingFrameId={editingFrame.id}
                scope={scope}
                seedEditorId={commandId(step, "seed")}
                nextEditorId={commandId(step, "next")}
                focusToken={commandFocus(
                  commandId(step, "next"),
                  step.focusToken
                )}
                onChange={(update) =>
                  patch(step.id, (current) =>
                    current.kind === "recurrence"
                      ? { ...current, ...update }
                      : current
                  )
                }
                onCommit={(update) =>
                  commitPatch(step.id, (current) =>
                    current.kind === "recurrence"
                      ? { ...current, ...update }
                      : current
                  )
                }
                onReject={rejectCommand}
              />
            )}

            {step.kind === "select" && step.mode === "rearrange" && (
              <PipelineRearrangeStep
                step={step}
                visible={visible}
                draggingColumn={draggingColumn}
                setDraggingColumn={setDraggingColumn}
                columnDrop={columnDrop}
                setColumnDrop={setColumnDrop}
                savePatch={savePatch}
              />
            )}

            {step.kind === "select" && step.mode !== "rearrange" && (
              <PipelineCommand
                editorId={commandId(step)}
                label={selectStepLabel(step)}
                initialDraft={columnListCommand(
                  visible
                    .filter((column) => !step.columnIds.includes(column.id))
                    .map((column) => column.id),
                  visible
                )}
                references={columnListReferences}
                focusToken={commandFocus(commandId(step))}
                onChange={(draft) => {
                  const ids = meltedColumnIds(draft, visible);
                  if (ids.length > 0 && ids.length < visible.length)
                    patch(step.id, (current) =>
                      current.kind === "select"
                        ? {
                            ...current,
                            columnIds: visible
                              .filter((column) => !ids.includes(column.id))
                              .map((column) => column.id),
                          }
                        : current
                    );
                }}
                onCommit={(draft) => {
                  const ids = meltedColumnIds(draft, visible);
                  if (ids.length === 0)
                    return rejectCommand("Name at least one column to delete");
                  if (ids.length === visible.length)
                    return rejectCommand("A frame needs at least one column");
                  const kept = visible
                    .filter((column) => !ids.includes(column.id))
                    .map((column) => column.id);
                  return commitPatch(step.id, (current) =>
                    current.kind === "select"
                      ? { ...current, columnIds: kept }
                      : current
                  );
                }}
              />
            )}

            {step.kind === "summarize" && (
              <PipelineSummarizeStep
                step={step}
                visible={visible}
                stepReferences={stepReferences}
                input={input}
                scope={scope}
                commandId={commandId}
                commandFocus={commandFocus}
                patch={patch}
                commitPatch={commitPatch}
                savePatch={savePatch}
                rejectCommand={rejectCommand}
                setPendingEditor={setPendingEditor}
              />
            )}

            {step.kind === "sort" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const keys = parseSortCommand(draft, visible);
                  if (!keys) {
                    if (saveNow)
                      rejectCommand(
                        "Write one or more backticked columns with asc or desc"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "sort" ? { ...current, keys } : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Sort"
                    initialDraft={sortCommand(step, visible)}
                    references={columnReferences}
                    focusToken={commandFocus(commandId(step))}
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {step.kind === "union" &&
              <PipelineFrameStepCommand
                editorId={commandId(step)}
                label="Stack frame"
                frameId={step.frameId}
                frames={frames}
                focusToken={commandFocus(commandId(step))}
                resolveName={exactName}
                onInvalid={() => rejectCommand("Choose a frame to stack")}
                onSelect={(frameId, saveNow) => {
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "union" ? { ...current, frameId } : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                }}
              />}

            {step.kind === "expand" &&
              <PipelineFrameStepCommand
                editorId={commandId(step)}
                label="Expand frame"
                frameId={step.frameId}
                frames={frames}
                focusToken={commandFocus(commandId(step))}
                resolveName={exactName}
                onInvalid={() => rejectCommand("Choose a frame to expand with")}
                onSelect={(frameId, saveNow) => {
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "expand" ? { ...current, frameId } : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                }}
              />}

            {step.kind === "pivot" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const parsed = parsePivotCommand(draft, visible);
                  if (!parsed) {
                    if (saveNow)
                      rejectCommand(
                        "Write columns=, values=, and a supported aggregate"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "pivot" ? { ...current, ...parsed } : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Pivot"
                    initialDraft={pivotCommand(step, visible)}
                    references={columnReferences}
                    focusToken={commandFocus(commandId(step))}
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {step.kind === "unpivot" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const parsed = parseUnpivotCommand(draft);
                  if (!parsed) {
                    if (saveNow)
                      rejectCommand(
                        "Write columns=, names=, and values= with backticked output names"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "unpivot" ? { ...current, ...parsed } : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Unpivot"
                    initialDraft={unpivotCommand(step)}
                    references={columnListReferences}
                    focusToken={commandFocus(commandId(step))}
                    focusSelection={
                      step.columns.trim()
                        ? undefined
                        : { start: "columns=".length, end: "columns=".length }
                    }
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {step.kind === "broadcast" &&
              (() => {
                const update = (
                  change: Partial<BroadcastStepDraft>,
                  saveNow: boolean
                ) => {
                  const patchStep = (current: StepDraft): StepDraft =>
                    current.kind === "broadcast" ? { ...current, ...change } : current;
                  if (saveNow) return commitPatch(step.id, patchStep);
                  patch(step.id, patchStep);
                };
                return (
                  <BroadcastStepRow
                    step={step}
                    visible={visible}
                    columnReferences={columnListReferences}
                    references={stepReferences}
                    columnsEditorId={commandId(step, "columns")}
                    vectorEditorId={commandId(step, "vector")}
                    columnsFocusToken={commandFocus(commandId(step, "columns"))}
                    vectorFocusToken={commandFocus(commandId(step, "vector"))}
                    onUpdate={update}
                  />
                );
              })()}

            {step.kind === "zipVector" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const parsed = parseNamedTransformation(draft);
                  if (!parsed) {
                    if (saveNow)
                      rejectCommand("Write a backticked column name = a list");
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "zipVector"
                      ? { ...current, name: parsed.name, vector: parsed.formula }
                      : current;
                  if (saveNow) return commitPatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <ZipVectorStepRow
                    step={step}
                    references={stepReferences}
                    editorId={commandId(step)}
                    focusToken={commandFocus(commandId(step))}
                    onDraft={update}
                  />
                );
              })()}

            {stepFailure && !stepIsIncomplete(step) && (
              <p className="pipeline-step-shape broken">
                <CircleAlert size={11} /> {stepFailure}
              </p>
            )}
          </section>
        );
      })}

      <label className="add-step compact-add-step">
        <select
          value=""
          aria-label="Add transformation"
          onChange={(event) => {
            const kind = event.target.value as AddStepKind;
            if (!kind) return;
            const step = blankStep(
              kind,
              columnsBeforeStep(input.columns, steps, steps.length),
              input.columns
            );
            setSteps((current) => [...current, step]);
            const itemId =
              step.kind === "filter"
                ? step.predicates[0]?.id
                : step.kind === "withColumns"
                ? step.columns[0]?.id
                : step.kind === "summarize"
                ? step.aggregates[0]?.id
                : undefined;
            setPendingEditor(commandId(step, itemId));
          }}
        >
          <option value="">+ Add transformation</option>
          <option value="filter">Filter rows</option>
          <option value="withColumns">Add or replace columns</option>
          <option value="deleteColumns">Delete columns</option>
          <option value="rearrangeColumns">Rearrange columns</option>
          <option value="summarize">Summarize</option>
          <option value="sort">Sort</option>
          <option value="union">Stack frame</option>
          <option value="expand">Expand frame</option>
          <option value="pivot">Pivot</option>
          <option value="unpivot">Unpivot</option>
          <option value="broadcast">Apply vector across columns</option>
          <option value="zipVector">Pair vector as column</option>
          <option value="comment">Comment</option>
        </select>
      </label>

      {formulaError && (
        <FormulaErrorDetails
          error={formulaError ?? ""}
          formulas={steps.flatMap(stepFormulas)}
        />
      )}
    </div>
  );
}
