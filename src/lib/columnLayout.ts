import type { CSSProperties } from "react";
import type { FrameColumnLayout } from "./bindings/FrameColumnLayout";
import type { Operation } from "./types";
import { TABLE_ROW_HEIGHT } from "./frameVirtualization";

/**
 * Stored column widths and wrapping, and the arithmetic around them.
 *
 * The engine keeps these as display-only state on the frame and replaces a
 * column's whole layout on every write, so everything that sends one has to
 * carry the half it is not changing: a resize must repeat the column's
 * current `wrap`, a wrap toggle its current `width`. That rule lives here,
 * once, in `columnLayoutOperation`, so no caller can forget it.
 */

/** The engine clamps to this range; the interface clamps first so a preview never promises more. */
export const MIN_COLUMN_WIDTH = 24;
export const MAX_COLUMN_WIDTH = 2000;

/** One keyboard nudge on the resize handle, in layout pixels. */
export const COLUMN_WIDTH_KEY_STEP = 8;

/**
 * What a column with no stored width is given once the table switches to
 * fixed layout. A fixed table cannot ask its content how wide to be, so an
 * automatic column beside a sized one takes the same 150px the table has
 * always reserved per column for its minimum width.
 */
export const AUTOMATIC_COLUMN_WIDTH = 150;

/**
 * How many lines a wrapped cell shows. Rows are virtualised on the
 * assumption that every row is the same height (`frameVirtualization`), so a
 * wrapped cell cannot simply grow its row: a row of unknown height would put
 * the scroll position, the spacer rows and reveal-the-active-cell all out by
 * an amount nobody measured. Wrapping instead gives every row of the frame
 * one taller, known height and clips a cell at this many lines.
 */
export const WRAPPED_LINES = 3;
export const WRAPPED_ROW_HEIGHT = 62;

type LayoutFrame = {
  columns: Array<{ id: string }>;
  display?: { columnLayout?: { [columnId: string]: FrameColumnLayout } } | null;
};

export function clampColumnWidth(width: number): number {
  return Math.min(MAX_COLUMN_WIDTH, Math.max(MIN_COLUMN_WIDTH, Math.round(width)));
}

export function storedColumnWidth(frame: LayoutFrame, columnId: string): number | null {
  return frame.display?.columnLayout?.[columnId]?.width ?? null;
}

export function isColumnWrapped(frame: LayoutFrame, columnId: string): boolean {
  return frame.display?.columnLayout?.[columnId]?.wrap === true;
}

/**
 * Whether any column needs the table to stop sizing itself from its content.
 * A resize in flight counts: its preview names every column's width, and
 * previewing them in an auto-layout table would make each only a minimum, so
 * dragging a column narrower would show nothing until release.
 */
export function usesFixedColumns(
  frame: LayoutFrame,
  preview?: Record<string, number> | null
): boolean {
  if (preview && Object.keys(preview).length > 0) return true;
  return frame.columns.some(
    (column) =>
      storedColumnWidth(frame, column.id) !== null || isColumnWrapped(frame, column.id)
  );
}

export function gridRowHeight(frame: LayoutFrame): number {
  return frame.columns.some((column) => isColumnWrapped(frame, column.id))
    ? WRAPPED_ROW_HEIGHT
    : TABLE_ROW_HEIGHT;
}

/**
 * The width each `<col>` is given, or null where the table sizes itself.
 * `preview` is a drag in flight and wins over what is stored.
 */
export function columnWidthFor(
  frame: LayoutFrame,
  columnId: string,
  preview?: Record<string, number> | null
): number | null {
  const width = preview?.[columnId] ?? storedColumnWidth(frame, columnId);
  if (width !== null && width !== undefined) return width;
  return usesFixedColumns(frame, preview) ? AUTOMATIC_COLUMN_WIDTH : null;
}

/** The table's own style: its natural minimum, or the exact sum once columns are fixed. */
export function gridTableStyle(
  frame: LayoutFrame,
  preview?: Record<string, number> | null
): CSSProperties {
  const style = {
    "--grid-row-height": `${gridRowHeight(frame)}px`,
  } as CSSProperties;
  if (!usesFixedColumns(frame, preview))
    return { ...style, minWidth: Math.max(360, frame.columns.length * 150 + 66) };
  const total = frame.columns.reduce(
    (sum, column) => sum + (columnWidthFor(frame, column.id, preview) ?? 0),
    // The row-number gutter and the trailing edge column.
    66
  );
  return { ...style, tableLayout: "fixed", width: total };
}

export type ColumnLayoutChange = {
  columnId: string;
  /** Omit to keep the stored width; `null` returns the column to automatic. */
  width?: number | null;
  /** Omit to keep the stored wrapping. */
  wrap?: boolean;
};

/** One operation, hence one undo entry, for any number of columns. */
export function columnLayoutOperation(
  frame: LayoutFrame & { id: string },
  changes: ColumnLayoutChange[]
): Operation {
  return {
    type: "setFrameDisplayColumnLayout",
    frameId: frame.id,
    columns: changes.map((change) => ({
      columnId: change.columnId,
      width:
        change.width === undefined
          ? storedColumnWidth(frame, change.columnId)
          : change.width === null
            ? null
            : clampColumnWidth(change.width),
      wrap: change.wrap ?? isColumnWrapped(frame, change.columnId),
    })),
  };
}

/**
 * The columns a resize of `columnId` applies to: the whole-column selection
 * when the dragged column is inside it, otherwise just that column. Dragging
 * an edge of an unrelated column must not resize the selection somewhere else.
 */
export function resizeTargets(
  frame: LayoutFrame,
  columnId: string,
  selected: { left: number; right: number } | null
): string[] {
  const index = frame.columns.findIndex((column) => column.id === columnId);
  if (selected && index >= selected.left && index <= selected.right)
    return frame.columns.slice(selected.left, selected.right + 1).map((c) => c.id);
  return [columnId];
}
