import { X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { formulaToken } from "./lib/formulaReferences";
import type { Column } from "./lib/types";

export type RunningOperation = "sum" | "count" | "min" | "max";

function useCloseOnEscape(onCancel: () => void) {
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onCancel]);
}

export function runningColumnFormula(
  sourceName: string,
  operation: RunningOperation,
  openingValue = 0,
  partitionName?: string
): string {
  const method = {
    sum: "cum_sum",
    count: "cum_count",
    min: "cum_min",
    max: "cum_max",
  }[operation];
  let formula = `${formulaToken(sourceName)}.${method}(False)`;
  if (partitionName) formula += `.over([${formulaToken(partitionName)}])`;
  if (operation === "sum" && openingValue !== 0) formula = `${formula} + ${openingValue}`;
  return formula;
}

/** Click-first authoring for the useful cumulative forms of recurrence. */
export function RunningCalculationDialog({
  targetName,
  columns,
  initialSourceColumnId,
  alreadyOrdered,
  onApply,
  onCancel,
}: {
  targetName: string;
  columns: Column[];
  initialSourceColumnId?: string;
  alreadyOrdered: boolean;
  onApply: (formula: string, orderByColumnId?: string) => void;
  onCancel: () => void;
}) {
  const numericColumns = useMemo(
    () =>
      columns.filter((column) =>
        ["integer", "number", "currency", "percentage"].includes(column.dataType)
      ),
    [columns]
  );
  const [sourceId, setSourceId] = useState(
    numericColumns.some((column) => column.id === initialSourceColumnId)
      ? initialSourceColumnId!
      : numericColumns[0]?.id ?? ""
  );
  const [operation, setOperation] = useState<RunningOperation>("sum");
  const [opening, setOpening] = useState("0");
  const [orderBy, setOrderBy] = useState(columns[0]?.id ?? "");
  const [partitionBy, setPartitionBy] = useState("");
  const source = columns.find((column) => column.id === sourceId);
  const partition = columns.find((column) => column.id === partitionBy);
  const openingNumber = Number(opening);
  const valid = Boolean(
    source &&
      (alreadyOrdered || orderBy) &&
      (operation !== "sum" || Number.isFinite(openingNumber))
  );
  useCloseOnEscape(onCancel);
  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <form
        className="insert-dialog running-calculation-dialog"
        onSubmit={(event) => {
          event.preventDefault();
          if (!valid || !source) return;
          onApply(
            runningColumnFormula(
              source.name,
              operation,
              operation === "sum" ? openingNumber : 0,
              partition?.name
            ),
            alreadyOrdered ? undefined : orderBy
          );
        }}
      >
        <div className="dialog-header">
          <div>
            <span className="eyebrow">RUNNING CALCULATION</span>
            <h2>{targetName}</h2>
          </div>
          <button type="button" className="icon-button" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>
        <div className="running-calculation-fields">
          <label>
            Values from
            <select autoFocus value={sourceId} onChange={(event) => setSourceId(event.target.value)}>
              {numericColumns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Calculation
            <select
              value={operation}
              onChange={(event) => setOperation(event.target.value as RunningOperation)}
            >
              <option value="sum">Running total</option>
              <option value="count">Running count</option>
              <option value="min">Running minimum</option>
              <option value="max">Running maximum</option>
            </select>
          </label>
          {operation === "sum" && (
            <label>
              Opening value
              <input
                inputMode="decimal"
                value={opening}
                onChange={(event) => setOpening(event.target.value)}
              />
            </label>
          )}
          {!alreadyOrdered && (
            <label>
              Order rows by
              <select value={orderBy} onChange={(event) => setOrderBy(event.target.value)}>
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
              onChange={(event) => setPartitionBy(event.target.value)}
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
        <p className="new-document-note">
          The order and calculation stay visible in Wrangle. Clicking the same
          column’s earlier row brings you here because that is recurrence, not a shift.
        </p>
        <div className="dialog-actions">
          <button type="button" className="secondary-action" onClick={onCancel}>
            Cancel
          </button>
          <button className="primary-action" disabled={!valid}>
            Create running calculation
          </button>
        </div>
      </form>
    </div>
  );
}
