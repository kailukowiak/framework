import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  FunctionSquare,
  KeyRound,
  ListFilter,
  Plus,
} from "lucide-react";
import {
  ColumnFormatBadge,
  EditableColumnHeader,
} from "./FrameCells";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import {
  effectiveFrameCellStyle,
  frameCellStyleProperties,
  isCalculatedFrameColumn,
} from "./FrameGrid";
import { nextSortKeys } from "./lib/sortKeys";
import { formulaToken } from "./lib/formulaReferences";
import type { Column } from "./lib/types";

export function RecordsFrameHeader({
  model,
}: {
  model: RecordsAsRowsFrameCardProps;
}) {
  const {
    frame,
    computed,
    selection,
    virtualRange,
    canAddColumns,
    frameColumnDrop,
    onOperation,
    onSelect,
    selectWholeColumn,
    beginFrameColumnDrag,
    addColumn,
    editCalculatedColumn,
  } = model;
  return (
          <thead>
            <tr>
              <th
                className="row-number-header styled-frame-cell"
                style={frameCellStyleProperties(effectiveFrameCellStyle(frame))}
              />
              {frame.columns.map((column, columnIndex) => {
                const calculated = isCalculatedFrameColumn(computed, column);
                return (
                  <th
                    key={column.id}
                    data-column-id={column.id}
                    title={
                      calculated
                        ? `Select ${column.name} · type = or click its formula below to edit all rows`
                        : `Select ${column.name} · type = to calculate all rows`
                    }
                    onPointerDown={(event) => selectWholeColumn(event, column)}
                    style={frameCellStyleProperties(
                      effectiveFrameCellStyle(frame, undefined, column.id)
                    )}
                    className={`column-header styled-frame-cell selectable-header ${
                      selection?.columnId === column.id ? "active" : ""
                    } ${
                      frameColumnDrop?.columnId === column.id
                        ? frameColumnDrop.after
                          ? "column-drop-after"
                          : "column-drop-before"
                        : ""
                    }`}
                  >
                    <div className="column-header-row">
                      <ColumnSortButton model={model} column={column} />
                      <ColumnFilterButton model={model} column={column} />
                      <button
                        className="column-select"
                        data-reorderable={frame.columns.length > 1 ? "true" : undefined}
                        title="Drag to rearrange columns"
                        onPointerDown={(event) =>
                          beginFrameColumnDrag(event, column.id)
                        }
                        onClick={(event) => {
                          // A pointer press already selected the whole column
                          // with a grid anchor. Do not replace that richer
                          // selection with the inspector-only keyboard form.
                          if (event.detail === 0)
                            onSelect({ objectId: frame.id, columnId: column.id });
                        }}
                      >
                        <EditableColumnHeader
                          frame={frame}
                          column={column}
                          onOperation={onOperation}
                        />
                        <ColumnFormatBadge format={column.format} />
                        {calculated && <FunctionSquare size={13} />}
                        {frame.uniqueKeys.some(
                          (key) =>
                            key.columnIds.length === 1 && key.columnIds[0] === column.id
                        ) && <KeyRound className="unique-key-icon" size={12} />}
                      </button>
                    </div>
                    {canAddColumns && columnIndex < frame.columns.length - 1 && (
                      <button
                        className="column-insert"
                        title={`Insert a column after ${column.name}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          addColumn(column.id);
                        }}
                      >
                        <Plus size={12} />
                      </button>
                    )}
                  </th>
                );
              })}
              <th className="frame-edge-header">
                {canAddColumns && (
                  <button
                    title="Add column"
                    onClick={() => addColumn(frame.columns.at(-1)?.id ?? null)}
                  >
                    <Plus size={14} />
                  </button>
                )}
              </th>
            </tr>
            <tr className="type-row">
              <th />
              {frame.columns.map((column) => (
                <th
                  key={column.id}
                  data-column-id={column.id}
                  className="styled-frame-cell"
                  style={frameCellStyleProperties(
                    effectiveFrameCellStyle(frame, undefined, column.id)
                  )}
                >
                  {isCalculatedFrameColumn(computed, column) ? (
                    <button
                      className="column-formula-declaration"
                      title={`Edit ${column.name} for all rows`}
                      onClick={() => editCalculatedColumn(column, virtualRange.start)}
                    >
                      <FunctionSquare size={11} />
                      <code>{computed.formulas[column.id]}</code>
                    </button>
                  ) : column.dataType === "categorical"
                    ? `categorical · ${column.categories?.length ?? 0}`
                    : column.dataType}
                </th>
              ))}
              <th className="frame-edge-cell" />
            </tr>
          </thead>

  );
}

export function filterUsesColumn(predicates: string[], columnName: string): boolean {
  const token = formulaToken(columnName);
  return predicates.some((predicate) => predicate.includes(token));
}

function ColumnFilterButton({
  model,
  column,
}: {
  model: RecordsAsRowsFrameCardProps;
  column: Column;
}) {
  const active = filterUsesColumn(model.filterPredicates, column.name);
  return (
    <button
      className={`column-filter-toggle ${active ? "active" : ""}`}
      title={`Filter ${column.name} in Wrangle`}
      aria-label={`Filter ${column.name}`}
      onClick={(event) => {
        event.stopPropagation();
        model.filterColumn(column);
      }}
    >
      <ListFilter size={11} />
    </button>
  );
}

function ColumnSortButton({
  model,
  column,
}: {
  model: RecordsAsRowsFrameCardProps;
  column: Column;
}) {
  const { frame, sortKeys, onOperation } = model;
  const sortIndex = sortKeys.findIndex((key) => key.columnId === column.id);
  const sortKey = sortIndex >= 0 ? sortKeys[sortIndex] : null;
  const title = sortKey
    ? `Sort key ${sortIndex + 1} of ${sortKeys.length}, ${
        !sortKey.descending ? "ascending" : "descending"
      } — click for ${
        !sortKey.descending ? "descending" : "no sort"
      }, shift-click to sort by this column alone`
    : sortKeys.length
      ? `Click to add as sort key ${
          sortKeys.length + 1
        } · shift-click to sort by this column alone`
      : "Click to sort · click again for descending";
  return (
    <button
      className={`column-sort-toggle ${sortKey ? "active" : ""}`}
      title={title}
      aria-label={`Sort by ${column.name}`}
      onClick={(event) => {
        event.stopPropagation();
        void onOperation({
          type: "setFrameDisplaySort",
          frameId: frame.id,
          keys: nextSortKeys(
            sortKeys,
            column.id,
            event.shiftKey ? "only" : "accumulate"
          ),
        });
      }}
    >
      {/* An unsorted column still renders a muted glyph: an empty button is
          invisible, so the sort affordance has to be discoverable before the
          first click. */}
      {sortKey ? (
        !sortKey.descending ? (
          <ArrowUp size={11} />
        ) : (
          <ArrowDown size={11} />
        )
      ) : (
        <ArrowUpDown size={11} />
      )}
      {/* Every applied key carries its ordinal, so the precedence of a
          multi-column sort is readable from the headers. */}
      {sortKey && sortKeys.length > 1 && (
        <span className="column-sort-ordinal">{sortIndex + 1}</span>
      )}
    </button>
  );
}
