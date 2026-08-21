import { gridRangeForFocus, type GridContext, type GridFocus } from "./FrameGrid";
import { displayedSummaryRows } from "./FrameSummaryFooter";
import { selectionStatistics } from "./lib/selectionStatistics";
import type { SummaryOperation } from "./lib/types";

const numericTypes = new Set(["integer", "number", "currency", "percentage"]);

function formatStatistic(value: number): string {
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 4,
  }).format(value);
}

/** Excel-style ephemeral readouts, plus an explicit path to a saved summary. */
export function SelectionStatisticsStatus({
  context,
  focus,
  onAddSummary,
}: {
  context: GridContext | null;
  focus: GridFocus | null;
  onAddSummary: (operation: SummaryOperation) => void;
}) {
  if (!context || !focus) return null;
  const statistics = selectionStatistics(context, focus);
  const range = gridRangeForFocus(context, focus);
  const selectsOneColumn = Boolean(
    range &&
      (context.orientation === "fieldsAsRows"
        ? range.top === range.bottom
        : range.left === range.right)
  );
  const summaryColumn =
    focus.span === "column" && range && selectsOneColumn
      ? context.frame.columns[
          context.orientation === "fieldsAsRows" ? range.top : range.left
        ]
      : null;
  if (!statistics && !summaryColumn) return null;

  const operations: Array<{ operation: SummaryOperation; label: string }> = [
    ...(summaryColumn && numericTypes.has(summaryColumn.dataType)
      ? [
          { operation: "sum" as const, label: "Sum" },
          { operation: "mean" as const, label: "Average" },
        ]
      : []),
    { operation: "count", label: "Count" },
  ];
  const summaryRows = displayedSummaryRows(context.frame);
  return (
    <div className="selection-statistics" aria-label="Selection statistics">
      {statistics?.partial ? (
        <span>{statistics.selectedCells.toLocaleString()} selected</span>
      ) : statistics ? (
        <>
          <span>Count {statistics.count}</span>
          {statistics.sum !== null && <span>Sum {formatStatistic(statistics.sum)}</span>}
          {statistics.average !== null && (
            <span>Average {formatStatistic(statistics.average)}</span>
          )}
        </>
      ) : null}
      {summaryColumn && (
        <span className="selection-summary-actions">
          {operations.map(({ operation, label }) => {
            const exists = summaryRows.includes(operation);
            return (
              <button
                key={operation}
                disabled={exists}
                title={
                  exists
                    ? `${label} summary already shown`
                    : `Keep ${label.toLowerCase()} as a summary row`
                }
                onClick={() => onAddSummary(operation)}
              >
                {exists ? `✓ ${label}` : `Keep ${label}`}
              </button>
            );
          })}
        </span>
      )}
    </div>
  );
}
