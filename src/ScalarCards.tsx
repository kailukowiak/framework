import { useMemo, useState, type PointerEvent as ReactPointerEvent } from "react";
import { FormulaField } from "./FormulaField";
import { ParameterAnswer } from "./ParameterInputs";
import { DebugTracePanel } from "./DebugTracePanel";
import { useActiveFormulaEditorCommands } from "./ActiveFormulaEditor";
import {
  formulaToken,
  isFormulaExecuteShortcut,
  type FormulaReference,
} from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type {
  ComputedResult,
  ComputedFrame,
  ComputedValue,
  ContainerObject,
  DataObject,
  DataType,
  FormulaFunction,
  ResultObject,
  SeriesObject,
  ValueObject,
} from "./lib/types";
import { beginVectorPointerDrag } from "./lib/vectorDrag";
import { formatFormulaChains } from "./lib/formulaFormatting";

const SERIES_TYPES: DataType[] = [
  "string",
  "number",
  "currency",
  "percentage",
  "boolean",
  "date",
];

/**
 * One named formula with none of a window's chrome around it.
 *
 * It is deliberately the same FormulaEditor used by results and Wrangle, so
 * the local field and the top formula bar are two views of one draft. Its
 * answer may be one value or a vector, exactly like one Scratchwork line.
 */
export function VariableCard({
  result,
  formula,
  computed,
  objects,
  computedFrames,
  formulaFunctions,
  onOperation,
  onMovePointerDown,
}: {
  result: ResultObject;
  formula: string;
  computed: ComputedResult | undefined;
  objects: DataObject[];
  computedFrames: Record<string, ComputedFrame>;
  formulaFunctions: FormulaFunction[];
  onOperation: OperationHandler;
  onMovePointerDown: (event: ReactPointerEvent) => void;
}) {
  const displayName = variableNameFromInput(result.name);
  const editorId = `variable:${result.id}`;
  const activeEditor = useActiveFormulaEditorCommands();
  const formatActiveDraft = () => {
    const active = activeEditor.getActive();
    if (active?.id !== editorId) return;
    const formatted = formatFormulaChains(
      active.draft,
      active.selection.start,
      active.selection.end
    );
    if (formatted.source !== active.draft)
      activeEditor.setDraft(formatted.source, formatted.selection);
  };
  const references = useMemo(
    () =>
      scalarFormulaReferences(objects, formulaFunctions, computedFrames, result.id),
    [objects, formulaFunctions, computedFrames, result.id]
  );
  const valueCount = computed?.valueCount ?? 0;
  return (
    <div
      className="variable-card"
      onKeyDownCapture={(event) => {
        if (event.target instanceof HTMLTextAreaElement && isFormulaExecuteShortcut(event))
          formatActiveDraft();
      }}
      onBlurCapture={(event) => {
        const target = event.target;
        if (!(target instanceof HTMLTextAreaElement)) return;
        // A variable is one self-saving formula surface: its editor persists
        // on blur through FormulaField's `commitOnBlur` below. Formatting
        // here happens first, while the shared draft is still this card's,
        // so the surface keeps the multiline shape the save will produce. It
        // used to persist by synthesizing a ⌘↵ keydown; that spelling became
        // wrong once an explicit commit started ending the shared session —
        // blur is routinely the formula bar taking over this same draft.
        formatActiveDraft();
      }}
    >
      <span
        className="variable-move-handle"
        aria-hidden="true"
        title={`Move ${result.name}`}
        onPointerDown={onMovePointerDown}
      >
        •••
      </span>
      <input
        className="variable-name-input"
        aria-label="Variable name"
        defaultValue={displayName}
        key={result.name}
        size={1}
        style={{ width: `${Math.max(1, displayName.length + 1)}ch` }}
        spellCheck={false}
        onBlur={(event) => {
          const name = variableNameFromInput(event.target.value);
          event.target.value = name;
          if (name !== result.name)
            void onOperation({
              type: "renameObject",
              objectId: result.id,
              name,
            });
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <FormulaField
        editorId={editorId}
        label={result.name}
        initial={formatFormulaChains(computed?.formula ?? "").source}
        references={references}
        compact
        commitOnBlur
        onCommit={(draft) =>
          onOperation(
            {
              type: "setResultFormula",
              objectId: result.id,
              formula: formatFormulaChains(draft).source,
            },
            { inlineError: true }
          )
        }
      />
      <ParameterAnswer id={result.id}><output
        className={computed?.error ? "variable-answer error" : "variable-answer"}
        title={computed?.error ?? computed?.display}
      >
        → {computed?.error ? "—" : computed?.display ?? "—"}
      </output></ParameterAnswer>
      <small
        className="variable-vector-handle"
        data-vector-drag={valueCount > 0 ? "true" : undefined}
        title={valueCount > 0 ? "Drag this value or vector" : computed?.error ?? undefined}
        onPointerDown={
          valueCount > 0
            ? (event) =>
                beginVectorPointerDrag(event.nativeEvent, {
                  objectId: result.id,
                  formula,
                  name: result.name,
                  length: valueCount,
                })
            : undefined
        }
      >
        {valueCount > 1 ? valueCount : ""}
      </small>
    </div>
  );
}

/** Names are stored plainly; backticks only quote those names in formulas. */
function variableNameFromInput(name: string): string {
  const trimmed = name.trim();
  if (trimmed.length < 2 || !trimmed.startsWith("`") || !trimmed.endsWith("`"))
    return trimmed;
  return trimmed.slice(1, -1).replaceAll("``", "`");
}

/**
 * A named vector, read compactly and edited as one text surface.
 *
 * The resting view wraps values across the available width instead of making
 * a three-item vector look like a cramped three-row form. Clicking it opens
 * one ordinary paste surface; values never turn into a row of separate
 * controls.
 */
export function SeriesCard({
  series,
  formula,
  onOperation,
}: {
  series: SeriesObject;
  /** Its fully-qualified address, including any container names. */
  formula: string;
  onOperation: OperationHandler;
}) {
  const text = series.values.join("\n");
  const [editingValues, setEditingValues] = useState(false);
  return (
    <div className="value-card series-card">
      <input
        className="object-name-input"
        defaultValue={series.name}
        key={series.name}
        onBlur={(event) => {
          if (event.target.value !== series.name)
            onOperation({
              type: "renameObject",
              objectId: series.id,
              name: event.target.value,
            });
        }}
      />
      {editingValues ? (
        <textarea
          autoFocus
          className="series-values series-values-editing"
          aria-label={`${series.name} values`}
          defaultValue={text}
          key={text}
          spellCheck={false}
          onBlur={(event) => {
            setEditingValues(false);
            if (event.target.value !== text)
              void onOperation({
                type: "setSeries",
                objectId: series.id,
                values: event.target.value,
              });
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.currentTarget.value = text;
              setEditingValues(false);
            } else if (event.key === "Enter" && event.metaKey) {
              event.currentTarget.blur();
            }
          }}
        />
      ) : (
        <button
          type="button"
          className="series-values-preview"
          aria-label={`Edit ${series.name} values`}
          onClick={() => setEditingValues(true)}
        >
          {series.values.map((value, index) => (
            <span className="series-value" key={`${index}:${value}`}>
              {value}
            </span>
          ))}
        </button>
      )}
      <div className="series-footer">
        <select
          aria-label={`${series.name} type`}
          value={series.dataType}
          onChange={(event) =>
            onOperation({
              type: "setSeriesType",
              objectId: series.id,
              dataType: event.target.value as DataType,
            })
          }
        >
          {SERIES_TYPES.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
        <small
          data-vector-drag={series.values.length > 0 ? "true" : undefined}
          title="Drag this vector onto a table or empty canvas"
          onPointerDown={(event) =>
            beginVectorPointerDrag(event.nativeEvent, {
              objectId: series.id,
              formula,
              name: series.name,
              length: series.values.length,
            })
          }
        >
          {series.values.length} {series.values.length === 1 ? "value" : "values"}
        </small>
      </div>
    </div>
  );
}

/**
 * A number somebody typed, and — when a scenario is overriding it — the
 * number actually being read, with the scenario's name after it.
 *
 * The override goes in the field itself and its provenance immediately after
 * it, as muted text on the same line (`1.15 · Upside`), rather than into a
 * badge. A chip would be a second thing on a card that holds one number, and
 * the point of the cue is that the number and the reason it is that number
 * are read in one glance — under the field, a line down and among the type
 * and the value count, the scenario's name was there and went unread.
 */
export function ValueCard({
  value,
  computed,
  formula,
  onOperation,
}: {
  value: ValueObject;
  /** Absent while a document has no scenarios, which is most of them. */
  computed?: ComputedValue;
  formula: string;
  onOperation: OperationHandler;
}) {
  // The number actually being read. Under an override that is the
  // scenario's, and editing it edits *that scenario* — the card is a window
  // onto whatever is in force, not a fixed window onto the base.
  const scenarioId = computed?.scenarioId ?? null;
  const scenarioName = computed?.scenarioName ?? null;
  const raw = scenarioId ? computed!.raw : value.raw;
  return (
    <div className="value-card">
      <input
        className="object-name-input"
        defaultValue={value.name}
        key={value.name}
        onBlur={(event) => {
          if (event.target.value !== value.name)
            onOperation({
              type: "renameObject",
              objectId: value.id,
              name: event.target.value,
            });
        }}
      />
      <div className="value-input-row">
        <input
          className="value-input"
          type={value.dataType === "date" ? "date" : "text"}
          defaultValue={raw}
          key={raw}
          onBlur={(event) => {
            if (event.target.value === raw) return;
            onOperation(
              scenarioId
                ? {
                    type: "setScenarioValue",
                    scenarioId,
                    valueId: value.id,
                    raw: event.target.value === "" ? null : event.target.value,
                  }
                : { type: "setValue", objectId: value.id, raw: event.target.value }
            );
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
          }}
        />
        {scenarioName && (
          <span
            className="value-scenario"
            title={`${scenarioName} overrides this value`}
          >
            · {scenarioName}
          </span>
        )}
      </div>
      <small
        data-vector-drag="true"
        title="Drag this value onto a table or empty canvas"
        onPointerDown={(event) =>
          beginVectorPointerDrag(event.nativeEvent, {
            objectId: value.id,
            formula,
            name: value.name,
            length: 1,
          })
        }
      >
        {value.dataType} · 1 value
      </small>
    </div>
  );
}

/**
 * The name a canvas object answers to in a formula — the containers it sits
 * in, outermost first, then its own name — matching what the core renders
 * back.
 */
function qualifiedObjectPath(objects: DataObject[], objectId: string): string[] {
  const byId = new Map(objects.map((object) => [object.id, object]));
  const holder = new Map<string, ContainerObject>();
  for (const object of objects)
    if (object.kind === "container")
      for (const memberId of object.memberIds) holder.set(memberId, object);
  const path: string[] = [];
  for (
    let current = byId.get(objectId);
    current;
    current = holder.get(current.id)
  )
    path.unshift(current.name);
  return path;
}

/** A stable formula address for a canvas object, including its containers. */
export function objectFormulaToken(objects: DataObject[], objectId: string): string {
  return qualifiedObjectPath(objects, objectId).map(formulaToken).join(".");
}

/**
 * What a formula outside any frame may reference: canvas values, results,
 * lists, columns of materialized frames, and the functions. The scalar
 * cousin of the per-frame list the inspector builds.
 */
export function scalarFormulaReferences(
  objects: DataObject[],
  formulaFunctions: FormulaFunction[],
  computedFrames: Record<string, ComputedFrame>,
  excludeId?: string
): FormulaReference[] {
  const references: FormulaReference[] = [];
  for (const object of objects) {
    if (object.id === excludeId) continue;
    if (object.kind === "value" || object.kind === "result") {
      const path = qualifiedObjectPath(objects, object.id);
      references.push({
        id: object.id,
        objectId: object.id,
        label: path.join("."),
        token: path.map(formulaToken).join("."),
        kind: "value",
        detail:
          object.kind === "value"
            ? `Canvas value · ${object.raw}`
            : object.variable
              ? "Canvas variable"
              : "Computed result",
      });
    } else if (object.kind === "series") {
      // Lists were missing here entirely, which is why typing `` `List ``
      // offered everything except the list.
      const path = qualifiedObjectPath(objects, object.id);
      references.push({
        id: object.id,
        objectId: object.id,
        label: path.join("."),
        token: path.map(formulaToken).join("."),
        kind: "value",
        detail: `Vector · ${object.values.length} ${
          object.values.length === 1 ? "value" : "values"
        } · ${object.dataType}`,
      });
    } else if (object.kind === "block") {
      // Blank and comment lines answer to no name, so there is nothing to
      // offer and nothing that would resolve.
      for (const line of object.lines.filter((line) => line.name))
        references.push({
          id: line.id,
          objectId: object.id,
          label: `${object.name}.${line.name}`,
          token: `${formulaToken(object.name)}.${formulaToken(line.name)}`,
          kind: "value",
          detail: `Line of ${object.name}`,
        });
      // Every frame, not only the ones holding a snapshot. Scratchwork reads
      // live and derived frames directly; offering only materialized frames
      // here would hide most of the document from its ad-hoc calculation
      // surface.
    } else if (object.kind === "frame") {
      references.push({
        id: object.id,
        objectId: object.id,
        label: object.name,
        token: `${formulaToken(object.name)}.`,
        kind: "frame",
        detail: `${object.columns.length} columns`,
      });
      for (const column of object.columns)
        references.push({
          id: column.id,
          objectId: object.id,
          frameId: object.id,
          label: `${object.name}.${column.name}`,
          token: `${formulaToken(object.name)}.${formulaToken(column.name)}`,
          kind: "column",
          detail: `${column.dataType} column in ${object.name}`,
        });
    }
  }
  references.push(
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
    }))
  );
  return references.filter((reference) => reference.token.length > 0);
}

export function ResultCard({
  result,
  formula,
  computed,
  objects,
  computedFrames,
  formulaFunctions,
  onOperation,
  onFreeze,
}: {
  result: ResultObject;
  formula: string;
  computed: ComputedResult | undefined;
  objects: DataObject[];
  computedFrames: Record<string, ComputedFrame>;
  formulaFunctions: FormulaFunction[];
  onOperation: OperationHandler;
  onFreeze: (objectId: string) => Promise<void>;
}) {
  const references = useMemo(
    () =>
      scalarFormulaReferences(objects, formulaFunctions, computedFrames, result.id),
    [objects, formulaFunctions, computedFrames, result.id]
  );
  const drag = resultVectorDrag(result, formula, computed);
  return (
    <div className="value-card result-card">
      <input
        className="object-name-input"
        defaultValue={result.name}
        key={result.name}
        onBlur={(event) => {
          if (event.target.value !== result.name)
            onOperation({
              type: "renameObject",
              objectId: result.id,
              name: event.target.value,
            });
        }}
      />
      <div className="value-input-row">
        <ParameterAnswer id={result.id}><output className="result-display">
          {computed?.error ? "—" : computed?.display ?? "—"}
        </output></ParameterAnswer>
        {/* Nothing at all when it is live: a card whose whole job is to be
            live does not need a badge saying so. A written-down answer is
            the case worth marking, because its age is a fact about it. */}
        {computed?.frozen && (
          <button
            className="frozen-chip"
            title="Refresh this answer from live data"
            onClick={() => void onFreeze(result.id)}
          >
            {computed.frozen.stale ? "stale" : "frozen"} ·{" "}
            {takenWhen(computed.frozen.takenAt)}
          </button>
        )}
      </div>
      <FormulaField
        editorId={`result:${result.id}`}
        label="Formula"
        initial={computed?.formula ?? ""}
        references={references}
        onCommit={(draft) =>
          onOperation(
            { type: "setResultFormula", objectId: result.id, formula: draft },
            { inlineError: true }
          )
        }
      />
      {computed?.error ? (
        <>
          <small className="result-error">{computed.error}</small>
          <DebugTracePanel objectId={result.id} />
        </>
      ) : (
        <small
          data-vector-drag={drag ? "true" : undefined}
          title={drag ? "Drag this result onto a table or empty canvas" : undefined}
          onPointerDown={
            drag
              ? (event) => beginVectorPointerDrag(event.nativeEvent, drag)
              : undefined
          }
        >
          {resultSummary(computed)}
        </small>
      )}
    </div>
  );
}

function resultVectorDrag(
  result: ResultObject,
  formula: string,
  computed: ComputedResult | undefined
) {
  if (!computed || computed.error || computed.valueCount < 1) return null;
  return {
    objectId: result.id,
    formula,
    name: result.name,
    length: computed.valueCount,
  };
}

function resultSummary(computed: ComputedResult | undefined): string {
  if (!computed) return "… · computed from its references";
  const noun = computed.valueCount === 1 ? "value" : "values";
  return `${computed.dataType} · ${computed.valueCount} ${noun} · computed from its references`;
}

/**
 * The scratchpad: one text surface, and every line's answer beside it.
 *
 * This is a text editor rather than a stack of formula fields, and the
 * difference is the whole point of the object. A block exists to solve a
 * density problem — forty scratch calculations should not be forty cards —
 * and a card that spent a labelled field, a delete button and an Execute
 * button on every line would be forty cards again, stacked. So: type down
 * the page, one calculation per line, answers in the gutter.
 *
 * `x = 10` names a line as it defines it, siblings above resolve bare, and
 * a line that does not parse yet keeps its text and says why in its own
 * gutter — see `BlockLine` in the core for why that leniency is confined to
 * this one surface.
 */

/** A frozen answer's age, in the words someone would use out loud. */
export function takenWhen(takenAt: string): string {
  const taken = new Date(takenAt);
  if (Number.isNaN(taken.getTime())) return "earlier";
  const minutes = Math.round((Date.now() - taken.getTime()) / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return taken.toLocaleDateString();
}
