import { Plus } from "lucide-react";
import type { CSSProperties } from "react";
import { FillHandle } from "./FillHandle";
import { GridCellContent } from "./FrameCells";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import {
  effectiveFrameCellStyle,
  frameCellStyleProperties,
  isCalculatedFrameColumn,
  isEditableGridColumn,
  isEntryFrameColumn,
} from "./FrameGrid";
import { positionInRange } from "./lib/gridNavigation";
import { pinnedCellClass, pinnedCellStyle } from "./lib/pinnedColumns";

function cellEditHandlers(
  model: RecordsAsRowsFrameCardProps,
  row: RecordsAsRowsFrameCardProps["displayedRows"][number],
  column: RecordsAsRowsFrameCardProps["frame"]["columns"][number],
  rowIndex: number
) {
  return {
    onEdit: () => model.focusCell(row, column, "edit"),
    onEditFormula: isCalculatedFrameColumn(model.computed, column)
      ? () => model.editCalculatedColumn(column, rowIndex)
      : undefined,
    // A leading `=` in the value editor: the editor settles and the
    // column's formula opens instead, the same door as from navigation.
    onFormulaKey: () => {
      model.settleCellEdit(row, column);
      model.onTransformColumn(model.frame, column, "");
    },
  };
}

/** Every state a body cell can be in, gathered out of the render loop. */
function bodyCellClasses(
  model: RecordsAsRowsFrameCardProps,
  row: RecordsAsRowsFrameCardProps["displayedRows"][number],
  column: RecordsAsRowsFrameCardProps["frame"]["columns"][number],
  rowIndex: number,
  state: { error: boolean; focus: boolean; inRange: boolean; pinned: string }
): string {
  const active =
    model.selection?.rowId === row.id && model.selection.columnId === column.id;
  return [
    "styled-frame-cell",
    active ? "active" : "",
    state.error ? "cell-error" : "",
    state.focus ? "cell-focus" : "",
    state.inRange ? "cell-range" : "",
    state.pinned,
    model.fillHandle.cellClass(column.id, rowIndex),
  ]
    .filter(Boolean)
    .join(" ");
}

/**
 * A body cell's own styling and, where the column is frozen, the offset it
 * sticks to. Gathered here so the render loop stays a loop.
 */
function bodyCellStyle(
  model: RecordsAsRowsFrameCardProps,
  row: RecordsAsRowsFrameCardProps["displayedRows"][number],
  column: RecordsAsRowsFrameCardProps["frame"]["columns"][number],
  columnIndex: number,
  pinned: number[]
): CSSProperties {
  return {
    ...frameCellStyleProperties(
      effectiveFrameCellStyle(model.frame, row.id, column.id, model.styleMatches)
    ),
    ...pinnedCellStyle(columnIndex + 1, pinned),
  };
}

/** The row's index in the gutter, which freezes with the frozen columns. */
function RowNumberCell({
  model,
  row,
  rowIndex,
  pinned,
  blank,
}: {
  model: RecordsAsRowsFrameCardProps;
  row: RecordsAsRowsFrameCardProps["displayedRows"][number];
  rowIndex: number;
  pinned: number[];
  blank: boolean;
}) {
  return (
    <td
      className={`row-number styled-frame-cell selectable-header ${pinnedCellClass(0, pinned)}`}
      title={`Select row ${rowIndex + 1}`}
      onPointerDown={(event) => model.selectWholeRow(event, row)}
      style={{
        // No column: the gutter takes a rule that styles the whole row and
        // nothing narrower.
        ...frameCellStyleProperties(
          effectiveFrameCellStyle(model.frame, row.id, undefined, model.styleMatches)
        ),
        ...pinnedCellStyle(0, pinned),
      }}
    >
      {blank ? "" : rowIndex + 1}
    </td>
  );
}

export function RecordsFrameBody({
  model,
  pinned,
}: {
  model: RecordsAsRowsFrameCardProps;
  /** Sticky offsets for the frozen gutter and leading columns. */
  pinned: number[];
}) {
  const {
    frame,
    computed,
    gridFocus: gridFocusHere,
    visibleRows, virtualRange, selectionRange,
    isDerived,
    isFileBacked,
    isTransposed,
    isReadOnly,
    canAddColumns,
    placeholderOffsets,
    beginCellSelection,
    extendCellSelection,
    focusCell,
    commitCellEdit,
    settleCellEdit,
    addColumn,
  } = model;
  return (
          <tbody>
            {virtualRange.paddingTop > 0 && (
              <tr className="virtual-spacer" aria-hidden="true">
                <td
                  colSpan={frame.columns.length + 2}
                  style={{ height: virtualRange.paddingTop }}
                />
              </tr>
            )}
            {visibleRows.map((row, visibleIndex) => {
              const rowIndex = virtualRange.start + visibleIndex;
              const isPlaceholderRow = Boolean(
                isFileBacked &&
                  !isTransposed &&
                  placeholderOffsets.has(visibleIndex)
              );
              return (
                <tr
                  key={row.id}
                  data-row-id={row.id}
                  data-row-index={rowIndex}
                  aria-rowindex={rowIndex + 3}
                  className={isPlaceholderRow ? "row-loading-skeleton" : ""}
                >
                  <RowNumberCell
                    model={model}
                    row={row}
                    rowIndex={rowIndex}
                    pinned={pinned}
                    blank={isPlaceholderRow}
                  />
                  {frame.columns.map((column, columnIndex) => {
                    const result = computed.rows[row.id]?.[column.id];
                    const calculated = isCalculatedFrameColumn(computed, column);
                    const isFocusCell =
                      gridFocusHere?.rowId === row.id &&
                      gridFocusHere.columnId === column.id;
                    const inRange = Boolean(
                      selectionRange &&
                        positionInRange(
                          { row: rowIndex, col: columnIndex },
                          selectionRange
                        )
                    );
                    return (
                      <td
                        key={column.id}
                        data-column-id={column.id}
                        onPointerDown={(event) =>
                          beginCellSelection(event, row, column)
                        }
                        onPointerEnter={(event) =>
                          extendCellSelection(event, row, column)
                        }
                        style={bodyCellStyle(model, row, column, columnIndex, pinned)}
                        className={bodyCellClasses(model, row, column, rowIndex, {
                          error: Boolean(result?.error),
                          focus: isFocusCell,
                          inRange,
                          pinned: pinnedCellClass(columnIndex + 1, pinned),
                        })}
                      >
                        {isPlaceholderRow ? (
                          <span className="cell-skeleton" aria-hidden="true" />
                        ) : (
                          <GridCellContent
                            column={column}
                            row={row}
                            // A page already carries evaluated display text.
                            // `computed.rows` is the non-paged answer cache
                            // and may describe the pre-page probe instead.
                            computedCell={isFileBacked ? undefined : result}
                            isDerived={
                              (isDerived || calculated) &&
                              !isEntryFrameColumn(frame, column)
                            }
                            // `paged` describes how rows reached React, not
                            // whether the person owns them. A literal frame
                            // with a row-preserving Wrangle chain is paged too;
                            // its input cells still take edits, while its
                            // calculated columns use the computed-cell path.
                            paged={
                              isFileBacked &&
                              isReadOnly &&
                              !isEntryFrameColumn(frame, column)
                            }
                            readOnly={isReadOnly && !isEntryFrameColumn(frame, column)}
                            // The per-column answer, from the same rule the
                            // engine enforces: the frame-level flags above
                            // choose how the value is drawn, this one alone
                            // decides whether an editor may open.
                            editable={isEditableGridColumn(computed, column, frame)}
                            readOnlyReason={computed.editing.reason}
                            editing={
                              isFocusCell && gridFocusHere.mode === "edit"
                                ? { seed: gridFocusHere.editSeed }
                                : null
                            }
                            onNavigate={(event) =>
                              focusCell(row, column, "navigate", {
                                extend: event.shiftKey,
                              })
                            }
                            {...cellEditHandlers(model, row, column, rowIndex)}
                            onCommit={(raw, move) =>
                              commitCellEdit(row, column, raw, move)
                            }
                            onCancel={() => settleCellEdit(row, column)}
                          />
                        )}
                        <FillHandle model={model} column={column} rowIndex={rowIndex} />
                      </td>
                    );
                  })}
                  <td
                    className="frame-edge-cell"
                    onClick={() => {
                      if (canAddColumns) addColumn(frame.columns.at(-1)?.id ?? null);
                    }}
                  />
                </tr>
              );
            })}
            {virtualRange.paddingBottom > 0 && (
              <tr className="virtual-spacer" aria-hidden="true">
                <td
                  colSpan={frame.columns.length + 2}
                  style={{ height: virtualRange.paddingBottom }}
                />
              </tr>
            )}
            <DraftFrameRow model={model} pinned={pinned} />
          </tbody>
  );
}

function DraftFrameRow({
  model,
  pinned,
}: {
  model: RecordsAsRowsFrameCardProps;
  pinned: number[];
}) {
  const {
    frame,
    computed,
    canAddRows,
    draftRow,
    setDraftRow,
    commitDraftRow,
    addColumn,
    onTransformColumn,
  } = model;
  return canAddRows ? (
              <tr
                className="draft-row"
                onBlur={(event) => {
                  if (!event.currentTarget.contains(event.relatedTarget as Node | null))
                    commitDraftRow();
                }}
              >
                <td
                  className={`row-number ${pinnedCellClass(0, pinned)}`}
                  style={pinnedCellStyle(0, pinned)}
                >
                  <button title="Add empty row" onClick={() => commitDraftRow(true)}>
                    <Plus size={12} />
                  </button>
                </td>
                {frame.columns.map((column, columnIndex) => (
                  <td
                    key={column.id}
                    data-column-id={column.id}
                    className={pinnedCellClass(columnIndex + 1, pinned)}
                    style={pinnedCellStyle(columnIndex + 1, pinned)}
                  >
                    {isCalculatedFrameColumn(computed, column) ? (
                      <span className="draft-formula">ƒ</span>
                    ) : column.dataType === "categorical" ? (
                      <select
                        className="categorical-cell"
                        aria-label={`New row ${column.name}`}
                        value={draftRow[column.id] ?? ""}
                        onChange={(event) =>
                          setDraftRow((current) => ({
                            ...current,
                            [column.id]: event.target.value,
                          }))
                        }
                      >
                        <option value="">—</option>
                        {(column.categories ?? []).map((category) => (
                          <option key={category} value={category}>
                            {category}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        aria-label={`New row ${column.name}`}
                        value={draftRow[column.id] ?? ""}
                        placeholder={
                          column.dataType === "date"
                            ? "YYYY-MM-DD"
                            : column.dataType === "boolean"
                            ? "true / false"
                            : ""
                        }
                        onChange={(event) =>
                          setDraftRow((current) => ({
                            ...current,
                            [column.id]: event.target.value,
                          }))
                        }
                        onKeyDown={(event) => {
                          if (event.key === "Enter") commitDraftRow();
                          // The same doorway as in the grid: a leading `=`
                          // opens the column's formula, not a value.
                          else if (event.key === "=" && !event.currentTarget.value) {
                            event.preventDefault();
                            onTransformColumn(frame, column, "");
                          }
                        }}
                      />
                    )}
                  </td>
                ))}
                <td
                  className="frame-edge-cell"
                  onClick={() => addColumn(frame.columns.at(-1)?.id ?? null)}
                />
              </tr>
  ) : null;
}
