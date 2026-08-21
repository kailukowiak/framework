import { formulaReferenceDecorations } from "./lib/formulaReferenceDecorations";
import type { FormulaReference } from "./lib/formulaReferences";

export function relativeRowReading(rowOffset: number): string | null {
  if (rowOffset === 0) return null;
  if (rowOffset === 1) return "previous row";
  if (rowOffset === -1) return "next row";
  return rowOffset > 1 ? `${rowOffset} rows earlier` : `${-rowOffset} rows later`;
}

/** The readable, clickable counterpart to colored formula syntax. */
export function FormulaReferenceLegend({
  source,
  references,
  onSelect,
}: {
  source: string;
  references: FormulaReference[];
  onSelect: (start: number, end: number) => void;
}) {
  const decorations = formulaReferenceDecorations(source, references);
  if (!decorations.length) return null;
  return (
    <span className="formula-reference-legend" aria-label="Formula references">
      {decorations.map((decoration) => {
        const relative = relativeRowReading(decoration.rowOffset);
        return (
          <button
            type="button"
            key={`${decoration.start}-${decoration.reference.id}`}
            className={`formula-reference-chip formula-ref-color-${decoration.colorIndex}`}
            title={`${decoration.reference.detail}${relative ? ` · ${relative}` : ""}`}
            onPointerDown={(event) => event.preventDefault()}
            onClick={() => onSelect(decoration.start, decoration.end)}
          >
            <i aria-hidden />
            {decoration.reference.label}
            {relative && <small>{relative}</small>}
          </button>
        );
      })}
    </span>
  );
}
