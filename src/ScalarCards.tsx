import { useMemo } from "react";
import { FormulaField } from "./FormulaField";
import { DebugTracePanel } from "./DebugTracePanel";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type {
  ComputedResult,
  ComputedFrame,
  ContainerObject,
  DataObject,
  DataType,
  FormulaFunction,
  ResultObject,
  SeriesObject,
  ValueObject,
} from "./lib/types";

const SERIES_TYPES: DataType[] = [
  "string",
  "number",
  "currency",
  "percentage",
  "boolean",
  "date",
];

/**
 * A named list, edited as text.
 *
 * One value per line, because that is what a list looks like and what
 * pasting a spreadsheet column produces. Anything else that gets pasted in —
 * `[1, 2, 3]`, a NumPy or R repr, a comma-separated line — is read by the
 * core, so the box accepts what people have rather than what it would prefer.
 */
export function SeriesCard({
  series,
  onOperation,
}: {
  series: SeriesObject;
  onOperation: OperationHandler;
}) {
  const text = series.values.join("\n");
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
      <textarea
        className="series-values"
        aria-label={`${series.name} values`}
        defaultValue={text}
        key={text}
        spellCheck={false}
        onBlur={(event) => {
          if (event.target.value !== text)
            onOperation({
              type: "setSeries",
              objectId: series.id,
              values: event.target.value,
            });
        }}
      />
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
        <small>
          {series.values.length} {series.values.length === 1 ? "value" : "values"}
        </small>
      </div>
    </div>
  );
}

export function ValueCard({
  value,
  onOperation,
}: {
  value: ValueObject;
  onOperation: OperationHandler;
}) {
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
          defaultValue={value.raw}
          key={value.raw}
          onBlur={(event) => {
            if (event.target.value !== value.raw)
              onOperation({
                type: "setValue",
                objectId: value.id,
                raw: event.target.value,
              });
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
          }}
        />
      </div>
      <small>{value.dataType} · referenced by formulas</small>
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
        detail: `List · ${object.values.length} ${
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
  computed,
  objects,
  computedFrames,
  formulaFunctions,
  onOperation,
  onFreeze,
}: {
  result: ResultObject;
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
        <output className="result-display">
          {computed?.error ? "—" : computed?.display ?? "—"}
        </output>
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
        <small>{computed?.dataType ?? "…"} · computed from its references</small>
      )}
    </div>
  );
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
