import { useEffect, useRef, useState } from "react";
import { FormulaEditor } from "./FormulaEditor";
import { formatFormulaChains } from "./lib/formulaFormatting";
import type { FormulaReference } from "./lib/formulaReferences";
import type { CalculationMatrixAxisFormula } from "./lib/types";
import { hasVectorDrag, readVectorDrag, type VectorDrag } from "./lib/vectorDrag";
import "./matrix-inputs.css";

export function MatrixAxis({
  axis,
  sources,
  tuples,
  references,
  inputReferences,
  onBind,
  onCommit,
  onDrop,
}: {
  axis: "rows" | "columns";
  sources: CalculationMatrixAxisFormula[];
  tuples: Array<{ values: string[] }>;
  references: FormulaReference[];
  inputReferences: FormulaReference[];
  onBind: (index: number, targetId: string) => Promise<string | null>;
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
        <div className="matrix-axis-source" key={source.id}>
        <MatrixInputBinding source={source} title={title} references={inputReferences} onBind={(target) => onBind(index, target)} />
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
        </div>
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

function MatrixInputBinding({ source, title, references, onBind }: {
  source: CalculationMatrixAxisFormula;
  title: string;
  references: FormulaReference[];
  onBind: (target: string) => Promise<string | null>;
}) {
  const [error, setError] = useState<string | null>(null);
  if (!references.length && !source.targetId) return null;
  return <>
    <select className="matrix-input-binding" aria-label={`${title} · ${source.name} input`} value={source.targetId ?? ""}
      onChange={(event) => { void onBind(event.target.value).then(setError); }}>
      <option value="">Local axis only</option>
      {source.targetId && !references.some((reference) => reference.id === source.targetId) && <option value={source.targetId}>Unavailable input</option>}
      {references.map((reference) => <option key={reference.id} value={reference.id}>Vary {reference.label}</option>)}
    </select>
    {error && <span className="formula-error-line">{error}</span>}
  </>;
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

export function MatrixFormulaField({
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
      className={`calculation-matrix-formula${format ? " calculation-matrix-body-formula" : ""}`}
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
