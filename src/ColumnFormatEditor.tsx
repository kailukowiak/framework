import { ColumnDateFormatEditor } from "./ColumnDateFormatEditor";
import { Field } from "./Field";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ColumnFormat,
  ColumnFormatScale,
  ColumnFormatStyle,
  FrameObject,
} from "./lib/types";

/**
 * The columns a number-format change lands on. A date column has its own
 * editor, and it does not gang: dates and numbers do not share a format, so
 * a range mixing them formats the numbers and leaves the dates as they were.
 */
function numberFormatTargets(column: Column, columns: Column[] | undefined): Column[] {
  return (columns?.length ? columns : [column]).filter(
    (candidate) => candidate.dataType !== "date"
  );
}

export function ColumnFormatEditor({
  frame,
  column,
  columns,
  onOperation,
}: {
  frame: FrameObject;
  /** The column whose current format the controls show. */
  column: Column;
  /** Every column a change lands on; defaults to `column` alone. */
  columns?: Column[];
  onOperation: OperationHandler;
}) {
  const targets = numberFormatTargets(column, columns);
  if (column.dataType === "date" && targets.length === 0) {
    return (
      <ColumnDateFormatEditor frame={frame} column={column} onOperation={onOperation} />
    );
  }
  const shown = column.dataType === "date" ? targets[0] : column;
  const format = shown.format ?? null;
  const commit = (next: ColumnFormat | null) => {
    for (const target of targets)
      void onOperation({
        type: "setColumnFormat",
        frameId: frame.id,
        columnId: target.id,
        format: next,
      });
  };
  const patch = (changes: Partial<ColumnFormat>) =>
    commit({ style: "number", ...format, ...changes });
  const isCurrency = format?.style === "currency" || format?.style === "accounting";
  return (
    <div className="column-format-editor">
      <label className="inspector-field">
        {targets.length > 1 ? `Style · ${targets.length} columns` : "Style · whole column"}
        <select
          value={format?.style ?? ""}
          onChange={(event) => {
            const style = event.target.value as ColumnFormatStyle | "";
            if (!style) commit(null);
            else patch({ style });
          }}
        >
          <option value="">Default</option>
          <option value="plain">Plain</option>
          <option value="number">Number</option>
          <option value="currency">Currency</option>
          <option value="accounting">Accounting</option>
          <option value="percent">Percent</option>
        </select>
      </label>
      {format && format.style !== "plain" && (
        <>
          <div className="column-format-row">
            <label className="inspector-field">
              Decimals
              <input
                type="number"
                min={0}
                max={8}
                placeholder="auto"
                value={format.decimals ?? ""}
                onChange={(event) =>
                  patch({
                    decimals:
                      event.target.value === ""
                        ? null
                        : Math.max(
                            0,
                            Math.min(8, Math.trunc(Number(event.target.value)))
                          ),
                  })
                }
              />
            </label>
            <label className="inspector-field">
              Scale
              <select
                value={format.scale ?? "units"}
                onChange={(event) =>
                  patch({ scale: event.target.value as ColumnFormatScale })
                }
              >
                <option value="units">Units</option>
                <option value="thousands">Thousands (K)</option>
                <option value="millions">Millions (M)</option>
              </select>
            </label>
            {isCurrency && (
              <Field
                label="Currency"
                initial={format.currencyCode ?? ""}
                onCommit={(code) =>
                  patch({ currencyCode: code.trim().toUpperCase() || null })
                }
              />
            )}
          </div>
          <label className="column-format-toggle">
            <input
              type="checkbox"
              checked={format.negativeParens ?? format.style === "accounting"}
              onChange={(event) => patch({ negativeParens: event.target.checked })}
            />
            Negatives in parentheses
          </label>
          <label className="column-format-toggle">
            <input
              type="checkbox"
              checked={format.zeroDash ?? format.style === "accounting"}
              onChange={(event) => patch({ zeroDash: event.target.checked })}
            />
            Zero as dash
          </label>
          <small className="column-format-note">
            Display only — stored values keep full precision.
          </small>
        </>
      )}
    </div>
  );
}
