import { useEffect, useMemo, useState, type CSSProperties, type RefObject } from "react";
import { getFrameSummary, type FrameSummary } from "./lib/api";
import { summaryFormulaToken } from "./lib/formulaPicking";
import { formulaToken } from "./lib/formulaReferences";
import type { DataType, SummaryOperation, FrameObject } from "./lib/types";

export const PROFILE_SUMMARY_ROWS: SummaryOperation[] = [
  "count",
  "missing",
  "countDistinct",
  "min",
  "quartile25",
  "mean",
  "median",
  "quartile75",
  "max",
  "sum",
  "mode",
];

const SUMMARY_CHOICES: Array<{
  operation: SummaryOperation;
  label: string;
  title: string;
}> = [
  { operation: "count", label: "Count", title: "Non-missing values" },
  { operation: "missing", label: "Nulls", title: "Missing values" },
  { operation: "countDistinct", label: "Distinct", title: "Distinct non-missing values" },
  { operation: "min", label: "Min", title: "Minimum" },
  { operation: "quartile25", label: "25%", title: "First quartile" },
  { operation: "mean", label: "Mean", title: "Arithmetic mean" },
  { operation: "median", label: "50%", title: "Median" },
  { operation: "quartile75", label: "75%", title: "Third quartile" },
  { operation: "max", label: "Max", title: "Maximum" },
  { operation: "sum", label: "Sum", title: "Sum" },
  { operation: "mode", label: "Mode", title: "Most frequent value" },
];

export type FrameSummaryState = {
  data: FrameSummary | null;
  loading: boolean;
  error: string | null;
};

/** New row configuration, falling back to column summaries in older files. */
export function displayedSummaryRows(frame: FrameObject): SummaryOperation[] {
  if (frame.display?.summaryRows != null) return frame.display.summaryRows;
  const operations: SummaryOperation[] = [];
  for (const summary of frame.summaries) {
    if (!operations.includes(summary.operation)) operations.push(summary.operation);
  }
  return operations;
}

/** Fetches the expensive profile only while there is something to draw. */
export function useFrameSummary(
  frame: FrameObject,
  fingerprint: string,
  enabled = true
): FrameSummaryState {
  const operations = displayedSummaryRows(frame);
  const signature = useMemo(
    () =>
      JSON.stringify([
        frame.id,
        enabled,
        fingerprint,
        operations,
        frame.summaries.map((summary) => [
          summary.columnId,
          summary.operation,
        ]),
      ]),
    [enabled, fingerprint, operations, frame.id, frame.summaries]
  );
  const [state, setState] = useState<FrameSummaryState>({
    data: null,
    loading: false,
    error: null,
  });

  useEffect(() => {
    if (!enabled || !operations.length) {
      setState({ data: null, loading: false, error: null });
      return;
    }
    let disposed = false;
    setState({ data: null, loading: true, error: null });
    void getFrameSummary(frame.id)
      .then((data) => {
        if (!disposed) setState({ data, loading: false, error: null });
      })
      .catch((reason) => {
        if (!disposed)
          setState({
            data: null,
            loading: false,
            error: String(reason).replace(/^Error:\s*/, ""),
          });
      });
    return () => {
      disposed = true;
    };
    // `signature` contains every input that should trigger this scan while
    // leaving unrelated document edits out of it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature, frame.id]);

  return state;
}

function summarySupports(
  operation: SummaryOperation,
  dataType: DataType
): boolean {
  const numeric = new Set<DataType>([
    "integer",
    "number",
    "currency",
    "percentage",
  ]).has(dataType);
  if (["sum", "mean", "quartile25", "median", "quartile75"].includes(operation))
    return numeric;
  if (operation === "min" || operation === "max")
    return numeric || dataType === "date";
  if (operation === "mode")
    return dataType === "string" || dataType === "categorical";
  return true;
}

export function FrameSummaryDrawer({
  frame,
  summary,
  height,
  drawerRef,
  scrollRef,
  onScroll,
  onResize,
  onSetRows,
}: {
  frame: FrameObject;
  summary: FrameSummaryState;
  height: number;
  drawerRef: RefObject<HTMLElement | null>;
  scrollRef: RefObject<HTMLDivElement | null>;
  onScroll: (scrollLeft: number) => void;
  onResize: (event: React.PointerEvent<HTMLButtonElement>) => void;
  onSetRows: (operations: SummaryOperation[]) => void;
}) {
  const operations = displayedSummaryRows(frame);
  const resultRows = new Map(
    summary.data?.rows.map((row) => [row.operation, row]) ?? []
  );
  const set = new Set(operations);
  const toggle = (operation: SummaryOperation) => {
    if (set.has(operation)) set.delete(operation);
    else set.add(operation);
    onSetRows(
      SUMMARY_CHOICES.map((choice) => choice.operation).filter((choice) =>
        set.has(choice)
      )
    );
  };
  return (
    <section
      ref={drawerRef}
      className="summary-drawer"
      aria-label={`${frame.name} profile`}
      style={{ "--summary-drawer-height": `${height}px` } as CSSProperties}
    >
      <button
        type="button"
        className="summary-drawer-resize"
        aria-label="Resize profile drawer"
        title="Drag to show more or fewer statistics"
        onPointerDown={onResize}
      />
      <div className="summary-drawer-controls">
        <span>Stats</span>
        <div className="summary-picker-actions" role="toolbar" aria-label="Statistics">
          <button
            type="button"
            className="summary-preset"
            aria-pressed={PROFILE_SUMMARY_ROWS.every((operation) => set.has(operation))}
            onClick={() => onSetRows(PROFILE_SUMMARY_ROWS)}
          >
            Basic profile
          </button>
          {SUMMARY_CHOICES.map((choice) => (
            <button
              type="button"
              key={choice.operation}
              aria-pressed={set.has(choice.operation)}
              title={choice.title}
              onClick={() => toggle(choice.operation)}
            >
              {choice.label}
            </button>
          ))}
          <button type="button" onClick={() => onSetRows([])}>None</button>
        </div>
      </div>
      <div
        className="frame-scroll summary-frame-scroll"
        ref={scrollRef}
        onScroll={(event) => onScroll(event.currentTarget.scrollLeft)}
      >
        <table
          aria-rowcount={operations.length}
          style={{ minWidth: Math.max(360, frame.columns.length * 150 + 66) }}
        >
          <colgroup>
            <col className="row-number-column" />
            {frame.columns.map((column) => <col key={column.id} />)}
            <col className="frame-edge-column" />
          </colgroup>
          <tbody>
            {operations.map((operation) => {
              const result = resultRows.get(operation);
              const label =
                SUMMARY_CHOICES.find((choice) => choice.operation === operation)?.label ??
                operation;
              return (
                <tr className="summary-row" key={operation}>
                  <th scope="row" title={label}>{label}</th>
                  {frame.columns.map((column) => {
                    const cell = result?.cells[column.id];
                    const supported = summarySupports(operation, column.dataType);
                    const token = summaryFormulaToken(operation, formulaToken(column.name));
                    return (
                      <td
                        key={column.id}
                        data-column-id={column.id}
                        data-summary-operation={operation}
                        data-summary-referenceable={supported ? "true" : undefined}
                        title={
                          cell?.error ??
                          summary.error ??
                          (supported ? `Formula: ${token}` : `${label} does not apply`)
                        }
                        className={supported ? "summary-referenceable" : "not-applicable"}
                      >
                        {summary.loading
                          ? "…"
                          : summary.error || cell?.error
                            ? "!"
                            : cell?.display ?? "n/a"}
                      </td>
                    );
                  })}
                  <td className="frame-edge-cell" />
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
