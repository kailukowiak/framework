import type { OperationHandler } from "./lib/handlers";
import type { Column, DatePattern, FrameObject } from "./lib/types";

const DATE_PATTERN_OPTIONS: Array<{ value: DatePattern; label: string }> = [
  { value: "iso", label: "2026-09-01" },
  { value: "dayMonthYear", label: "1 Sep 2026" },
  { value: "monthDayYear", label: "Sep 1, 2026" },
  { value: "monthYear", label: "Sep 2026" },
  { value: "quarter", label: "Q3 2026" },
];

/** A date column's format section: one pattern choice, nothing else. */
export function ColumnDateFormatEditor({
  frame,
  column,
  onOperation,
}: {
  frame: FrameObject;
  column: Column;
  onOperation: OperationHandler;
}) {
  const pattern: DatePattern = column.format?.datePattern ?? "iso";
  return (
    <div className="column-format-editor">
      <label className="inspector-field">
        Date format
        <select
          value={pattern}
          onChange={(event) =>
            void onOperation({
              type: "setColumnFormat",
              frameId: frame.id,
              columnId: column.id,
              format: {
                ...(column.format ?? { style: "plain" }),
                datePattern: event.target.value as DatePattern,
              },
            })
          }
        >
          {DATE_PATTERN_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
