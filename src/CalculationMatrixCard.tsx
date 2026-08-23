import { useEffect, useMemo, useRef, useState } from "react";
import { FormulaEditor, FormulaErrorDetails } from "./FormulaEditor";
import { scalarFormulaReferences } from "./ScalarCards";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import { formatFormulaChains } from "./lib/formulaFormatting";
import type { OperationHandler } from "./lib/handlers";
import type {
  CalculationMatrixAxisFormula,
  CalculationMatrixFormulaInput,
  CalculationMatrixObject,
  ComputedCalculationMatrix,
  ComputedFrame,
  DataObject,
  FormulaFunction,
} from "./lib/types";
import {
  hasVectorDrag,
  readVectorDrag,
  type VectorDrag,
} from "./lib/vectorDrag";

export type CalculationMatrixCardObject = CalculationMatrixObject;
export type CalculationMatrixCardComputed = ComputedCalculationMatrix;
export type CalculationMatrixFormulaDraft = CalculationMatrixFormulaInput;

export type MatrixDraft = {
  rows: CalculationMatrixFormulaDraft[];
  columns: CalculationMatrixFormulaDraft[];
  body: string;
};

/** The small operation adapter between generated document types and the card. */
export function CalculationMatrixCanvasCard({
  matrix,
  computed,
  objects,
  computedFrames,
  formulaFunctions,
  onOperation,
}: {
  matrix: CalculationMatrixObject;
  computed?: ComputedCalculationMatrix;
  objects: DataObject[];
  computedFrames: Record<string, ComputedFrame>;
  formulaFunctions: FormulaFunction[];
  onOperation: OperationHandler;
}) {
  return (
    <CalculationMatrixCard
      matrix={matrix}
      computed={computed}
      references={scalarFormulaReferences(
        objects,
        formulaFunctions,
        computedFrames,
        matrix.id
      )}
      onRename={(name) =>
        onOperation({ type: "renameObject", objectId: matrix.id, name })
      }
      onCommit={(draft) =>
        onOperation(
          { type: "setCalculationMatrix", objectId: matrix.id, ...draft },
          { inlineError: true }
        )
      }
    />
  );
}

export function CalculationMatrixCard({
  matrix,
  computed,
  references,
  onRename,
  onCommit,
}: {
  matrix: CalculationMatrixCardObject;
  computed?: CalculationMatrixCardComputed;
  references: FormulaReference[];
  onRename: (name: string) => unknown;
  onCommit: (draft: MatrixDraft) => Promise<string | null>;
}) {
  const rows = axisInputs(matrix.rows);
  const columns = axisInputs(matrix.columns);
  const bodyReferences = useMemo(
    () => [...references, ...matrixAxisReferences(matrix)],
    [matrix, references]
  );
  const commitAxis = (
    axis: "rows" | "columns",
    index: number,
    source: string,
    suggestedName?: string
  ) => {
    const current = axis === "rows" ? rows : columns;
    const next = current.slice();
    const canonical = canonicalMatrixSource(source, references);
    if (!canonical) next.splice(index, 1);
    else if (index < current.length)
      next[index] = { ...current[index], formula: canonical };
    else
      next.push({
        name: uniqueAxisName(
          suggestedName ?? matrixSourceName(canonical, references, axis),
          [...rows, ...columns].map((item) => item.name)
        ),
        formula: canonical,
      });
    return onCommit({ rows: axis === "rows" ? next : rows, columns: axis === "columns" ? next : columns, body: matrix.body.source });
  };
  const appendVector = (axis: "rows" | "columns", vector: VectorDrag) =>
    commitAxis(axis, axis === "rows" ? rows.length : columns.length, vector.formula, vector.name);

  return (
    <div className="calculation-matrix-card">
      <input
        className="calculation-matrix-name"
        aria-label="Calculation Matrix name"
        defaultValue={matrix.name}
        key={matrix.name}
        onBlur={(event) => {
          const name = event.currentTarget.value.trim();
          if (name && name !== matrix.name) void onRename(name);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <div className="calculation-matrix-axes">
        <MatrixAxis
          axis="rows"
          sources={matrix.rows}
          tuples={computed?.rowTuples ?? []}
          references={references}
          onCommit={(index, source) => commitAxis("rows", index, source)}
          onDrop={(vector) => void appendVector("rows", vector)}
        />
        <MatrixAxis
          axis="columns"
          sources={matrix.columns}
          tuples={computed?.columnTuples ?? []}
          references={references}
          onCommit={(index, source) => commitAxis("columns", index, source)}
          onDrop={(vector) => void appendVector("columns", vector)}
        />
      </div>
      <MatrixFormulaField
        editorId={`calculation-matrix:${matrix.id}:body`}
        label="Formula"
        initial={matrix.body.source}
        references={bodyReferences}
        error={matrix.body.error}
        placeholder="Revenue * Rate"
        format
        onCommit={(body) => onCommit({ rows, columns, body })}
      />
      <MatrixOutput matrix={matrix} computed={computed} />
    </div>
  );
}

function MatrixAxis({
  axis,
  sources,
  tuples,
  references,
  onCommit,
  onDrop,
}: {
  axis: "rows" | "columns";
  sources: CalculationMatrixAxisFormula[];
  tuples: Array<{ values: string[] }>;
  references: FormulaReference[];
  onCommit: (index: number, source: string) => Promise<string | null>;
  onDrop: (vector: VectorDrag) => void;
}) {
  const title = axis === "rows" ? "Rows" : "Columns";
  return (
    <section
      className="calculation-matrix-axis"
      aria-label={`${title} formulas`}
      data-matrix-axis={axis}
      data-vector-drop-target="true"
      data-vector-drop-action={`Use in ${axis}`}
      onDragOver={(event) => {
        if (!hasVectorDrag(event.dataTransfer)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "link";
      }}
      onDrop={(event) => {
        const vector = readVectorDrag(event.dataTransfer);
        if (!vector) return;
        event.preventDefault();
        onDrop(vector);
      }}
    >
      <header>
        <strong>{title}</strong>
        <AxisPreview sources={sources} tuples={tuples} />
      </header>
      {sources.map((source, index) => (
        <MatrixFormulaField
          key={source.id}
          editorId={`calculation-matrix:${source.id}`}
          label={`${title} · ${source.name}`}
          initial={source.source}
          references={references}
          error={source.error}
          placeholder={axis === "rows" ? "Scenario" : "Period"}
          onCommit={(draft) => onCommit(index, draft)}
        />
      ))}
      <MatrixFormulaField
        editorId={`calculation-matrix:new-${axis}`}
        label={`Add ${title.toLowerCase()} source`}
        initial=""
        references={references}
        placeholder={axis === "rows" ? "MyVar" : "MyTable.Column1"}
        onCommit={(draft) => onCommit(sources.length, draft)}
      />
    </section>
  );
}

function AxisPreview({
  sources,
  tuples,
}: {
  sources: CalculationMatrixAxisFormula[];
  tuples: Array<{ values: string[] }>;
}) {
  if (!sources.length || !tuples.length)
    return <span className="calculation-matrix-axis-empty">drop or type a vector</span>;
  return (
    <span className="calculation-matrix-axis-preview" title={`${tuples.length} combinations`}>
      {sources.map((source, sourceIndex) => (
        <span key={source.id}>
          {source.name}: {tuples.slice(0, 3).map((tuple) => tuple.values[sourceIndex] ?? "—").join(", ")}
          {tuples.length > 3 ? "…" : ""}
        </span>
      ))}
    </span>
  );
}

function MatrixFormulaField({
  editorId,
  label,
  initial,
  references,
  error,
  placeholder,
  format = false,
  onCommit,
}: {
  editorId: string;
  label: string;
  initial: string;
  references: FormulaReference[];
  error?: string | null;
  placeholder: string;
  format?: boolean;
  onCommit: (source: string) => Promise<string | null>;
}) {
  const [draft, setDraft] = useState(initial);
  const [commitError, setCommitError] = useState<string | null>(null);
  const committed = useRef(initial);
  useEffect(() => {
    setDraft(initial);
    committed.current = initial;
    setCommitError(null);
  }, [initial]);
  const execute = async (source = draft) => {
    const next = format ? formatFormulaChains(source).source : source.trim();
    setDraft(next);
    if (next === committed.current) return;
    const nextError = await onCommit(next);
    setCommitError(nextError);
    if (!nextError) committed.current = next;
  };
  return (
    <div
      className="calculation-matrix-formula"
      onBlurCapture={(event) => {
        if (!(event.target instanceof HTMLTextAreaElement)) return;
        void execute(event.target.value);
      }}
    >
      <FormulaEditor
        editorId={editorId}
        label={label}
        value={draft}
        references={references}
        error={commitError ?? error}
        placeholder={placeholder}
        compact
        onChange={(next) => {
          setDraft(next);
          setCommitError(null);
        }}
        onCommit={execute}
      />
    </div>
  );
}

function MatrixOutput({
  matrix,
  computed,
}: {
  matrix: CalculationMatrixCardObject;
  computed?: CalculationMatrixCardComputed;
}) {
  const validBody = Boolean(matrix.body.source.trim() && matrix.body.formula && !matrix.body.error);
  if (!validBody)
    return <div className="calculation-matrix-output empty" aria-label="Calculation Matrix output" />;
  if (computed?.error)
    return <FormulaErrorDetails error={computed.error} formulas={[matrix.body.source]} title="Calculation Matrix could not run" />;
  if (!computed?.cells.length || !computed.columnTuples.length || !computed.rowTuples.length)
    return <div className="calculation-matrix-output empty" aria-label="Calculation Matrix output" />;
  return (
    <div className="calculation-matrix-output" aria-label="Calculation Matrix output">
      <table>
        <thead>
          <tr>
            {matrix.rows.map((source) => <th key={source.id}>{source.name}</th>)}
            {computed.columnTuples.map((tuple, index) => (
              <th key={index}>{tuple.values.join(" · ")}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {computed.rowTuples.map((tuple, rowIndex) => (
            <tr key={rowIndex}>
              {tuple.values.map((value, index) => <th key={matrix.rows[index]?.id ?? index}>{value}</th>)}
              {(computed.cells[rowIndex] ?? []).map((cell, columnIndex) => (
                <td className={cell.error ? "error" : ""} title={cell.error ?? cell.display} key={columnIndex}>
                  {cell.error ? "—" : cell.display}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function axisInputs(axis: CalculationMatrixAxisFormula[]): CalculationMatrixFormulaDraft[] {
  return axis.map((item) => ({ id: item.id, name: item.name, formula: item.source }));
}

export function matrixAxisReferences(matrix: CalculationMatrixCardObject): FormulaReference[] {
  return [...matrix.rows, ...matrix.columns].map((item) => ({
    id: item.id,
    objectId: matrix.id,
    label: item.name,
    token: formulaToken(item.name),
    kind: "value" as const,
    detail: "Calculation Matrix axis",
  }));
}

export function canonicalMatrixSource(
  source: string,
  references: FormulaReference[]
): string {
  const trimmed = source.trim();
  const unquoted = trimmed.replaceAll("`", "").toLocaleLowerCase();
  const exact = references.find(
    (reference) =>
      reference.kind !== "function" &&
      (reference.label.toLocaleLowerCase() === unquoted ||
        reference.token.replaceAll("`", "").toLocaleLowerCase() === unquoted)
  );
  return exact?.token.replace(/\.$/, "") ?? trimmed;
}

function matrixSourceName(
  source: string,
  references: FormulaReference[],
  axis: "rows" | "columns"
): string {
  const canonical = canonicalMatrixSource(source, references);
  const exact = references.find((reference) => reference.token.replace(/\.$/, "") === canonical);
  if (exact) return exact.label.split(".").at(-1) ?? exact.label;
  const quoted = [...canonical.matchAll(/`((?:``|[^`])+)`/g)].at(-1)?.[1];
  if (quoted) return quoted.replaceAll("``", "`");
  const bare = canonical.match(/[\p{L}_][\p{L}\p{N}_]*\s*$/u)?.[0].trim();
  return bare || (axis === "rows" ? "Row" : "Column");
}

function uniqueAxisName(name: string, existing: string[]): string {
  if (!existing.includes(name)) return name;
  let suffix = 2;
  while (existing.includes(`${name} ${suffix}`)) suffix += 1;
  return `${name} ${suffix}`;
}
