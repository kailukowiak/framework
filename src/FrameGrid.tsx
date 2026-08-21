import { createContext, type CSSProperties, type PointerEvent } from "react";
import type { FramePage } from "./lib/api";
import {
  normalizeRange,
  type GridBounds,
  type GridDirection,
  type GridPosition,
  type GridRange,
} from "./lib/gridNavigation";
import { expandRangeForSpan } from "./lib/gridSpan";
import { themedColor } from "./lib/palette";
import { parseUseThousandsSeparators } from "./lib/preferences";
import type {
  CanvasView,
  Column,
  ComputedFrame,
  DocumentView,
  RenderedFrameStep,
  Row,
  Selection,
  SortKey,
  TabObject,
  FrameCellStyle,
  FrameObject,
  FrameOrientation,
  FrameStyle,
  FrameStyleMatch,
  FrameStyleRule,
  FrameStyleTarget,
} from "./lib/types";

/**
 * What the conditional-formatting rules made of each row, by row id, in the
 * frame's own rule order.
 *
 * The core answers this, because a rule is a formula and only the core runs
 * formulas: it evaluates every rule as a hidden column over the rows it is
 * about to hand back and reads each answer as style. An in-memory frame
 * carries the result on its computed view, a paged frame on each page, which
 * is why both are reduced to this one shape here.
 */
export type FrameStyleMatches = Record<string, FrameStyleMatch[]>;

export const emptyFrameCellStyle = (): FrameCellStyle => ({
  bold: null,
  italic: null,
  underline: null,
  textColor: null,
  fillColor: null,
  alignment: null,
  lineStyle: null,
});

export function styleTargetForSelection(selection: Selection): FrameStyleTarget {
  if (selection.rowId && selection.columnId)
    return { kind: "cell", rowId: selection.rowId, columnId: selection.columnId };
  if (selection.rowId) return { kind: "row", rowId: selection.rowId };
  if (selection.columnId) return { kind: "column", columnId: selection.columnId };
  return { kind: "frame" };
}

export function sameStyleTarget(left: FrameStyleTarget, right: FrameStyleTarget): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function exactFrameCellStyle(
  frame: FrameObject,
  target: FrameStyleTarget
): FrameCellStyle {
  return (
    frameStyles(frame).find((entry) => sameStyleTarget(entry.target, target))?.style ??
    emptyFrameCellStyle()
  );
}

export function mergedFrameCellStyle(
  base: FrameCellStyle,
  next: FrameCellStyle
): FrameCellStyle {
  return Object.fromEntries(
    Object.entries(base).map(([key, value]) => [
      key,
      next[key as keyof FrameCellStyle] ?? value,
    ])
  ) as unknown as FrameCellStyle;
}

export function effectiveFrameCellStyle(
  frame: FrameObject,
  rowId?: string,
  columnId?: string,
  matches?: FrameStyleMatches
): FrameCellStyle {
  let style = exactFrameCellStyle(frame, { kind: "frame" });
  if (columnId)
    style = mergedFrameCellStyle(
      style,
      exactFrameCellStyle(frame, { kind: "column", columnId })
    );
  if (rowId)
    style = mergedFrameCellStyle(
      style,
      exactFrameCellStyle(frame, { kind: "row", rowId })
    );
  if (rowId && columnId)
    style = mergedFrameCellStyle(
      style,
      exactFrameCellStyle(frame, { kind: "cell", rowId, columnId })
    );
  // Rules are the layer above every direct format, never a replacement for
  // one: each contributes only the properties it sets, so a rule that paints
  // a row red leaves the bold somebody typed onto one cell of it alone.
  for (const match of matchedFrameStyles(frame, rowId, columnId, matches))
    style = mergedFrameCellStyle(style, match.style);
  return style;
}

/**
 * The rule answers that reach this cell, in the order they are applied —
 * frame order, so a later rule overrides an earlier one property by
 * property.
 *
 * A rule with no column of its own is the whole row's, the row number
 * gutter (`columnId` absent) included. A rule confined to a column still
 * read the whole row to reach its answer; all that is decided here is where
 * that answer is allowed to show.
 */
export function matchedFrameStyles(
  frame: FrameObject,
  rowId?: string,
  columnId?: string,
  matches?: FrameStyleMatches
): FrameStyleMatch[] {
  const matched = rowId ? matches?.[rowId] : undefined;
  if (!matched?.length) return [];
  const scopes = new Map(
    frameStyleRules(frame).map((rule) => [rule.id, rule.columnId ?? null])
  );
  return matched.filter((match) => {
    const scope = scopes.get(match.ruleId);
    // A rule the frame no longer holds is an answer to a question nobody is
    // asking any more: the page it came on outlived the rule being deleted.
    return scope !== undefined && (scope === null || scope === columnId);
  });
}

/**
 * A stored style as CSS.
 *
 * Colors go out as `light-dark()` pairs rather than as the hex the document
 * holds: a fill chosen to be a quiet highlight against light paper is a
 * bright blob against dark paper, and text chosen to be readable on one is
 * invisible on the other. The reflection that produces the second half is in
 * lib/palette, and it is left to CSS to choose between them so the window
 * follows the system theme when it changes rather than when React last
 * rendered.
 */
export function frameCellStyleProperties(style: FrameCellStyle): CSSProperties {
  return {
    backgroundColor: themedColor(style.fillColor),
    color: themedColor(style.textColor),
    fontWeight: style.bold === null ? undefined : style.bold ? 700 : 400,
    fontStyle: style.italic === null ? undefined : style.italic ? "italic" : "normal",
    textDecoration:
      style.underline === null ? undefined : style.underline ? "underline" : "none",
    textAlign: style.alignment ?? undefined,
    borderBottomStyle: style.lineStyle ?? undefined,
    borderBottomWidth: style.lineStyle
      ? style.lineStyle === "double"
        ? 3
        : 2
      : undefined,
    borderBottomColor: style.lineStyle
      ? themedColor(style.textColor) ?? "light-dark(#8a8d85, #6c6f68)"
      : undefined,
    "--cell-fill": themedColor(style.fillColor),
    "--cell-justify":
      style.alignment === "center"
        ? "center"
        : style.alignment === "right"
        ? "flex-end"
        : "flex-start",
  } as CSSProperties;
}

export type SetFrameCachedHandler = (
  frameId: string,
  cached: boolean,
  options?: { inlineError?: boolean }
) => Promise<string | null>;
export type RefreshConnectorHandler = (
  frameId: string,
  options?: { inlineError?: boolean }
) => Promise<string | null>;
/** Opens a picker and repoints the frame; resolves to an error, or null. */
export type SetFrameSourceHandler = (frameId: string) => Promise<string | null>;
/** Makes a frame's values the document's own; resolves to an error, or null. */
export type TakeOwnershipHandler = (
  frameId: string,
  options?: { inlineError?: boolean }
) => Promise<string | null>;
/** Adds a frozen copy beside the frame; resolves to an error, or null. */
export type FreezeCopyHandler = (
  frameId: string,
  position: { x: number; y: number }
) => Promise<string | null>;
/** Pointer press or drag-over on a grid cell, for range selection. */
export type CellPointerHandler = (
  event: React.PointerEvent,
  row: Row,
  column: Column
) => void;
export type ContextMenuState = {
  screenX: number;
  screenY: number;
  canvasX: number;
  canvasY: number;
  frameId?: string;
  columnId?: string;
  rowId?: string;
  rowIndex?: number;
  objectId?: string;
  viewId?: string;
};

export type GridFocusMode = "navigate" | "edit";
export type GridFocus = {
  viewId: string;
  objectId: string;
  rowId: string;
  columnId: string;
  mode: GridFocusMode;
  /** Initial editor content for type-to-replace; null preserves the cell content. */
  editSeed: string | null;
  /** Range-selection anchor left behind by Shift+Arrow, or null for a single cell. */
  anchor: { rowId: string; columnId: string } | null;
  /**
   * A selection that covers a whole axis of the frame, however much of it
   * happens to be loaded — clicking a column header, or a row's index.
   *
   * It cannot be expressed with `anchor` alone: an imported frame holds
   * only the scrolled-to window on the client, so "every row of this
   * column" has no far corner to point at. `"column"` means the anchored
   * columns across every row; `"row"` means the anchored rows across every
   * column.
   */
  span: "row" | "column" | null;
};

/**
 * What a card has on screen, published up so the window-level keyboard and
 * clipboard handlers can see it.
 *
 * An imported frame keeps no rows in the document — they are read out of
 * the artifact a page at a time — so for one of those this is the only
 * place its rows exist on the client, and it holds only the scrolled-to
 * window. `offset` and `totalRows` are what let a selection describe rows
 * that are not loaded at all.
 */
export type RenderedGrid = {
  rows: Row[];
  /** Logical index of `rows[0]`; nonzero only on a scrolled paged frame. */
  offset: number;
  /** Every row the frame has, loaded or not. */
  totalRows: number;
};

export type GridContext = {
  frame: FrameObject;
  computed?: ComputedFrame;
  displayedRows: Row[];
  /** Logical index of `displayedRows[0]`. */
  rowOffset: number;
  /** Every row the frame has, including any not loaded. */
  totalRows: number;
  orientation: FrameOrientation;
  viewportRows: number;
};

export const ARROW_DIRECTIONS: Record<string, GridDirection> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

/**
 * The objects a card offers as tabs, in strip order.
 *
 * Frames and plots both qualify: a plot of a frame on the card reads the
 * same data through the same lineage, so it is another way of looking at
 * what the card already holds rather than a separate subject.
 */
export function tabObjects(view: CanvasView, document: DocumentView): TabObject[] {
  const ids = view.tabObjectIds?.length ? view.tabObjectIds : [view.objectId];
  return ids.flatMap((id) => {
    const object = document.objects.find((candidate) => candidate.id === id);
    return object && (object.kind === "frame" || object.kind === "plot")
      ? [object]
      : [];
  });
}

/** Just the frames among a card's tabs — what a frame-shaped edit can target. */
export function tabFrames(view: CanvasView, document: DocumentView): FrameObject[] {
  return tabObjects(view, document).filter(
    (object): object is FrameObject => object.kind === "frame"
  );
}

/**
 * The object the card is currently showing. `objectId` *is* the selected
 * tab, so there is no second piece of state that can disagree with it.
 */
export function activeTabObject(
  view: CanvasView,
  document: DocumentView
): TabObject | undefined {
  const objects = tabObjects(view, document);
  return objects.find((object) => object.id === view.objectId) ?? objects[0];
}

/**
 * The frame a card's edits apply to: the selected tab when it is a frame,
 * and otherwise the frame the selected plot draws. A plot tab is still
 * *about* a frame, so the inspector has something to talk about either way.
 */
export function activeTabFrame(
  view: CanvasView,
  document: DocumentView
): FrameObject | undefined {
  const active = activeTabObject(view, document);
  if (active?.kind === "frame") return active;
  if (active?.kind === "plot") {
    const source = document.objects.find(
      (candidate): candidate is FrameObject =>
        candidate.kind === "frame" && candidate.id === active.sourceFrameId
    );
    if (source) return source;
  }
  return tabFrames(view, document)[0];
}

export function frameOrientation(frame: FrameObject): FrameOrientation {
  return frame.display?.orientation ?? "recordsAsRows";
}

export function frameStyles(frame: FrameObject): FrameStyle[] {
  return frame.display?.styles ?? [];
}

export function frameStyleRules(frame: FrameObject): FrameStyleRule[] {
  return frame.display?.styleRules ?? [];
}

export function pipelineSortKeys(computed?: ComputedFrame): SortKey[] {
  const trailing = computed?.steps?.at(-1);
  return trailing?.kind === "sort" ? trailing.keys : [];
}

/**
 * The wrangle chain a card is showing, rendered back to text.
 *
 * `computed.steps` is one field for both kinds of frame — a derived frame's
 * chain and a source frame's own — so anything reading it has to say which
 * it meant. The two readers below both mean a derived frame's chain, and
 * check `derivation` to say so.
 */
export function chainSteps(computed?: ComputedFrame): RenderedFrameStep[] {
  return computed?.steps ?? [];
}

/** How many predicates the chain narrows by, across every filter step in it. */
export function chainFilterCount(computed?: ComputedFrame): number {
  return chainSteps(computed).reduce(
    (total, step) => (step.kind === "filter" ? total + step.predicates.length : total),
    0
  );
}

/**
 * The identity of one row on a page. Positional when the page brought none,
 * which is all a row streamed out of a parquet scan ever had.
 */
function framePageRowId(frame: FrameObject, page: FramePage, pageIndex: number): string {
  return page.rowIds[pageIndex] ?? `source:${frame.id}:${page.offset + pageIndex}`;
}

export function rowsFromFramePage(frame: FrameObject, page: FramePage): Row[] {
  return page.rows.map((values, pageIndex) => ({
    id: framePageRowId(frame, page, pageIndex),
    cells: Object.fromEntries(
      frame.columns.map((column, columnIndex) => [
        column.id,
        { raw: values[columnIndex] ?? "", overrideFormula: null },
      ])
    ),
  }));
}

/** The page's rule answers, keyed the same way its rows are. */
export function styleMatchesFromFramePage(
  frame: FrameObject,
  page: FramePage
): FrameStyleMatches {
  const matches: FrameStyleMatches = {};
  page.styleMatches?.forEach((matched, pageIndex) => {
    if (matched.length > 0) matches[framePageRowId(frame, page, pageIndex)] = matched;
  });
  return matches;
}

/**
 * What the keyboard and clipboard layer is looking at.
 *
 * `rendered` is the rows a card currently has on screen, published by
 * `FrameCard`. An imported frame keeps no rows in the document at all —
 * they are read out of the artifact a page at a time — so `frame.rows` is
 * empty for one, and reading it here is what used to leave copy and arrow
 * navigation with a zero-row grid on exactly the frames large enough to
 * need them.
 */
export function resolveGridContext(
  document: DocumentView,
  focus: GridFocus,
  rendered: Map<string, RenderedGrid>
): GridContext | null {
  const view = document.views.find((candidate) => candidate.id === focus.viewId);
  const frame = document.objects.find(
    (candidate): candidate is FrameObject =>
      candidate.kind === "frame" && candidate.id === focus.objectId
  );
  if (!view || !frame || !tabFrames(view, document).some((tab) => tab.id === frame.id))
    return null;
  const grid = rendered.get(frame.id);
  return {
    frame,
    computed: document.computedFrames[frame.id],
    displayedRows: grid?.rows ?? frame.rows,
    rowOffset: grid?.offset ?? 0,
    totalRows: grid?.totalRows ?? frame.rows.length,
    orientation: frameOrientation(frame),
    viewportRows: Math.max(1, Math.floor((view.height - 184) / 34)),
  };
}

export function gridBoundsFor(context: GridContext): GridBounds {
  return context.orientation === "fieldsAsRows"
    ? {
        rowCount: context.frame.columns.length,
        columnCount: context.displayedRows.length,
      }
    : {
        rowCount: context.displayedRows.length,
        columnCount: context.frame.columns.length,
      };
}

export function visualGridPosition(
  context: GridContext,
  rowId: string,
  columnId: string
): GridPosition | null {
  const rowIndex = context.displayedRows.findIndex((row) => row.id === rowId);
  const columnIndex = context.frame.columns.findIndex(
    (column) => column.id === columnId
  );
  if (rowIndex < 0 || columnIndex < 0) return null;
  return context.orientation === "fieldsAsRows"
    ? { row: columnIndex, col: rowIndex }
    : { row: rowIndex, col: columnIndex };
}

export function gridCellAt(
  context: GridContext,
  position: GridPosition
): { row: Row; column: Column } | null {
  const rowIndex = context.orientation === "fieldsAsRows" ? position.col : position.row;
  const columnIndex =
    context.orientation === "fieldsAsRows" ? position.row : position.col;
  const row = context.displayedRows[rowIndex];
  const column = context.frame.columns[columnIndex];
  return row && column ? { row, column } : null;
}

export function isVisualCellEmpty(context: GridContext, position: GridPosition): boolean {
  const target = gridCellAt(context, position);
  if (!target) return true;
  const computedCell = context.computed?.rows[target.row.id]?.[target.column.id];
  if (computedCell) return computedCell.typedValue.type === "null";
  return (target.row.cells[target.column.id]?.raw ?? "").trim().length === 0;
}

/**
 * Whether a value can be typed into this column's cells.
 *
 * The frame half of the answer comes from the backend, which is where the
 * rule is enforced — asking the model here produced a second copy of it
 * that could drift. The column half stays local because it is not a rule
 * about the frame: a calculated column holds a formula wherever it lives.
 */
export function isEditableGridColumn(
  computed: ComputedFrame | undefined,
  column: Column,
  frame?: FrameObject
): boolean {
  return (
    (Boolean(computed?.editing.cells) || isEntryFrameColumn(frame, column)) &&
    !isCalculatedFrameColumn(computed, column)
  );
}

/**
 * An entry column takes typing even on a computed frame: the value is
 * stored against the row's key, not written into the computed rows. Only
 * single-cell edits go through it, so the bulk paths (fill, paste, clear)
 * deliberately keep the two-argument call and stay out.
 */
export function isEntryFrameColumn(
  frame: FrameObject | undefined,
  column: Column
): boolean {
  return Boolean(
    frame?.entryColumns?.some((entry) => entry.columnId === column.id)
  );
}

/** Legacy declarations and Wrangle outputs are the same grid affordance. */
export function isCalculatedFrameColumn(
  computed: ComputedFrame | undefined,
  column: Column
): boolean {
  return Boolean(column.formula || computed?.formulas[column.id] !== undefined);
}

export function gridRangeForFocus(context: GridContext, focus: GridFocus): GridRange | null {
  const active = visualGridPosition(context, focus.rowId, focus.columnId);
  if (!active) return null;
  const anchor = focus.anchor
    ? visualGridPosition(context, focus.anchor.rowId, focus.anchor.columnId)
    : null;
  return expandRangeForSpan(
    normalizeRange(anchor ?? active, active),
    focus.span,
    gridBoundsFor(context),
    context.orientation === "fieldsAsRows"
  );
}

export function rawGridValue(context: GridContext, position: GridPosition): string {
  const target = gridCellAt(context, position);
  if (!target) return "";
  if (!target.column.formula && !context.frame.derivation)
    return target.row.cells[target.column.id]?.raw ?? "";
  const value = context.computed?.rows[target.row.id]?.[target.column.id]?.typedValue;
  if (!value || value.type === "null") return "";
  if (value.type === "number") return String(value.value);
  if (value.type === "boolean") return value.value ? "true" : "false";
  return value.value;
}

/**
 * The selection in frame terms — logical rows and real columns — rather
 * than in screen terms.
 *
 * A whole-column selection on an imported frame names rows that are not
 * loaded and, in the transposed orientation, runs across the screen instead
 * of down it. Copy cares about none of that: it wants "these columns, these
 * row numbers", which is also what `get_frame_page` takes.
 */
export function selectedFrameRegion(
  context: GridContext,
  focus: GridFocus
): { columns: Column[]; firstRow: number; rowCount: number } | null {
  const range = gridRangeForFocus(context, focus);
  if (!range) return null;
  const transposed = context.orientation === "fieldsAsRows";
  const columnSlice = transposed
    ? { from: range.top, to: range.bottom }
    : { from: range.left, to: range.right };
  const rowSlice = transposed
    ? { from: range.left, to: range.right }
    : { from: range.top, to: range.bottom };
  const columns = context.frame.columns.slice(columnSlice.from, columnSlice.to + 1);
  if (!columns.length) return null;
  // A span reaches past the loaded window, so it is resolved against the
  // frame's real row count instead of the rendered rows.
  if (focus.span === "column") {
    return { columns, firstRow: 0, rowCount: context.totalRows };
  }
  return {
    columns,
    firstRow: context.rowOffset + rowSlice.from,
    rowCount: Math.max(0, rowSlice.to - rowSlice.from + 1),
  };
}

/** Rows the card already holds, so copying them needs no round trip. */
export function loadedRegionRows(
  context: GridContext,
  region: { firstRow: number; rowCount: number }
): Row[] | null {
  const start = region.firstRow - context.rowOffset;
  if (start < 0 || start + region.rowCount > context.displayedRows.length) return null;
  return context.displayedRows.slice(start, start + region.rowCount);
}

/** One row's cells as text, in the order the columns were selected. */
export function rowClipboardValues(
  context: GridContext,
  row: Row,
  columns: Column[]
): string[] {
  return columns.map((column) => {
    if (!column.formula && !context.frame.derivation)
      return row.cells[column.id]?.raw ?? "";
    const value = context.computed?.rows[row.id]?.[column.id]?.typedValue;
    if (!value || value.type === "null") return "";
    if (value.type === "number") return String(value.value);
    if (value.type === "boolean") return value.value ? "true" : "false";
    return value.value;
  });
}

/**
 * Past this many cells, copying is worth asking about rather than simply
 * doing: a whole column of a million-row ledger is one click away, and both
 * the fetch and the resulting clipboard string are large enough to be felt.
 */
export const COPY_CONFIRM_CELLS = 200_000;

/**
 * Whether Ctrl+C includes a header row, remembered across sessions.
 *
 * A preference rather than document state: which of the two you want
 * depends on where you are pasting — a spreadsheet wants the names, a
 * formula argument does not — and that is a property of how someone works,
 * not of the document. The context menu offers both regardless.
 */
export const COPY_HEADERS_PREFERENCE = "framework.copyIncludesHeaders";
export const THOUSANDS_SEPARATORS_PREFERENCE = "framework.useThousandsSeparators";

export const NumberDisplayContext = createContext(true);

export function readCopyHeadersPreference(): boolean {
  try {
    return window.localStorage.getItem(COPY_HEADERS_PREFERENCE) === "true";
  } catch {
    return false;
  }
}

export function readThousandsSeparatorsPreference(): boolean {
  try {
    return parseUseThousandsSeparators(
      window.localStorage.getItem(THOUSANDS_SEPARATORS_PREFERENCE)
    );
  } catch {
    return true;
  }
}

export function isTextEntryTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.isContentEditable
  );
}

/**
 * How heavily a card's funnel should be drawn, and what it means.
 *
 * A pipeline filter changes the data read downstream, so it receives the
 * structural treatment. An unfiltered frame stays visually quiet.
 */
export function filterWeight(computed?: ComputedFrame): {
  weight: "unfiltered" | "structural";
  count: number;
  reading: string;
} {
  const structural = chainFilterCount(computed);
  return {
    weight: structural ? "structural" : "unfiltered",
    count: structural,
    reading: structural
      ? `${structural} filter${structural === 1 ? "" : "s"} in the pipeline`
      : "Not filtered",
  };
}

export function nextColumnName(frame: FrameObject): string {
  const names = new Set(frame.columns.map((column) => column.name.toLocaleLowerCase()));
  if (!names.has("new column")) return "New column";
  let suffix = 2;
  while (names.has(`new column ${suffix}`)) suffix += 1;
  return `New column ${suffix}`;
}
