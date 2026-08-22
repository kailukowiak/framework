import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  FunctionSquare,
  KeyRound,
  ListFilter,
  Plus,
} from "lucide-react";
import { useState, type Dispatch, type SetStateAction } from "react";
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
import type { GridRange } from "./lib/gridNavigation";
import { hasVectorDrag, readVectorDrag } from "./lib/vectorDrag";

export function RecordsFrameHeader({
  model,
}: {
  model: RecordsAsRowsFrameCardProps;
}) {
  const {
    frame,
    computed,
    virtualRange,
    canAddColumns,
    addColumn,
    editCalculatedColumn,
    pairVector,
  } = model;
  const [vectorTarget, setVectorTarget] = useState<string | null>(null);
  return (
          <thead>
            <tr>
              <th
                className="row-number-header styled-frame-cell"
                style={frameCellStyleProperties(effectiveFrameCellStyle(frame))}
              />
              {frame.columns.map((column, columnIndex) => (
                <RecordsColumnHeader
                  key={column.id}
                  model={model}
                  column={column}
                  columnIndex={columnIndex}
                  vectorTarget={vectorTarget}
                  setVectorTarget={setVectorTarget}
                />
              ))}
              <VectorFrameEdgeHeader
                canAddColumns={canAddColumns}
                lastColumnId={frame.columns.at(-1)?.id}
                addColumn={addColumn}
                pairVector={pairVector}
              />
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

function RecordsColumnHeader({
  model,
  column,
  columnIndex,
  vectorTarget,
  setVectorTarget,
}: {
  model: RecordsAsRowsFrameCardProps;
  column: Column;
  columnIndex: number;
  vectorTarget: string | null;
  setVectorTarget: Dispatch<SetStateAction<string | null>>;
}) {
  const { frame, computed, selection, frameColumnDrop } = model;
  const calculated = isCalculatedFrameColumn(computed, column);
  return (
    <th
      data-column-id={column.id}
      title={
        calculated
          ? `Select ${column.name} · type = or click its formula below to edit all rows`
          : `Select ${column.name} · type = to calculate all rows`
      }
      onPointerDown={(event) => model.selectWholeColumn(event, column)}
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
      } ${vectorTarget === column.id ? "vector-column-drop" : ""}`}
      onDragOver={(event) => {
        if (!hasVectorDrag(event.dataTransfer)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "link";
        setVectorTarget(column.id);
      }}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node))
          setVectorTarget((current) => (current === column.id ? null : current));
      }}
      onDrop={(event) => {
        const vector = readVectorDrag(event.dataTransfer);
        setVectorTarget(null);
        if (!vector) return;
        event.preventDefault();
        event.stopPropagation();
        const targets = vectorTargetColumns(
          frame.columns,
          columnIndex,
          model.selectionRange,
          vector.length
        );
        model.applyVector(
          targets.map((candidate) => candidate.id),
          vector.formula,
          vector.length
        );
      }}
    >
      <div className="column-header-row">
        <ColumnSortButton model={model} column={column} />
        <ColumnFilterButton model={model} column={column} />
        <button
          className="column-select"
          data-reorderable={frame.columns.length > 1 ? "true" : undefined}
          title="Drag to rearrange columns"
          onPointerDown={(event) => model.beginFrameColumnDrag(event, column.id)}
          onClick={(event) => {
            // A pointer press already selected the whole column with a grid
            // anchor. Keyboard activation supplies the simpler selection.
            if (event.detail === 0)
              model.onSelect({ objectId: frame.id, columnId: column.id });
          }}
        >
          <EditableColumnHeader
            frame={frame}
            column={column}
            onOperation={model.onOperation}
          />
          <ColumnFormatBadge format={column.format} />
          {calculated && <FunctionSquare size={13} />}
          {frame.uniqueKeys.some(
            (key) => key.columnIds.length === 1 && key.columnIds[0] === column.id
          ) && <KeyRound className="unique-key-icon" size={12} />}
        </button>
      </div>
      {model.canAddColumns && columnIndex < frame.columns.length - 1 && (
        <button
          className="column-insert"
          title={`Insert a column after ${column.name}`}
          onClick={(event) => {
            event.stopPropagation();
            model.addColumn(column.id);
          }}
        >
          <Plus size={12} />
        </button>
      )}
    </th>
  );
}

function VectorFrameEdgeHeader({
  canAddColumns,
  lastColumnId,
  addColumn,
  pairVector,
}: {
  canAddColumns: boolean;
  lastColumnId?: string;
  addColumn: (afterColumnId: string | null) => void;
  pairVector: (name: string, vector: string, expectedLength: number) => void;
}) {
  return (
    <th
      className="frame-edge-header"
      title="Drop a list here to add it as a column"
      onDragOver={(event) => {
        if (!hasVectorDrag(event.dataTransfer)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "link";
        event.currentTarget.classList.add("vector-column-drop");
      }}
      onDragLeave={(event) =>
        event.currentTarget.classList.remove("vector-column-drop")
      }
      onDrop={(event) => {
        event.currentTarget.classList.remove("vector-column-drop");
        const vector = readVectorDrag(event.dataTransfer);
        if (!vector) return;
        event.preventDefault();
        event.stopPropagation();
        pairVector(vector.name, vector.formula, vector.length);
      }}
    >
      {canAddColumns && (
        <button title="Add column" onClick={() => addColumn(lastColumnId ?? null)}>
          <Plus size={14} />
        </button>
      )}
    </th>
  );
}

/**
 * A selected header run wins when the list fits it evenly; otherwise the
 * drop spills right from the pointed header, like pasting a row in Excel.
 */
export function vectorTargetColumns<T>(
  columns: T[],
  columnIndex: number,
  selectionRange: GridRange | null,
  vectorLength: number
): T[] {
  const selected =
    selectionRange &&
    columnIndex >= selectionRange.left &&
    columnIndex <= selectionRange.right
      ? columns.slice(selectionRange.left, selectionRange.right + 1)
      : [];
  return selected.length > 0 && selected.length % vectorLength === 0
    ? selected
    : columns.slice(columnIndex, Math.min(columns.length, columnIndex + vectorLength));
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
