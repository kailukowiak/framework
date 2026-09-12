import { useMemo } from "react";
import { FormulaErrorDetails } from "./FormulaEditor";
import { MatrixAxis, MatrixFormulaField } from "./MatrixAxis";
import { useParameterInputs } from "./ParameterInputs";
import { scalarFormulaReferences } from "./ScalarCards";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
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
import type { VectorDrag } from "./lib/vectorDrag";

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
  const inputs = useParameterInputs();
  const inputIds = new Set([...inputs.map((input) => input.id), ...objects.filter((object) => object.kind === "value").map((object) => object.id)]);
  const references = scalarFormulaReferences(objects, formulaFunctions, computedFrames, matrix.id);
  return (
    <CalculationMatrixCard
      matrix={matrix}
      computed={computed}
      references={references}
      inputReferences={references.filter((reference) => inputIds.has(reference.id))}
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
  inputReferences = [],
  onRename,
  onCommit,
}: {
  matrix: CalculationMatrixCardObject;
  computed?: CalculationMatrixCardComputed;
  references: FormulaReference[];
  inputReferences?: FormulaReference[];
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
  const bindInput = (axis: "rows" | "columns", index: number, targetId: string) => {
    const next = (axis === "rows" ? rows : columns).map((item, i) => i === index ? { ...item, targetId: targetId || undefined } : item);
    return onCommit({ rows: axis === "rows" ? next : rows, columns: axis === "columns" ? next : columns, body: matrix.body.source });
  };

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
          inputReferences={inputReferences}
          onBind={(index, target) => bindInput("rows", index, target)}
          sources={matrix.rows}
          tuples={computed?.rowTuples ?? []}
          references={references}
          onCommit={(index, source) => commitAxis("rows", index, source)}
          onDrop={(vector) => void appendVector("rows", vector)}
        />
        <MatrixAxis
          axis="columns"
          inputReferences={inputReferences}
          onBind={(index, target) => bindInput("columns", index, target)}
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
  return axis.map((item) => ({ id: item.id, name: item.name, formula: item.source, targetId: item.targetId }));
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
