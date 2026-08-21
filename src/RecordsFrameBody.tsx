import { Plus } from "lucide-react";
import { GridCellContent } from "./FrameCells";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import {
  effectiveFrameCellStyle,
  frameCellStyleProperties,
  isCalculatedFrameColumn,
  isEntryFrameColumn,
} from "./FrameGrid";
import { positionInRange } from "./lib/gridNavigation";

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
  };
}

export function RecordsFrameBody({ model }: { model: RecordsAsRowsFrameCardProps }) {
  const {
    frame,
    computed,
    selection,
    gridFocus: gridFocusHere,
    visibleRows, virtualRange, selectionRange,
    isDerived,
    isFileBacked,
    isTransposed,
    isReadOnly,
    canAddColumns,
    placeholderOffsets,
    styleMatches,
    selectWholeRow,
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
                  <td
                    className="row-number styled-frame-cell selectable-header"
                    title={`Select row ${rowIndex + 1}`}
                    onPointerDown={(event) => selectWholeRow(event, row)}
                    style={frameCellStyleProperties(
                      // No column: the gutter takes a rule that styles the
                      // whole row and nothing narrower.
                      effectiveFrameCellStyle(frame, row.id, undefined, styleMatches)
                    )}
                  >
                    {isPlaceholderRow ? "" : rowIndex + 1}
                  </td>
                  {frame.columns.map((column, columnIndex) => {
                    const result = computed.rows[row.id]?.[column.id];
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
                        {isPlaceholderRow ? (
                          <span className="cell-skeleton" aria-hidden="true" />
                        ) : (
                          <GridCellContent
                            column={column}
                            row={row}
                            computedCell={result}
                            isDerived={isDerived && !isEntryFrameColumn(frame, column)}
                            paged={isFileBacked && !isEntryFrameColumn(frame, column)}
                            readOnly={isReadOnly && !isEntryFrameColumn(frame, column)}
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
            <DraftFrameRow model={model} />
          </tbody>
  );
}

function DraftFrameRow({
  model,
}: {
  model: RecordsAsRowsFrameCardProps;
}) {
  const {
    frame,
    computed,
    canAddRows,
    draftRow,
    setDraftRow,
    commitDraftRow,
    addColumn,
  } = model;
  return canAddRows ? (
              <tr
                className="draft-row"
                onBlur={(event) => {
                  if (!event.currentTarget.contains(event.relatedTarget as Node | null))
                    commitDraftRow();
                }}
              >
                <td className="row-number">
                  <button title="Add empty row" onClick={() => commitDraftRow(true)}>
                    <Plus size={12} />
                  </button>
                </td>
                {frame.columns.map((column) => (
                  <td key={column.id} data-column-id={column.id}>
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
