import {
  AlignCenter,
  AlignLeft,
  AlignRight,
  Bold,
  CircleAlert,
  Copy,
  Database,
  GitBranch,
  GitMerge,
  Italic,
  KeyRound,
  Palette,
  Play,
  RefreshCw,
  Sparkles,
  Underline,
  Workflow,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { FormulaErrorDetails } from "./FormulaEditor";
import { QueryPlanDetails } from "./DebugTracePanel";
import { FormatColorField } from "./FormatColorField";
import { FILL_SWATCHES, INK_SWATCHES } from "./lib/palette";
import {
  ConditionalFormattingRules,
  type RuleTarget,
} from "./ConditionalFormattingRules";
import {
  ruleInput,
  ruleWithStopStyle,
  scalePropertyLabel,
  stopLabel,
  stopStyle,
} from "./lib/conditionalFormatting";
import { Field } from "./Field";
import {
  effectiveFrameCellStyle,
  emptyFrameCellStyle,
  exactFrameCellStyle,
  frameStyleRules,
  frameStyles,
  sameStyleTarget,
  styleTargetForSelection,
  type FreezeCopyHandler,
  type SetFrameCachedHandler,
  type SetFrameSourceHandler,
  type TakeOwnershipHandler,
} from "./FrameGrid";
import { DerivedFrameCreator } from "./PipelineEditor";
import { FrameSourcePanel } from "./FrameSourcePanel";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ColumnFormat,
  ColumnFormatScale,
  ColumnFormatStyle,
  ComputedFrame,
  DataObject,
  DataType,
  FormulaFunction,
  FrameCellStyle,
  FrameObject,
  Selection,
} from "./lib/types";
import type {
  AddCalculatedColumnEditorRequest,
  FilterColumnEditorRequest,
  HidePipelineColumnEditorRequest,
  RearrangeColumnsEditorRequest,
  TransformColumnEditorRequest,
} from "./hooks/usePipelineColumnRequests";
import type { InspectorSection } from "./App";

function canFormatColumn(column: Column): boolean {
  return (
    ["integer", "number", "currency", "percentage"].includes(column.dataType) ||
    Boolean(column.format)
  );
}

function ColumnFormatEditor({
  frame,
  column,
  onOperation,
}: {
  frame: FrameObject;
  column: Column;
  onOperation: OperationHandler;
}) {
  const format = column.format ?? null;
  const commit = (next: ColumnFormat | null) =>
    void onOperation({
      type: "setColumnFormat",
      frameId: frame.id,
      columnId: column.id,
      format: next,
    });
  const patch = (changes: Partial<ColumnFormat>) =>
    commit({ style: "number", ...format, ...changes });
  const isCurrency = format?.style === "currency" || format?.style === "accounting";
  return (
    <div className="column-format-editor">
      <label className="inspector-field">
        Number format
        <select
          value={format?.style ?? ""}
          onChange={(event) => {
            const style = event.target.value as ColumnFormatStyle | "";
            if (!style) commit(null);
            else patch({ style });
          }}
        >
          <option value="">Default</option>
          <option value="plain">Plain</option>
          <option value="number">Number</option>
          <option value="currency">Currency</option>
          <option value="accounting">Accounting</option>
          <option value="percent">Percent</option>
        </select>
      </label>
      {format && format.style !== "plain" && (
        <>
          <div className="column-format-row">
            <label className="inspector-field">
              Decimals
              <input
                type="number"
                min={0}
                max={8}
                placeholder="auto"
                value={format.decimals ?? ""}
                onChange={(event) =>
                  patch({
                    decimals:
                      event.target.value === ""
                        ? null
                        : Math.max(
                            0,
                            Math.min(8, Math.trunc(Number(event.target.value)))
                          ),
                  })
                }
              />
            </label>
            <label className="inspector-field">
              Scale
              <select
                value={format.scale ?? "units"}
                onChange={(event) =>
                  patch({ scale: event.target.value as ColumnFormatScale })
                }
              >
                <option value="units">Units</option>
                <option value="thousands">Thousands (K)</option>
                <option value="millions">Millions (M)</option>
              </select>
            </label>
            {isCurrency && (
              <Field
                label="Currency"
                initial={format.currencyCode ?? ""}
                onCommit={(code) =>
                  patch({ currencyCode: code.trim().toUpperCase() || null })
                }
              />
            )}
          </div>
          <label className="column-format-toggle">
            <input
              type="checkbox"
              checked={format.negativeParens ?? format.style === "accounting"}
              onChange={(event) => patch({ negativeParens: event.target.checked })}
            />
            Negatives in parentheses
          </label>
          <label className="column-format-toggle">
            <input
              type="checkbox"
              checked={format.zeroDash ?? format.style === "accounting"}
              onChange={(event) => patch({ zeroDash: event.target.checked })}
            />
            Zero as dash
          </label>
          <small className="column-format-note">
            Display only — stored values keep full precision.
          </small>
        </>
      )}
    </div>
  );
}
export function FrameInspector({
  documentId,
  frame,
  objects,
  formulaFunctions,
  selection,
  computed,
  suggestedPosition,
  section,
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
  onOperation,
  onSourceChanged,
  onSetCached,
  onTakeOwnership,
  onFreezeCopy,
  onJoin,
  onTransformColumn,
}: {
  documentId: string;
  frame: FrameObject;
  objects: DataObject[];
  formulaFunctions: FormulaFunction[];
  selection: Selection;
  computed: ComputedFrame;
  suggestedPosition: { x: number; y: number };
  section: InspectorSection;
  addCalculatedColumnRequest?: AddCalculatedColumnEditorRequest;
  onAddCalculatedColumnRequestHandled: () => void;
  transformColumnRequest?: TransformColumnEditorRequest;
  onTransformColumnRequestHandled: () => void;
  filterColumnRequest?: FilterColumnEditorRequest;
  onFilterColumnRequestHandled: () => void;
  hidePipelineColumnRequest?: HidePipelineColumnEditorRequest;
  onHidePipelineColumnRequestHandled: () => void;
  rearrangeColumnsRequest?: RearrangeColumnsEditorRequest;
  onRearrangeColumnsRequestHandled: () => void;
  onOperation: OperationHandler;
  onSourceChanged: SetFrameSourceHandler;
  onSetCached: SetFrameCachedHandler;
  onTakeOwnership: TakeOwnershipHandler;
  onFreezeCopy: FreezeCopyHandler;
  onJoin: () => void;
  onTransformColumn: (column: Column, formula: string, focus?: boolean) => void;
}) {
  // The card's object is the selected tab, and a tab is a frame.
  const activeFrameView = frame;
  const column = frame.columns.find((candidate) => candidate.id === selection.columnId);
  const sourceFrame = frame.derivation
    ? objects.find(
        (candidate): candidate is FrameObject =>
          candidate.kind === "frame" && candidate.id === frame.derivation!.sourceFrameId
      )
    : frame;
  const formulaFrame = frame.derivation?.join ? frame : sourceFrame ?? frame;
  // Where this frame's chain starts. A derived frame reads the frame it
  // derives from; a source frame reads its own data, whose schema is
  // `baseColumns` once a chain exists — `columns` is the chain's output by
  // then. `completionFrameId` is only supplied when the core would resolve
  // backtick names against that same schema; otherwise the per-step
  // reference list below is the accurate one and typed completion sits out.
  const chainInput = frame.derivation
    ? frame.derivation.join
      ? {
          label: "Source: joined result",
          columns: frame.baseColumns?.length ? frame.baseColumns : frame.columns,
          // A later step may have changed the final frame schema. Local,
          // per-step references therefore describe the join input more
          // accurately than completion against the final frame object.
          completionFrameId: undefined,
        }
      : sourceFrame && {
        label: `Source: ${sourceFrame.name}`,
        columns: sourceFrame.columns,
        completionFrameId: frame.id,
      }
    : {
        label: "Source: this frame’s own data",
        columns: frame.baseColumns?.length ? frame.baseColumns : frame.columns,
        completionFrameId: frame.baseColumns?.length ? undefined : frame.id,
      };
  const references = useMemo<FormulaReference[]>(() => {
    // The name a canvas object answers to in a formula: the containers it
    // sits in, outermost first, then its own name — the same path the core
    // writes back when it renders a formula.
    const containerOf = new Map(
      objects.flatMap((candidate) =>
        candidate.kind === "container"
          ? candidate.memberIds.map((memberId) => [memberId, candidate] as const)
          : []
      )
    );
    const qualifiedPath = (objectId: string, name: string) => {
      const path = [name];
      for (
        let container = containerOf.get(objectId);
        container;
        container = containerOf.get(container.id)
      )
        path.unshift(container.name);
      return path;
    };
    return [
        {
          id: formulaFrame.id,
          objectId: formulaFrame.id,
          label: formulaFrame.name,
          token: `${formulaToken(formulaFrame.name)}.`,
          kind: "frame" as const,
          detail: `${formulaFrame.columns.length} columns`,
        },
        ...formulaFrame.columns.map((candidate) => ({
          id: candidate.id,
          objectId: formulaFrame.id,
          frameId: formulaFrame.id,
          label: candidate.name,
          token: formulaToken(candidate.name),
          kind: "column" as const,
          detail: `${candidate.dataType} column in ${formulaFrame.name}`,
        })),
        ...objects.flatMap((candidate) => {
          if (candidate.kind !== "value" && candidate.kind !== "result") return [];
          const path = qualifiedPath(candidate.id, candidate.name);
          return [
            {
              id: candidate.id,
              objectId: candidate.id,
              label: path.join("."),
              token: path.map(formulaToken).join("."),
              kind: "value" as const,
              detail:
                candidate.kind === "value"
                  ? `Canvas value · ${candidate.raw}`
                  : "Computed result",
            },
          ];
        }),
        ...formulaFunctions.map((candidate) => ({
          id: candidate.id,
          label: candidate.name,
          token: `${candidate.name}(`,
          kind: "function" as const,
          detail: `${candidate.signature} → ${candidate.returnType} · ${candidate.description}`,
          searchTerms: candidate.aliases,
          signature: candidate.signature,
          description: candidate.description,
          arguments: candidate.arguments,
        })),
      ].filter((reference) => reference.token.length > 0);
  }, [formulaFunctions, formulaFrame, objects]);
  const selectionLabel =
    selection.rowId && column
      ? "Cell"
      : selection.rowId
      ? "Row"
      : column
      ? "Column"
      : "Frame";
  const styleTarget = styleTargetForSelection(selection);
  // A rule stop, when one is being dressed, is what the format controls
  // below point at instead of the selection. Same controls, same gesture:
  // bold is bold whether it lands on a cell or on the rule that paints it.
  const [ruleTarget, setRuleTarget] = useState<RuleTarget | null>(null);
  useEffect(() => setRuleTarget(null), [frame.id]);
  const styleRule = ruleTarget
    ? frameStyleRules(frame).find((rule) => rule.id === ruleTarget.ruleId)
    : undefined;
  const activeStop = styleRule && ruleTarget ? ruleTarget.stop : null;
  const exactStyle =
    styleRule && activeStop
      ? stopStyle(styleRule, activeStop)
      : activeFrameView
      ? exactFrameCellStyle(activeFrameView, styleTarget)
      : emptyFrameCellStyle();
  // A stop has no cascade above it: it *is* the layer, so what it holds and
  // what it shows are the same thing.
  const effectiveStyle =
    styleRule && activeStop
      ? exactStyle
      : activeFrameView
      ? effectiveFrameCellStyle(activeFrameView, selection.rowId, selection.columnId)
      : emptyFrameCellStyle();
  // A ramp end holds a color and nothing else, so the controls that cannot
  // reach it are shown inert rather than silently doing nothing.
  const colorOnly = styleRule?.output.kind === "scale" && Boolean(activeStop);
  const activeScale = styleRule?.output.kind === "scale" ? styleRule.output.scale : null;
  const clearingScaleMid = activeStop?.kind === "scale" && activeStop.end === "mid";
  const pendingStyleRef = useRef(exactStyle);
  useEffect(() => {
    pendingStyleRef.current = exactStyle;
  }, [activeFrameView?.id, exactStyle, ruleTarget, selection.columnId, selection.rowId]);
  const setDirectStyle = (patch: Partial<FrameCellStyle> | null) => {
    if (!activeFrameView) return;
    const style = patch
      ? { ...pendingStyleRef.current, ...patch }
      : emptyFrameCellStyle();
    pendingStyleRef.current = style;
    if (styleRule && activeStop) {
      const rules = frameStyleRules(activeFrameView);
      const edited = ruleWithStopStyle(styleRule, activeStop, style);
      void onOperation({
        type: "setFrameStyleRules",
        frameId: activeFrameView.id,
        rules: rules.map((rule) =>
          ruleInput(rule.id === edited.id ? edited : rule, computed.styleRuleFormulas ?? {})
        ),
      });
      return;
    }
    void onOperation({
      type: "setFrameStyle",
      frameId: activeFrameView.id,
      target: styleTarget,
      style,
    });
  };

  // Hoisted out of the section list because two tabs render it. Selecting
  // a frame without picking a column out of it *is* selecting the frame,
  // and the pane that used to answer that with a row count and a note
  // saying frame settings were elsewhere is the pane they are in.
  const frameSettings = (
      <div className="inspector-section-stack">
        <Field
          label="Frame name"
          initial={frame.name}
          onCommit={(name) =>
            onOperation({ type: "renameObject", objectId: frame.id, name })
          }
        />
        {/* Why the grid will not take a typed value, said once in the
            place someone looks when they wonder. A cell that silently
            refuses to enter edit mode reads as a bug. */}
        {computed?.editing.reason && (
          <div className="editing-reason">
            <p>
              <KeyRound size={12} /> {computed.editing.reason}
            </p>
            {/* The way out, offered where the wall is. A refusal that
                explains itself and then leaves you to find the remedy in
                a menu is only half an explanation. */}
            {!computed.editing.cells && (
              <OwnRowsActions
                frame={frame}
                suggestedPosition={suggestedPosition}
                onTakeOwnership={onTakeOwnership}
                onFreezeCopy={onFreezeCopy}
              />
            )}
          </div>
        )}
        {/* A connector without an artifact behind it is still a frame that
            reads a file, and gating on the artifact alone hid the only place
            to repoint one. A frame somebody typed in has no source to manage
            and is still offered nothing. */}
        {(frame.artifact || frame.connector || frame.sourceFile) && (
          <FrameSourcePanel
            key={frame.id}
            frame={frame}
            onSourceChanged={onSourceChanged}
          />
        )}
        {frame.derivation && (
          <FrameCachePanel
            frame={frame}
            computed={computed}
            onSetCached={onSetCached}
          />
        )}
        {frame.derivation?.join &&
          (() => {
            const lookup = objects.find(
              (candidate): candidate is FrameObject =>
                candidate.kind === "frame" &&
                candidate.id === frame.derivation!.join!.lookupFrameId
            );
            const primaryKey = sourceFrame?.columns.find(
              (candidate) =>
                candidate.id === frame.derivation!.join!.primaryKeyColumnIds[0]
            );
            const lookupKey = lookup?.columns.find(
              (candidate) =>
                candidate.id === frame.derivation!.join!.lookupKeyColumnIds[0]
            );
            const joinType = frame.derivation.join.joinType;
            const policyLabel =
              joinType === "left"
                ? `Keep every ${sourceFrame?.name ?? "source"} row`
                : joinType === "inner"
                ? `Matched ${sourceFrame?.name ?? "source"} row`
                : joinType === "anti"
                ? `${sourceFrame?.name ?? "Source"} rows without a match (anti)`
                : `${sourceFrame?.name ?? "Source"} rows with a match (semi)`;
            return (
              <div className="join-summary">
                <GitMerge size={16} />
                <div>
                  <strong>{policyLabel}</strong>
                  <span>
                    {primaryKey?.name} matches {lookup?.name}.{lookupKey?.name}
                  </span>
                </div>
              </div>
            );
          })()}
        <button className="secondary-action branch-derived-action" onClick={onJoin}>
          <GitMerge size={13} /> Join another frame
        </button>
        <button
          className="secondary-action branch-derived-action"
          onClick={() =>
            void onOperation({
              type: "addLinkedFrame",
              sourceFrameId: frame.id,
              name: `${frame.name} frame`,
              ...suggestedPosition,
            })
          }
        >
          <GitBranch size={13} /> Create frame from this
        </button>
        <QueryPlanDetails frameId={frame.id} />
        <div className="info-panel">
          <Sparkles size={16} />
          <p>
            {frame.derivation
              ? "This result refreshes automatically from its source and can branch into more derived frames."
              : "Calculated columns and frame branches live here; each branch has its own Wrangle chain."}
          </p>
        </div>
      </div>
  );

  return (
    <div className="inspector-content frame-inspector-content">
      {section === "selection" &&
        (!column ? (
          // No column picked out of it means the frame is what is selected,
          // so this is its own pane rather than a signpost to one. The Frame
          // tab still holds the same controls, for reaching them without
          // giving up the column you are working on.
          frameSettings
        ) : (
          <div className="inspector-section-stack">
            <div className="selection-crumb">
              {frame.name} <span>/</span> {column.name}
              {selection.rowId ? (
                <>
                  <span>/</span> selected row
                </>
              ) : null}
            </div>
            <Field
              label="Column name"
              initial={column.name}
              onCommit={(name) =>
                onOperation({
                  type: "renameColumn",
                  frameId: frame.id,
                  columnId: column.id,
                  name,
                })
              }
            />
            <>
              <label className="inspector-field">
                Column type
                <select
                  value={column.dataType}
                  onChange={(event) => {
                    const dataType = event.target.value as DataType;
                    if (computed.editing.rows) {
                      void onOperation({
                        type: "setColumnType",
                        frameId: frame.id,
                        columnId: column.id,
                        dataType,
                      });
                    } else {
                      onTransformColumn(
                        column,
                        `${formulaToken(column.name)}.cast("${dataType}")`
                      );
                    }
                  }}
                >
                  <option value="string">Text</option>
                  <option value="categorical" disabled={!computed.editing.rows}>
                    Categorical
                  </option>
                  <option value="integer">Integer</option>
                  <option value="number">Number</option>
                  <option value="currency" disabled={!computed.editing.rows}>
                    Currency
                  </option>
                  <option value="percentage" disabled={!computed.editing.rows}>
                    Percentage
                  </option>
                  <option value="boolean">Boolean</option>
                  <option value="date">Date</option>
                </select>
              </label>
              {computed.editing.rows && (
                <>
                  {column.dataType === "categorical" && (
                    <Field
                      label="Allowed values"
                      help="In order — this is how the column sorts and compares."
                      initial={(column.categories ?? []).join(", ")}
                      onCommit={(raw) =>
                        onOperation({
                          type: "setColumnCategories",
                          frameId: frame.id,
                          columnId: column.id,
                          categories: raw
                            .split(",")
                            .map((category) => category.trim())
                            .filter(Boolean),
                        })
                      }
                    />
                  )}
                <button
                  className="secondary-action key-action"
                  onClick={() =>
                    onOperation({
                      type: "setUniqueKey",
                      frameId: frame.id,
                      columnIds: [column.id],
                      enabled: !frame.uniqueKeys.some(
                        (key) =>
                          key.columnIds.length === 1 && key.columnIds[0] === column.id
                      ),
                    })
                  }
                >
                  <KeyRound size={14} />{" "}
                  {frame.uniqueKeys.some(
                    (key) =>
                      key.columnIds.length === 1 && key.columnIds[0] === column.id
                  )
                    ? "Remove unique key"
                    : "Mark as unique key"}
                </button>
                </>
              )}
            </>
            {frame.derivation && (
              <div className="info-panel">
                <GitBranch size={16} />
                <p>
                  Derived cells are read-only. Double-click a header in the frame to
                  change its alias.
                </p>
              </div>
            )}
            {(frame.sourceFile || frame.artifact) && (
              <div className="info-panel">
                <Database size={16} />
                <p>
                  This frame is an immutable, paged snapshot
                  {frame.artifact?.sourceName ? ` of ${frame.artifact.sourceName}` : ""}
                  .{" "}
                  {frame.connector
                    ? "Refresh it from the Frame section when the source file changes. "
                    : ""}
                  Filter, add columns, or summarize it on the Wrangle tab.
                </p>
              </div>
            )}
          </div>
        ))}

      {section === "format" && (
        <div className="inspector-section-stack">
          {/* The one strip that says what the controls below will change.
              A rule stop is a formatting target like any other, so it is
              said here rather than by a second panel saying it again. */}
          <div
            className={`selection-summary compact${styleRule && activeStop ? " rule-target" : ""}`}
          >
            <span>Formatting target</span>
            <strong>
              {styleRule && activeStop
                ? `Rule · ${stopLabel(styleRule, activeStop)}`
                : selectionLabel}
            </strong>
            <small>
              {styleRule && activeStop
                ? `${computed.styleRuleFormulas?.[styleRule.id] ?? ""}${
                    styleRule.output.kind === "scale"
                      ? ` · ${scalePropertyLabel(styleRule.output.scale)}`
                      : ""
                  }`
                : column
                ? `${frame.name} / ${column.name}`
                : frame.name}
            </small>
          </div>
          {column && canFormatColumn(column) && !styleRule && (
            <ColumnFormatEditor
              key={`format-${column.id}`}
              frame={frame}
              column={column}
              onOperation={onOperation}
            />
          )}
          <div className="format-preview-controls" aria-label="Formatting controls">
            <div>
              <span>Typeface</span>
              <div className="format-button-row">
                <button
                  className={effectiveStyle.bold ? "active" : ""}
                  aria-label="Bold"
                  aria-pressed={Boolean(effectiveStyle.bold)}
                  disabled={colorOnly}
                  onClick={() => setDirectStyle({ bold: !effectiveStyle.bold })}
                >
                  <Bold size={15} />
                </button>
                <button
                  className={effectiveStyle.italic ? "active" : ""}
                  aria-label="Italic"
                  aria-pressed={Boolean(effectiveStyle.italic)}
                  disabled={colorOnly}
                  onClick={() => setDirectStyle({ italic: !effectiveStyle.italic })}
                >
                  <Italic size={15} />
                </button>
                <button
                  className={effectiveStyle.underline ? "active" : ""}
                  aria-label="Underline"
                  aria-pressed={Boolean(effectiveStyle.underline)}
                  disabled={colorOnly}
                  onClick={() =>
                    setDirectStyle({ underline: !effectiveStyle.underline })
                  }
                >
                  <Underline size={15} />
                </button>
              </div>
            </div>
            <FormatColorField
              label="Text color"
              swatches={INK_SWATCHES}
              perRow={9}
              value={effectiveStyle.textColor}
              exact={exactStyle.textColor}
              fallback="#20221f"
              documentId={documentId}
              property="text"
              canReset={!activeScale || clearingScaleMid || Boolean(activeScale.fill)}
              resetLabel={activeScale ? "Clear" : "Reset"}
              onChange={(textColor) => setDirectStyle({ textColor })}
            />
            <FormatColorField
              label="Fill color"
              swatches={FILL_SWATCHES}
              perRow={9}
              value={effectiveStyle.fillColor}
              exact={exactStyle.fillColor}
              fallback="#ffffff"
              documentId={documentId}
              property="fill"
              canReset={!activeScale || clearingScaleMid || Boolean(activeScale.text)}
              resetLabel={activeScale ? "Clear" : "Reset"}
              onChange={(fillColor) => setDirectStyle({ fillColor })}
            />
            <div>
              <span>Alignment</span>
              <div className="format-button-row">
                <button
                  className={effectiveStyle.alignment === "left" ? "active" : ""}
                  aria-label="Align left"
                  disabled={colorOnly}
                  onClick={() => setDirectStyle({ alignment: "left" })}
                >
                  <AlignLeft size={15} />
                </button>
                <button
                  className={effectiveStyle.alignment === "center" ? "active" : ""}
                  aria-label="Align center"
                  disabled={colorOnly}
                  onClick={() => setDirectStyle({ alignment: "center" })}
                >
                  <AlignCenter size={15} />
                </button>
                <button
                  className={effectiveStyle.alignment === "right" ? "active" : ""}
                  aria-label="Align right"
                  disabled={colorOnly}
                  onClick={() => setDirectStyle({ alignment: "right" })}
                >
                  <AlignRight size={15} />
                </button>
              </div>
            </div>
            <label className="format-select-row">
              <span>Line style</span>
              <select
                aria-label="Line style"
                value={effectiveStyle.lineStyle ?? ""}
                disabled={colorOnly}
                onChange={(event) =>
                  setDirectStyle({
                    lineStyle: (event.target.value ||
                      null) as FrameCellStyle["lineStyle"],
                  })
                }
              >
                <option value="">None</option>
                <option value="solid">Solid</option>
                <option value="dashed">Dashed</option>
                <option value="dotted">Dotted</option>
                <option value="double">Double</option>
              </select>
            </label>
          </div>
          {styleRule && activeStop ? (
            <button
              className="secondary-action clear-formatting"
              onClick={() => setRuleTarget(null)}
            >
              <X size={13} /> Back to formatting the {selectionLabel.toLowerCase()}
            </button>
          ) : (
            <button
              className="secondary-action clear-formatting"
              disabled={
                !frameStyles(activeFrameView).some((entry) =>
                  sameStyleTarget(entry.target, styleTarget)
                )
              }
              onClick={() => setDirectStyle(null)}
            >
              <X size={13} /> Clear formatting for this {selectionLabel.toLowerCase()}
            </button>
          )}
          <ConditionalFormattingRules
            frame={frame}
            computed={computed}
            selection={selection}
            references={references}
            target={ruleTarget}
            onTarget={setRuleTarget}
            onOperation={onOperation}
          />
          <div className="info-panel">
            <Palette size={16} />
            <p>
              Formatting is stored on {activeFrameView?.name ?? "this view"}. Other tabs
              can present the same frame differently. Rules paint over it, in order.
            </p>
          </div>
        </div>
      )}

      {section === "wrangle" && (
        <div className="inspector-section-stack">
          {chainInput ? (
            <DerivedFrameCreator
              key={frame.id}
              input={chainInput}
              editingFrame={frame}
              renderedSteps={computed.steps ?? []}
              passThroughSteps={computed.passThroughSteps ?? 0}
              references={references}
              frames={objects
                .filter(
                  (candidate): candidate is FrameObject =>
                    candidate.kind === "frame" && candidate.id !== frame.id
                )
                .map((candidate) => ({ id: candidate.id, name: candidate.name }))}
              addCalculatedColumnRequest={addCalculatedColumnRequest}
              onAddCalculatedColumnRequestHandled={
                onAddCalculatedColumnRequestHandled
              }
              transformColumnRequest={transformColumnRequest}
              onTransformColumnRequestHandled={
                onTransformColumnRequestHandled
              }
              filterColumnRequest={filterColumnRequest}
              onFilterColumnRequestHandled={onFilterColumnRequestHandled}
              hidePipelineColumnRequest={hidePipelineColumnRequest}
              onHidePipelineColumnRequestHandled={
                onHidePipelineColumnRequestHandled
              }
              rearrangeColumnsRequest={rearrangeColumnsRequest}
              onRearrangeColumnsRequestHandled={
                onRearrangeColumnsRequestHandled
              }
              onOperation={onOperation}
            />
          ) : (
            <div className="empty-inspector-section">
              <Workflow size={20} />
              <strong>
                This frame’s source is missing
              </strong>
              <p>
                The frame this one derives from is not in the document, so there is no
                schema to write steps against.
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * The two ways to get values you can type into: convert this frame, or take
 * a frozen copy and leave it alone.
 *
 * Both write the same parquet beside the document. The difference is what
 * happens to the frame you started from, which is the only part worth
 * deciding: converting costs you the connector or the chain, and copying
 * costs you nothing but a second card. Copying is offered first for that
 * reason — it is the one that cannot lose anything.
 */
function OwnRowsActions({
  frame,
  suggestedPosition,
  onTakeOwnership,
  onFreezeCopy,
}: {
  frame: FrameObject;
  suggestedPosition: { x: number; y: number };
  onTakeOwnership: TakeOwnershipHandler;
  onFreezeCopy: FreezeCopyHandler;
}) {
  const [busy, setBusy] = useState<"freeze" | "own" | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const loses = frame.derivation
    ? "its chain stops recomputing"
    : "refreshing from its file stops";

  const run = (mode: "freeze" | "own", action: () => Promise<string | null>) => {
    setBusy(mode);
    setFailure(null);
    void action()
      .then(setFailure)
      .finally(() => {
        setBusy(null);
        setConfirming(false);
      });
  };

  return (
    <>
      <button
        className="secondary-action"
        disabled={busy !== null}
        onClick={() => run("freeze", () => onFreezeCopy(frame.id, suggestedPosition))}
      >
        <Copy size={13} />
        {busy === "freeze" ? "Copying…" : "Freeze a copy"}
      </button>
      <button
        className="secondary-action"
        disabled={busy !== null}
        onClick={() => {
          if (!confirming) {
            setConfirming(true);
            return;
          }
          run("own", () => onTakeOwnership(frame.id, { inlineError: true }));
        }}
      >
        <Database size={13} />
        {busy === "own"
          ? "Copying…"
          : confirming
          ? `Yes — ${loses}`
          : "Take ownership of these rows"}
      </button>
      <small className="own-rows-note">
        {confirming
          ? "The values you see now become this frame's own data, stored beside the document and editable. Undo puts it back."
          : "A frozen copy is a second frame holding these values, editable, with nothing left to refresh or recompute it."}
      </small>
      {failure && <FormulaErrorDetails title="Could not copy the rows" error={failure} />}
    </>
  );
}

/**
 * Live-or-cached control for a derived frame. A live frame recomputes its
 * transformation on every read; a cached one reads a parquet snapshot and
 * knows its own row count. Caching is offered, never applied automatically,
 * and a stale snapshot is stated plainly rather than silently refreshed --
 * for accounting numbers, quietly serving old values is worse than being
 * slow, and quietly recomputing defeats the point of caching at all.
 */
function FrameCachePanel({
  frame,
  computed,
  onSetCached,
}: {
  frame: FrameObject;
  computed: ComputedFrame;
  onSetCached: SetFrameCachedHandler;
}) {
  const [busy, setBusy] = useState<"cache" | "clear" | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const cache = computed.materialization;
  const run = (cached: boolean, mode: "cache" | "clear") => {
    setBusy(mode);
    setFailure(null);
    void onSetCached(frame.id, cached, { inlineError: true })
      .then(setFailure)
      .finally(() => setBusy(null));
  };

  return (
    <div className="connector-refresh-panel frame-cache-panel">
      <div>
        <Database size={15} />
        <span>
          <strong>
            {cache ? (cache.stale ? "Cached · out of date" : "Cached") : "Live"}
          </strong>
          <small>
            {cache
              ? `${cache.rowCount.toLocaleString()} rows in a snapshot`
              : "Recomputed from the source on every read"}
          </small>
        </span>
      </div>
      {cache?.stale && (
        <p className="frame-cache-stale">
          <CircleAlert size={12} /> The source changed after this snapshot was taken.
          These rows are the snapshot's until you refresh.
        </p>
      )}
      {computed.upstreamStale && (
        <p className="frame-cache-stale">
          <CircleAlert size={12} /> A frame this one reads from is serving an
          out-of-date snapshot, so these numbers are out of date too. Refresh
          that one first — the toolbar's Refresh does the whole chain in order.
        </p>
      )}
      <button
        className="secondary-action"
        disabled={busy !== null}
        onClick={() => run(true, "cache")}
      >
        <RefreshCw className={busy === "cache" ? "spinning" : ""} size={13} />
        {busy === "cache"
          ? "Computing…"
          : cache
          ? "Refresh snapshot"
          : "Cache this frame"}
      </button>
      {cache && (
        <button
          className="secondary-action"
          disabled={busy !== null}
          onClick={() => run(false, "clear")}
        >
          <Play size={13} /> {busy === "clear" ? "Clearing…" : "Read live instead"}
        </button>
      )}
      {failure && (
        <FormulaErrorDetails title="Could not update the snapshot" error={failure} />
      )}
      <p>
        {cache
          ? "Reads scan the snapshot instead of re-running this transformation. The transformation is kept, so it can be refreshed or set back to live."
          : "Caching computes this frame once and stores the result beside the document. Useful when a grouped result is read often and its source rarely changes."}
      </p>
    </div>
  );
}

