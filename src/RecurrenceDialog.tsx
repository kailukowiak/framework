import { X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { HighlightedFormulaTextarea } from "./FormulaReferenceText";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import type { Column } from "./lib/types";

export type RecurrenceFormulaParts = {
  seed: string;
  next: string;
  partitionName?: string;
};

function topLevelPieces(source: string): string[] {
  const pieces: string[] = [];
  let start = 0;
  let depth = 0;
  let quote: string | null = null;
  let backticked = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === "\\") index += 1;
      else if (character === quote) quote = null;
    } else if (backticked) {
      if (character === "`" && source[index + 1] === "`") index += 1;
      else if (character === "`") backticked = false;
    } else if (character === "`") backticked = true;
    else if (character === '"' || character === "'") quote = character;
    else if ("([{".includes(character)) depth += 1;
    else if (")]}".includes(character)) depth -= 1;
    else if (character === "," && depth === 0) {
      pieces.push(source.slice(start, index).trim());
      start = index + 1;
    }
  }
  pieces.push(source.slice(start).trim());
  return pieces;
}

export function recurrenceFormula(
  seed: string,
  next: string,
  partitionName?: string
): string {
  const restart = partitionName ? `, restart_by=[${formulaToken(partitionName)}]` : "";
  return `recur(${seed.trim()}, ${next.trim()}${restart})`;
}

export function parseRecurrenceFormula(source: string): RecurrenceFormulaParts | null {
  const trimmed = source.trim();
  if (!trimmed.startsWith("recur(") || !trimmed.endsWith(")")) return null;
  const pieces = topLevelPieces(trimmed.slice(6, -1));
  if (pieces.length < 2 || pieces.length > 3 || !pieces[0] || !pieces[1]) return null;
  if (pieces.length === 2) return { seed: pieces[0], next: pieces[1] };
  const restart = /^restart_by\s*=\s*\[\s*`((?:``|[^`])+)`\s*\]$/i.exec(pieces[2]);
  return restart
    ? {
        seed: pieces[0],
        next: pieces[1],
        partitionName: restart[1].replaceAll("``", "`"),
      }
    : null;
}

export function recurrenceReferences(
  targetName: string,
  columns: Column[]
): FormulaReference[] {
  return [
    {
      id: `previous:${targetName}`,
      label: `Previous ${targetName}`,
      token: "previous()",
      kind: "value",
      detail: "result from the preceding row",
    },
    ...columns.map((column) => ({
      id: column.id,
      label: column.name,
      token: formulaToken(column.name),
      kind: "column" as const,
      detail: `${column.dataType} column`,
    })),
  ];
}

function initialSeed(column: Column): string {
  if (column.dataType === "boolean") return "False";
  if (column.dataType === "date") return "today()";
  if (column.dataType === "string" || column.dataType === "categorical") return '""';
  return "0";
}

function RecurrenceSelectors({
  columns,
  alreadyOrdered,
  orderBy,
  partitionBy,
  onOrderBy,
  onPartitionBy,
}: {
  columns: Column[];
  alreadyOrdered: boolean;
  orderBy: string;
  partitionBy: string;
  onOrderBy: (columnId: string) => void;
  onPartitionBy: (columnId: string) => void;
}) {
  return (
    <div className="running-calculation-fields">
      {!alreadyOrdered && (
        <label>
          Order rows by
          <select value={orderBy} onChange={(event) => onOrderBy(event.target.value)}>
            {columns.map((column) => (
              <option key={column.id} value={column.id}>
                {column.name} — ascending
              </option>
            ))}
          </select>
        </label>
      )}
      <label>
        Restart for each
        <select
          value={partitionBy}
          onChange={(event) => onPartitionBy(event.target.value)}
        >
          <option value="">Never</option>
          {columns.map((column) => (
            <option key={column.id} value={column.id}>
              {column.name}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

/** Guided creation for an explicitly ordered, sequential calculated column. */
export function RecurrenceDialog({
  target,
  columns,
  initialSourceColumnId,
  alreadyOrdered,
  onApply,
  onCancel,
}: {
  target: Column;
  columns: Column[];
  initialSourceColumnId?: string;
  alreadyOrdered: boolean;
  onApply: (formula: string, orderByColumnId?: string) => void;
  onCancel: () => void;
}) {
  const source = columns.find((column) => column.id === initialSourceColumnId);
  const [seed, setSeed] = useState(
    source ? formulaToken(source.name) : initialSeed(target)
  );
  const [next, setNext] = useState(
    source && ["integer", "number", "currency", "percentage"].includes(source.dataType)
      ? `previous() + ${formulaToken(source.name)}`
      : "previous()"
  );
  const [orderBy, setOrderBy] = useState(columns[0]?.id ?? "");
  const [partitionBy, setPartitionBy] = useState("");
  const [activeField, setActiveField] = useState<"seed" | "next">("next");
  const seedRef = useRef<HTMLTextAreaElement>(null);
  const nextRef = useRef<HTMLTextAreaElement>(null);
  const references = useMemo(
    () => recurrenceReferences(target.name, columns),
    [columns, target.name]
  );
  useEffect(() => {
    const close = (event: KeyboardEvent) => event.key === "Escape" && onCancel();
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onCancel]);
  const insert = (token: string, field = activeField) => {
    const node = field === "seed" ? seedRef.current : nextRef.current;
    const value = field === "seed" ? seed : next;
    const start = node?.selectionStart ?? value.length;
    const end = node?.selectionEnd ?? start;
    const updated = `${value.slice(0, start)}${token}${value.slice(end)}`;
    (field === "seed" ? setSeed : setNext)(updated);
    requestAnimationFrame(() => {
      node?.focus();
      node?.setSelectionRange(start + token.length, start + token.length);
    });
  };
  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => event.target === event.currentTarget && onCancel()}
    >
      <form
        className="insert-dialog recurrence-dialog"
        onSubmit={(event) => {
          event.preventDefault();
          if (!seed.trim() || !next.trim() || (!alreadyOrdered && !orderBy)) return;
          const partition = columns.find((column) => column.id === partitionBy);
          onApply(
            recurrenceFormula(seed, next, partition?.name),
            alreadyOrdered ? undefined : orderBy
          );
        }}
      >
        <div className="dialog-header">
          <div>
            <span className="eyebrow">CALCULATE DOWN ROWS</span>
            <h2>{target.name}</h2>
          </div>
          <button type="button" className="icon-button" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>
        <div className="recurrence-formulas">
          <label>
            First row
            <HighlightedFormulaTextarea
              ref={seedRef}
              rows={1}
              value={seed}
              references={references.slice(1)}
              onFocus={() => setActiveField("seed")}
              onChange={(event) => setSeed(event.target.value)}
            />
          </label>
          <label>
            Each next row
            <HighlightedFormulaTextarea
              ref={nextRef}
              autoFocus
              rows={1}
              value={next}
              references={references}
              onFocus={() => setActiveField("next")}
              onChange={(event) => setNext(event.target.value)}
            />
          </label>
        </div>
        <div className="recurrence-reference-strip" aria-label="Insert a reference">
          {references.map((reference, index) => (
            <button
              key={reference.id}
              type="button"
              className={`formula-reference-chip formula-ref-color-${index % 6}`}
              onClick={() =>
                insert(
                  reference.token,
                  reference.id.startsWith("previous:") ? "next" : activeField
                )
              }
            >
              <i aria-hidden />
              {reference.label}
            </button>
          ))}
        </div>
        <RecurrenceSelectors
          columns={columns}
          alreadyOrdered={alreadyOrdered}
          orderBy={orderBy}
          partitionBy={partitionBy}
          onOrderBy={setOrderBy}
          onPartitionBy={setPartitionBy}
        />
        <p className="new-document-note">
          The first field supplies the starting value. In later rows, Previous{" "}
          {target.name} is the result directly above—not the column’s original input.
        </p>
        <div className="dialog-actions">
          <button type="button" className="secondary-action" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="primary-action"
            disabled={!seed.trim() || !next.trim() || (!alreadyOrdered && !orderBy)}
          >
            Create calculation
          </button>
        </div>
      </form>
    </div>
  );
}
