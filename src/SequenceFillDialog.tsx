import { X } from "lucide-react";
import { useEffect, useState } from "react";

export function sequenceColumnFormula(start: number, step: number): string {
  const stop =
    step < 0
      ? `${start} - ${Math.abs(step)} * frame.len()`
      : `${start} + ${step} * frame.len()`;
  return `sequence(${start}, ${stop}, step=${step})`;
}

export function dateSequenceColumnFormula(
  start: string,
  step: number,
  unit: "d" | "mo"
): string {
  return `sequence(${start}, periods=frame.len(), step=${step}${unit})`;
}

/** A click-first authoring surface for a durable, row-count-aware series. */
export function SequenceFillDialog({
  columnName,
  orderColumns,
  alreadyOrdered,
  initialStart,
  initialStep = 1,
  kind = "number",
  dateUnit = "d",
  onApply,
  onCancel,
}: {
  columnName: string;
  orderColumns: Array<{ id: string; name: string }>;
  alreadyOrdered: boolean;
  initialStart: number | string;
  initialStep?: number;
  kind?: "number" | "date";
  dateUnit?: "d" | "mo";
  onApply: (formula: string, orderByColumnId?: string) => void;
  onCancel: () => void;
}) {
  const [start, setStart] = useState(String(initialStart));
  const [step, setStep] = useState(String(initialStep));
  const [orderBy, setOrderBy] = useState(orderColumns[0]?.id ?? "");
  const startNumber = Number(start);
  const stepNumber = Number(step);
  const validDate = /^\d{4}-\d{2}-\d{2}$/.test(start) && !Number.isNaN(Date.parse(start));
  const valid =
    (kind === "date" ? validDate : Number.isInteger(startNumber)) &&
    Number.isInteger(stepNumber) &&
    stepNumber !== 0 &&
    (alreadyOrdered || Boolean(orderBy));
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onCancel]);
  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <form
        className="insert-dialog sequence-fill-dialog"
        onSubmit={(event) => {
          event.preventDefault();
          if (valid)
            onApply(
              kind === "date"
                ? dateSequenceColumnFormula(start, stepNumber, dateUnit)
                : sequenceColumnFormula(startNumber, stepNumber),
              alreadyOrdered ? undefined : orderBy
            );
        }}
      >
        <div className="dialog-header">
          <div>
            <span className="eyebrow">FILL SERIES</span>
            <h2>{columnName}</h2>
          </div>
          <button type="button" className="icon-button" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>
        <div className="sequence-fill-fields">
          <label>
            {kind === "date" ? "Starting date" : "Starting number"}
            <input
              autoFocus
              inputMode={kind === "date" ? undefined : "numeric"}
              value={start}
              onChange={(event) => setStart(event.target.value)}
            />
          </label>
          <label>
            Change each row{kind === "date" ? ` (${dateUnit === "mo" ? "months" : "days"})` : ""}
            <input
              inputMode="numeric"
              value={step}
              onChange={(event) => setStep(event.target.value)}
            />
          </label>
        </div>
        {!alreadyOrdered && (
          <label>
            Order rows by
            <select value={orderBy} onChange={(event) => setOrderBy(event.target.value)}>
              {orderColumns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name} — ascending
                </option>
              ))}
            </select>
          </label>
        )}
        <p className="new-document-note">
          {alreadyOrdered
            ? "The existing Wrangle order is used. The series grows when the frame grows."
            : "The order and series become visible Wrangle steps and grow with the frame."}
        </p>
        <div className="dialog-actions">
          <button type="button" className="secondary-action" onClick={onCancel}>
            Cancel
          </button>
          <button className="primary-action" disabled={!valid}>
            Fill column
          </button>
        </div>
      </form>
    </div>
  );
}
