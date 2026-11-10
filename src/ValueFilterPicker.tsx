import { useEffect, useRef, type KeyboardEvent } from "react";
import { useValueFilterPicker } from "./hooks/useValueFilterPicker";
import type { StepDraft } from "./lib/pipelineSteps";
import type { Column } from "./lib/types";
import type { LoadDistinctValues } from "./lib/valueFilter";
import "./ValueFilterPicker.css";

/**
 * Excel's most-used filter gesture: a checklist of the column's values.
 *
 * It is a different way to *write* the Filter step, not a different filter.
 * Committing hands back the whole chain with the column's predicate written
 * or rewritten in the trailing Filter step (`onChangeSteps`), so the filter
 * has one home and the Wrangle chain shows exactly what was done.
 *
 * Every row is a plain checkbox in the ordinary type size; there is no
 * per-row control beyond it, and the list scrolls rather than grows, so a
 * column of twenty values and a column of two hundred are the same shape.
 * Keyboard: arrows move, Space toggles (while the box is empty, since a
 * space typed into a query is a character), Return commits, Escape cancels.
 */
export function ValueFilterPicker({
  frameId,
  column,
  steps,
  loadDistinctValues,
  onChangeSteps,
  onEditInWrangle,
  onClose,
}: {
  frameId: string;
  column: Pick<Column, "id" | "name">;
  steps: StepDraft[];
  loadDistinctValues: LoadDistinctValues;
  onChangeSteps: (steps: StepDraft[]) => void;
  /** The existing draft path: appends a typed predicate for this column. */
  onEditInWrangle: () => void;
  onClose: () => void;
}) {
  const picker = useValueFilterPicker({
    frameId,
    column,
    steps,
    load: loadDistinctValues,
  });
  const search = useRef<HTMLInputElement>(null);
  useEffect(() => search.current?.focus(), [picker.status]);

  const commit = () => {
    if (!picker.canApply) return;
    const next = picker.commit();
    if (next) onChangeSteps(next);
    onClose();
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    } else if (event.key === "Enter") {
      event.preventDefault();
      commit();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      picker.moveActive(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === " " && picker.query === "") {
      event.preventDefault();
      picker.toggleActive();
    }
  };

  if (picker.existing.kind === "foreign")
    return (
      <div className="value-filter" role="group" aria-label={`Filter ${column.name}`}>
        <span className="value-filter-note">
          {column.name} already has a filter written by hand.
        </span>
        <div className="value-filter-foot">
          <button
            onClick={() => {
              onEditInWrangle();
              onClose();
            }}
          >
            Edit in Wrangle
          </button>
        </div>
      </div>
    );

  return (
    <div
      className="value-filter"
      role="group"
      aria-label={`Filter ${column.name}`}
      onKeyDown={onKeyDown}
    >
      <input
        ref={search}
        type="search"
        aria-label={`Search ${column.name} values`}
        placeholder="Search"
        value={picker.query}
        onChange={(event) => picker.setQuery(event.target.value)}
      />
      {picker.status === "error" ? (
        <span className="value-filter-error" role="alert">
          {picker.error}
        </span>
      ) : picker.status === "loading" ? (
        <span className="value-filter-note">Loading…</span>
      ) : (
        <div className="value-filter-list">
          {picker.rows.map((row, index) => (
            <label
              key={row.key}
              className={`value-filter-row ${index === picker.activeIndex ? "active" : ""}`}
            >
              <input
                type="checkbox"
                tabIndex={-1}
                checked={picker.isChecked(row)}
                onChange={() => picker.toggle(row)}
              />
              <span className="value-filter-label">{row.label}</span>
              {row.count !== undefined && (
                <span className="value-filter-count">{row.count}</span>
              )}
            </label>
          ))}
        </div>
      )}
      {picker.shownOf && (
        // Only a count: a list that stops short says so, and the search box
        // above it already says how to reach the rest.
        <span className="value-filter-note">
          {picker.shownOf.shown.toLocaleString()} of {picker.shownOf.total.toLocaleString()}
        </span>
      )}
      <div className="value-filter-foot">
        <button aria-label="Select all" onClick={() => picker.setVisible(true)}>
          All
        </button>
        <button aria-label="Select none" onClick={() => picker.setVisible(false)}>
          None
        </button>
        <span className="value-filter-note" />
        {picker.status === "error" && (
          // The way out when the list cannot be offered (a date-like or
          // over-precise column, an unfinished earlier step): the typed
          // filter has none of those limits.
          <button
            onClick={() => {
              onEditInWrangle();
              onClose();
            }}
          >
            Edit in Wrangle
          </button>
        )}
        <button onClick={commit} disabled={!picker.canApply}>
          Apply
        </button>
      </div>
    </div>
  );
}
