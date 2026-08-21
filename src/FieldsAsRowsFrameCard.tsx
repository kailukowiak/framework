import { FunctionSquare, GitBranch, Plus } from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from "react";
import {
  ColumnFormatBadge,
  EditableColumnHeader,
  GridCellContent,
} from "./FrameCells";
import {
  effectiveFrameCellStyle,
  frameCellStyleProperties,
  isCalculatedFrameColumn,
  type CellPointerHandler,
  type FrameStyleMatches,
  type GridFocus,
  type GridFocusMode,
} from "./FrameGrid";
import {
  normalizeRange,
  positionInRange,
  scrollLeftToRevealColumn,
  scrollTopToRevealRow,
  type GridDirection,
  type GridPosition,
} from "./lib/gridNavigation";
import { expandRangeForSpan } from "./lib/gridSpan";
import type { OperationHandler } from "./lib/handlers";
import { calculateVirtualRowRange } from "./lib/frameVirtualization";
import type {
  Column,
  ComputedFrame,
  Row,
  Selection,
  FrameObject,
} from "./lib/types";

export function FramePageControls({
  offset,
  loaded,
  total,
  loading,
  error,
  onPrevious,
  onNext,
}: {
  offset: number;
  loaded: number;
  total: number;
  loading: boolean;
  error: string | null;
  onPrevious: () => void;
  onNext: () => void;
}) {
  return (
    <div className="frame-page-controls">
      <span>
        {error
          ? error
          : loading && loaded === 0
          ? "Loading rows…"
          : total === 0
          ? "No rows"
          : `${(offset + 1).toLocaleString()}–${Math.min(
              total,
              offset + loaded
            ).toLocaleString()} of ${total.toLocaleString()}`}
      </span>
      <div>
        <button disabled={loading || offset === 0} onClick={onPrevious}>
          Previous
        </button>
        <button disabled={loading || offset + loaded >= total} onClick={onNext}>
          Next
        </button>
      </div>
    </div>
  );
}

// Status line for the scroll-driven records-as-rows paged path: no
// Previous/Next (the virtualizer drives fetching directly off scroll
// position), just row count plus a load error if the last fetch failed.
// The row count itself, and a "· loading…" suffix, already render inline
// in the frame's title row; this only surfaces errors.
export function FramePagedStatus({
  total,
  loading,
  error,
}: {
  total: number;
  loading: boolean;
  error: string | null;
}) {
  if (!error) return null;
  return (
    <div className="frame-page-controls frame-page-error">
      <span>{error}</span>
      <span>{loading ? "retrying…" : `${total.toLocaleString()} rows`}</span>
    </div>
  );
}

function TransposedFieldName({
  frame,
  column,
  calculated,
  onOperation,
}: {
  frame: FrameObject;
  column: Column;
  calculated: boolean;
  onOperation: OperationHandler;
}) {
  return (
    <span>
      <EditableColumnHeader
        frame={frame}
        column={column}
        onOperation={onOperation}
      />
      <ColumnFormatBadge format={column.format} />
      {calculated && <FunctionSquare size={12} />}
    </span>
  );
}

export function FieldsAsRowsFrameCard({
  frame,
  rows,
  styleMatches,
  computed,
  selection,
  gridFocus,
  transformationLabels,
  draftRow,
  setDraftRow,
  commitDraftRow,
  addColumn,
  onSelect,
  onFocusCell,
  onCellPointerDown,
  onCellPointerEnter,
  onCommitCell,
  onSettleCell,
  onEditCalculatedColumn,
  onOperation,
  readOnly,
  rowOffset,
  totalRows,
  footer,
}: {
  frame: FrameObject;
  rows: Row[];
  styleMatches: FrameStyleMatches;
  computed: ComputedFrame;
  selection: Selection | null;
  gridFocus: GridFocus | null;
  transformationLabels: Array<string | null>;
  draftRow: Record<string, string>;
  setDraftRow: Dispatch<SetStateAction<Record<string, string>>>;
  commitDraftRow: (allowEmpty?: boolean) => void;
  addColumn: (afterColumnId: string | null) => void;
  onSelect: (selection: Selection) => void;
  onFocusCell: (
    row: Row,
    column: Column,
    mode: GridFocusMode,
    options?: { extend?: boolean }
  ) => void;
  onCellPointerDown: CellPointerHandler;
  onCellPointerEnter: CellPointerHandler;
  onCommitCell: (
    row: Row,
    column: Column,
    raw: string,
    move: GridDirection | null
  ) => void;
  onSettleCell: (row: Row, column: Column) => void;
  onEditCalculatedColumn: (column: Column, rowIndex: number) => void;
  onOperation: OperationHandler;
  readOnly: boolean;
  rowOffset: number;
  totalRows: number;
  footer?: ReactNode;
}) {
  const isDerived = Boolean(frame.derivation);
  const isPaged = Boolean(computed?.paged);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [scrollState, setScrollState] = useState({
    top: 0,
    left: 0,
    height: 300,
    width: 600,
  });
  const TRANSPOSED_RECORD_WIDTH = 150;
  const TRANSPOSED_FIELD_HEADER_WIDTH = 180;

  // Keep the active cell visible: fields scroll vertically, records horizontally.
  useEffect(() => {
    if (!gridFocus) return;
    const element = scrollRef.current;
    if (!element) return;
    const fieldIndex = frame.columns.findIndex(
      (column) => column.id === gridFocus.columnId
    );
    if (fieldIndex >= 0) {
      const nextTop = scrollTopToRevealRow(
        fieldIndex,
        frame.columns.length,
        element.scrollTop,
        element.clientHeight
      );
      if (nextTop !== null) element.scrollTop = nextTop;
    }
    const recordIndex = rows.findIndex((row) => row.id === gridFocus.rowId);
    if (recordIndex >= 0) {
      const nextLeft = scrollLeftToRevealColumn(
        TRANSPOSED_FIELD_HEADER_WIDTH + recordIndex * TRANSPOSED_RECORD_WIDTH,
        TRANSPOSED_RECORD_WIDTH,
        element.scrollLeft,
        element.clientWidth,
        TRANSPOSED_FIELD_HEADER_WIDTH
      );
      if (nextLeft !== null) element.scrollLeft = nextLeft;
    }
    // Only re-scroll when the active cell moves, not on unrelated document
    // refreshes. frame.columns and rows are still read fresh every time this
    // effect fires, since a gridFocus change always comes with a render
    // carrying current props — they just should not themselves retrigger it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gridFocus?.rowId, gridFocus?.columnId]);

  // Visual coordinates in this orientation: rows are fields, columns are records.
  const gridPositionOf = (rowId: string, columnId: string): GridPosition | null => {
    const recordIndex = rows.findIndex((row) => row.id === rowId);
    const fieldIndex = frame.columns.findIndex((column) => column.id === columnId);
    return recordIndex >= 0 && fieldIndex >= 0
      ? { row: fieldIndex, col: recordIndex }
      : null;
  };
  const rangeAnchor = gridFocus?.anchor
    ? gridPositionOf(gridFocus.anchor.rowId, gridFocus.anchor.columnId)
    : null;
  const rangeFocus = gridFocus
    ? gridPositionOf(gridFocus.rowId, gridFocus.columnId)
    : null;
  // Same spans, opposite axes: with fields as rows, a frame row runs across
  // the screen and a frame column runs down it.
  const selectionRange =
    rangeFocus && (rangeAnchor || gridFocus?.span)
      ? expandRangeForSpan(
          normalizeRange(rangeAnchor ?? rangeFocus, rangeFocus),
          gridFocus?.span ?? null,
          { rowCount: frame.columns.length, columnCount: rows.length },
          true
        )
      : null;

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const updateSize = () =>
      setScrollState((current) => ({
        ...current,
        height: element.clientHeight,
        width: element.clientWidth,
      }));
    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const fieldRange = useMemo(
    () =>
      calculateVirtualRowRange(
        frame.columns.length,
        scrollState.top,
        scrollState.height
      ),
    [scrollState.height, scrollState.top, frame.columns.length]
  );
  const fieldColumns = frame.columns.slice(fieldRange.start, fieldRange.end);
  const recordCount = rows.length + (readOnly ? 0 : 1);
  const recordStart = Math.max(
    0,
    Math.floor(Math.max(0, scrollState.left - 180) / 150) - 2
  );
  const recordEnd = Math.min(
    recordCount,
    Math.ceil((Math.max(0, scrollState.left - 180) + scrollState.width) / 150) + 2
  );
  const visibleRecordIndexes = Array.from(
    { length: Math.max(0, recordEnd - recordStart) },
    (_, index) => recordStart + index
  );
  const paddingBefore = recordStart * 150;
  const paddingAfter = Math.max(0, recordCount - recordEnd) * 150;
  const renderedColumnCount =
    2 + visibleRecordIndexes.length + (paddingBefore ? 1 : 0) + (paddingAfter ? 1 : 0);
  const rowAt = (index: number): Row | null => rows[index] ?? null;

  const renderDraftCell = (column: Column) =>
    isCalculatedFrameColumn(computed, column) ? (
      <span className="draft-formula">ƒ</span>
    ) : column.dataType === "categorical" ? (
      <select
        className="categorical-cell"
        aria-label={`New record ${column.name}`}
        value={draftRow[column.id] ?? ""}
        onChange={(event) =>
          setDraftRow((current) => ({ ...current, [column.id]: event.target.value }))
        }
      >
        <option value="">—</option>
        {(column.categories ?? []).map((category) => (
          <option key={category}>{category}</option>
        ))}
      </select>
    ) : (
      <input
        aria-label={`New record ${column.name}`}
        value={draftRow[column.id] ?? ""}
        placeholder={
          column.dataType === "date"
            ? "YYYY-MM-DD"
            : column.dataType === "boolean"
            ? "true / false"
            : ""
        }
        onChange={(event) =>
          setDraftRow((current) => ({ ...current, [column.id]: event.target.value }))
        }
        onKeyDown={(event) => {
          if (event.key === "Enter") commitDraftRow();
        }}
      />
    );

  return (
    <div className="frame-card fields-as-rows" data-frame-id={frame.id}>
      <div className="frame-title-row">
        <input
          className="frame-name"
          // An input is 20 characters wide whatever is in it, and the row was
          // spacing itself around that rather than around the name.
          size={Math.max(6, Math.min(30, frame.name.length))}
          defaultValue={frame.name}
          key={frame.name}
          onBlur={(event) => {
            if (event.target.value !== frame.name)
              onOperation({
                type: "renameObject",
                objectId: frame.id,
                name: event.target.value,
              });
          }}
        />
        <span>
          <button
            className="frame-orientation-toggle active"
            title="Display records as rows"
            onClick={() =>
              onOperation({
                type: "setFrameDisplayOrientation",
                frameId: frame.id,
                orientation: "recordsAsRows",
              })
            }
          >
            Records ↓
          </button>
          {isDerived ? (
            <>
              <GitBranch size={12} /> {transformationLabels.join(" · ")} ·{" "}
            </>
          ) : null}
          {/* `totalRows`, not `frame.rows.length`: a paged frame holds no rows of
    its own, so the document's count is 0 and the denominator would read
    "24 of 0 records". They are the same number for an in-memory frame. */}
          {readOnly && !isDerived
            ? `${totalRows.toLocaleString()} records · imported`
            : rows.length !== totalRows
            ? `${rows.length.toLocaleString()} of ${totalRows.toLocaleString()} records`
            : `${totalRows.toLocaleString()} records`}
        </span>
      </div>
      <div
        className="frame-scroll transposed-frame-scroll"
        ref={scrollRef}
        onScroll={(event) => {
          // Read synchronously: the updater runs after React nulls currentTarget.
          const { scrollTop, scrollLeft } = event.currentTarget;
          setScrollState((current) => ({
            ...current,
            top: scrollTop,
            left: scrollLeft,
          }));
        }}
      >
        <table
          aria-rowcount={frame.columns.length + 2}
          style={{ width: Math.max(360, 180 + recordCount * 150 + 26) }}
        >
          <colgroup>
            <col style={{ width: 180 }} />
            {paddingBefore > 0 && <col style={{ width: paddingBefore }} />}
            {visibleRecordIndexes.map((index) => (
              <col key={index} style={{ width: 150 }} />
            ))}
            {paddingAfter > 0 && <col style={{ width: paddingAfter }} />}
            <col className="frame-edge-column" />
          </colgroup>
          <thead>
            <tr>
              <th
                className="transposed-corner styled-frame-cell"
                style={frameCellStyleProperties(effectiveFrameCellStyle(frame))}
              >
                Field
              </th>
              {paddingBefore > 0 && (
                <th className="transposed-spacer" aria-hidden="true" />
              )}
              {visibleRecordIndexes.map((index) => {
                const row = rowAt(index);
                return row ? (
                  <th
                    key={row.id}
                    className="transposed-record-header styled-frame-cell"
                    data-row-id={row.id}
                    data-row-index={rowOffset + index}
                    style={frameCellStyleProperties(
                      effectiveFrameCellStyle(frame, row.id, undefined, styleMatches)
                    )}
                  >
                    <button
                      onClick={() => onSelect({ objectId: frame.id, rowId: row.id })}
                    >
                      Record {rowOffset + index + 1}
                    </button>
                  </th>
                ) : (
                  <th
                    key="draft-record"
                    className="transposed-record-header draft-record-header"
                  >
                    <button title="Add record" onClick={() => commitDraftRow(true)}>
                      <Plus size={12} /> New
                    </button>
                  </th>
                );
              })}
              {paddingAfter > 0 && (
                <th className="transposed-spacer" aria-hidden="true" />
              )}
              <th className="frame-edge-header" />
            </tr>
          </thead>
          <tbody>
            {fieldRange.paddingTop > 0 && (
              <tr className="virtual-spacer" aria-hidden="true">
                <td
                  colSpan={renderedColumnCount}
                  style={{ height: fieldRange.paddingTop }}
                />
              </tr>
            )}
            {fieldColumns.map((column, fieldSliceIndex) => {
              const fieldIndex = fieldRange.start + fieldSliceIndex;
              const summaryText = frame.summaries
                .filter((summary) => summary.columnId === column.id)
                .map((summary) => {
                  const result = computed.summaries[summary.id];
                  return `${summary.label} ${result?.error ? "!" : result?.display ?? "…"}`;
                })
                .join(" · ");
              return (
                <tr key={column.id} data-column-id={column.id}>
                  <th
                    className={`transposed-field-header styled-frame-cell ${
                      selection?.columnId === column.id ? "active" : ""
                    }`}
                    style={frameCellStyleProperties(
                      effectiveFrameCellStyle(frame, undefined, column.id)
                    )}
                  >
                    <button
                      onClick={() =>
                        onSelect({ objectId: frame.id, columnId: column.id })
                      }
                    >
                      <TransposedFieldName
                        frame={frame}
                        column={column}
                        calculated={isCalculatedFrameColumn(computed, column)}
                        onOperation={onOperation}
                      />
                      <small>
                        {isCalculatedFrameColumn(computed, column)
                          ? computed.formulas[column.id]
                          : column.dataType}
                        {summaryText && ` · ${summaryText}`}
                      </small>
                    </button>
                  </th>
                  {paddingBefore > 0 && (
                    <td className="transposed-spacer" aria-hidden="true" />
                  )}
                  {visibleRecordIndexes.map((index) => {
                    const row = rowAt(index);
                    if (!row)
                      return (
                        <td key="draft-record" className="draft-row-cell">
                          {renderDraftCell(column)}
                        </td>
                      );
                    const result = computed.rows[row.id]?.[column.id];
                    const isFocusCell =
                      gridFocus?.rowId === row.id && gridFocus.columnId === column.id;
                    const inRange = Boolean(
                      selectionRange &&
                        positionInRange({ row: fieldIndex, col: index }, selectionRange)
                    );
                    return (
                      <td
                        key={row.id}
                        data-row-id={row.id}
                        data-row-index={rowOffset + index}
                        onPointerDown={(event) =>
                          onCellPointerDown(event, row, column)
                        }
                        onPointerEnter={(event) =>
                          onCellPointerEnter(event, row, column)
                        }
                        style={frameCellStyleProperties(
                          effectiveFrameCellStyle(
                            frame,
                            row.id,
                            column.id,
                            styleMatches
                          )
                        )}
                        className={`styled-frame-cell ${
                          selection?.rowId === row.id &&
                          selection.columnId === column.id
                            ? "active"
                            : ""
                        } ${result?.error ? "cell-error" : ""} ${
                          isFocusCell ? "cell-focus" : ""
                        } ${inRange ? "cell-range" : ""}`}
                      >
                        <GridCellContent
                          column={column}
                          row={row}
                          computedCell={result}
                          isDerived={isDerived}
                          paged={isPaged}
                          readOnly={readOnly}
                          readOnlyReason={computed.editing.reason}
                          editing={
                            isFocusCell && gridFocus.mode === "edit"
                              ? { seed: gridFocus.editSeed }
                              : null
                          }
                          onNavigate={(event) =>
                            onFocusCell(row, column, "navigate", {
                              extend: event.shiftKey,
                            })
                          }
                          onEdit={() => onFocusCell(row, column, "edit")}
                          onEditFormula={calculatedColumnEditor(
                            computed,
                            column,
                            rowOffset + index,
                            onEditCalculatedColumn
                          )}
                          onCommit={(raw, move) => onCommitCell(row, column, raw, move)}
                          onCancel={() => onSettleCell(row, column)}
                        />
                      </td>
                    );
                  })}
                  {paddingAfter > 0 && (
                    <td className="transposed-spacer" aria-hidden="true" />
                  )}
                  <td className="frame-edge-cell" />
                </tr>
              );
            })}
            {fieldRange.paddingBottom > 0 && (
              <tr className="virtual-spacer" aria-hidden="true">
                <td
                  colSpan={renderedColumnCount}
                  style={{ height: fieldRange.paddingBottom }}
                />
              </tr>
            )}
            {!readOnly && (
              <tr className="transposed-add-field">
                <td colSpan={renderedColumnCount}>
                  <button onClick={() => addColumn(frame.columns.at(-1)?.id ?? null)}>
                    <Plus size={12} /> Add field
                  </button>
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {footer}
    </div>
  );
}

function calculatedColumnEditor(
  computed: ComputedFrame,
  column: Column,
  rowIndex: number,
  onEdit: (column: Column, rowIndex: number) => void
): (() => void) | undefined {
  return isCalculatedFrameColumn(computed, column)
    ? () => onEdit(column, rowIndex)
    : undefined;
}
