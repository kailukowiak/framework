import {
  MAX_VIRTUAL_SCROLL_HEIGHT,
  TABLE_HEADER_HEIGHT,
  TABLE_ROW_HEIGHT,
} from "./frameVirtualization";

export interface GridPosition {
  row: number;
  col: number;
}

export interface GridBounds {
  rowCount: number;
  columnCount: number;
}

export type GridDirection = "up" | "down" | "left" | "right";

export interface GridRange {
  top: number;
  left: number;
  bottom: number;
  right: number;
}

const DIRECTION_DELTAS: Record<GridDirection, GridPosition> = {
  up: { row: -1, col: 0 },
  down: { row: 1, col: 0 },
  left: { row: 0, col: -1 },
  right: { row: 0, col: 1 },
};

export function clampPosition(
  position: GridPosition,
  bounds: GridBounds
): GridPosition {
  return {
    row: Math.min(Math.max(position.row, 0), Math.max(0, bounds.rowCount - 1)),
    col: Math.min(Math.max(position.col, 0), Math.max(0, bounds.columnCount - 1)),
  };
}

function samePosition(left: GridPosition, right: GridPosition): boolean {
  return left.row === right.row && left.col === right.col;
}

/** One arrow-key step, clamped at the frame bounds. */
export function stepPosition(
  position: GridPosition,
  direction: GridDirection,
  bounds: GridBounds
): GridPosition {
  const delta = DIRECTION_DELTAS[direction];
  return clampPosition(
    { row: position.row + delta.row, col: position.col + delta.col },
    bounds
  );
}

/** Tab moves horizontally and wraps across rows like a spreadsheet. */
export function tabPosition(
  position: GridPosition,
  backwards: boolean,
  bounds: GridBounds
): GridPosition {
  const current = clampPosition(position, bounds);
  if (bounds.rowCount <= 0 || bounds.columnCount <= 0) return current;
  if (backwards) {
    if (current.col > 0) return { row: current.row, col: current.col - 1 };
    return current.row > 0
      ? { row: current.row - 1, col: bounds.columnCount - 1 }
      : current;
  }
  if (current.col < bounds.columnCount - 1)
    return { row: current.row, col: current.col + 1 };
  return current.row < bounds.rowCount - 1 ? { row: current.row + 1, col: 0 } : current;
}

/** Enter moves down, Shift+Enter moves up; both clamp at the column edges. */
export function enterPosition(
  position: GridPosition,
  backwards: boolean,
  bounds: GridBounds
): GridPosition {
  return stepPosition(position, backwards ? "up" : "down", bounds);
}

/** Home and End jump to the first or last column of the current row. */
export function rowEdgePosition(
  position: GridPosition,
  edge: "home" | "end",
  bounds: GridBounds
): GridPosition {
  return clampPosition(
    { row: position.row, col: edge === "home" ? 0 : bounds.columnCount - 1 },
    bounds
  );
}

/** Ctrl/Cmd+Home selects the origin; Ctrl/Cmd+End selects the last used cell. */
export function documentEdgePosition(
  edge: "home" | "end",
  bounds: GridBounds,
  isEmpty: (position: GridPosition) => boolean
): GridPosition {
  if (edge === "home" || bounds.rowCount <= 0 || bounds.columnCount <= 0)
    return { row: 0, col: 0 };
  const last = { row: 0, col: 0 };
  for (let row = 0; row < bounds.rowCount; row += 1) {
    for (let col = 0; col < bounds.columnCount; col += 1) {
      if (!isEmpty({ row, col })) {
        last.row = Math.max(last.row, row);
        last.col = Math.max(last.col, col);
      }
    }
  }
  return last;
}

/** Page movement preserves the column and clamps at the first/last row. */
export function pagePosition(
  position: GridPosition,
  direction: "up" | "down",
  pageSize: number,
  bounds: GridBounds
): GridPosition {
  const distance = Math.max(1, Math.trunc(pageSize));
  return clampPosition(
    {
      row: position.row + (direction === "up" ? -distance : distance),
      col: position.col,
    },
    bounds
  );
}

/**
 * Ctrl/Cmd+Arrow data-edge jump with Excel semantics:
 * - From a filled cell followed by a filled cell, run to the last filled cell before a gap.
 * - From a filled cell followed by a gap, or from an empty cell, land on the next filled
 *   cell in that direction, or the far edge when nothing else is filled.
 */
export function jumpPosition(
  position: GridPosition,
  direction: GridDirection,
  bounds: GridBounds,
  isEmpty: (position: GridPosition) => boolean
): GridPosition {
  const origin = clampPosition(position, bounds);
  const next = stepPosition(origin, direction, bounds);
  if (samePosition(next, origin)) return origin;

  if (!isEmpty(origin) && !isEmpty(next)) {
    let current = next;
    while (true) {
      const following = stepPosition(current, direction, bounds);
      if (samePosition(following, current) || isEmpty(following)) return current;
      current = following;
    }
  }

  let current = next;
  while (isEmpty(current)) {
    const following = stepPosition(current, direction, bounds);
    if (samePosition(following, current)) return following;
    current = following;
  }
  return current;
}

/** Normalized rectangle spanned by a range anchor and the active cell. */
export function normalizeRange(anchor: GridPosition, focus: GridPosition): GridRange {
  return {
    top: Math.min(anchor.row, focus.row),
    left: Math.min(anchor.col, focus.col),
    bottom: Math.max(anchor.row, focus.row),
    right: Math.max(anchor.col, focus.col),
  };
}

export function positionInRange(position: GridPosition, range: GridRange): boolean {
  return (
    position.row >= range.top &&
    position.row <= range.bottom &&
    position.col >= range.left &&
    position.col <= range.right
  );
}

export function sameRange(left: GridRange, right: GridRange): boolean {
  return (
    left.top === right.top &&
    left.left === right.left &&
    left.bottom === right.bottom &&
    left.right === right.right
  );
}

export function fullGridRange(bounds: GridBounds): GridRange {
  return {
    top: 0,
    left: 0,
    bottom: Math.max(0, bounds.rowCount - 1),
    right: Math.max(0, bounds.columnCount - 1),
  };
}

export function fullRowRange(row: number, bounds: GridBounds): GridRange {
  const position = clampPosition({ row, col: 0 }, bounds);
  return {
    top: position.row,
    left: 0,
    bottom: position.row,
    right: Math.max(0, bounds.columnCount - 1),
  };
}

export function fullColumnRange(col: number, bounds: GridBounds): GridRange {
  const position = clampPosition({ row: 0, col }, bounds);
  return {
    top: 0,
    left: position.col,
    bottom: Math.max(0, bounds.rowCount - 1),
    right: position.col,
  };
}

/**
 * Bounding rectangle of the orthogonally connected non-empty region containing
 * the active cell. An empty active cell selects only itself.
 */
export function contiguousDataRange(
  origin: GridPosition,
  bounds: GridBounds,
  isEmpty: (position: GridPosition) => boolean
): GridRange {
  const start = clampPosition(origin, bounds);
  if (bounds.rowCount <= 0 || bounds.columnCount <= 0 || isEmpty(start))
    return normalizeRange(start, start);
  const queue = [start];
  const visited = new Set([`${start.row}:${start.col}`]);
  const range = normalizeRange(start, start);
  for (let index = 0; index < queue.length; index += 1) {
    const current = queue[index];
    range.top = Math.min(range.top, current.row);
    range.left = Math.min(range.left, current.col);
    range.bottom = Math.max(range.bottom, current.row);
    range.right = Math.max(range.right, current.col);
    for (const direction of Object.keys(DIRECTION_DELTAS) as GridDirection[]) {
      const next = stepPosition(current, direction, bounds);
      const key = `${next.row}:${next.col}`;
      if (samePosition(next, current) || visited.has(key) || isEmpty(next)) continue;
      visited.add(key);
      queue.push(next);
    }
  }
  return range;
}

export function positionsInRange(range: GridRange): GridPosition[] {
  const positions: GridPosition[] = [];
  for (let row = range.top; row <= range.bottom; row += 1) {
    for (let col = range.left; col <= range.right; col += 1)
      positions.push({ row, col });
  }
  return positions;
}

export interface FillPair {
  source: GridPosition;
  target: GridPosition;
}

/** Source/target pairs for Ctrl/Cmd+D (down) and Ctrl/Cmd+R (right). */
export function fillPairs(range: GridRange, direction: "down" | "right"): FillPair[] {
  const pairs: FillPair[] = [];
  if (direction === "down") {
    for (let row = range.top + 1; row <= range.bottom; row += 1) {
      for (let col = range.left; col <= range.right; col += 1) {
        pairs.push({ source: { row: range.top, col }, target: { row, col } });
      }
    }
  } else {
    for (let row = range.top; row <= range.bottom; row += 1) {
      for (let col = range.left + 1; col <= range.right; col += 1) {
        pairs.push({ source: { row, col: range.left }, target: { row, col } });
      }
    }
  }
  return pairs;
}

/** True when a keydown should open the cell editor replacing its content. */
export function isPrintableKey(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey">
): boolean {
  return event.key.length === 1 && !event.ctrlKey && !event.metaKey;
}

/**
 * Scroll offset that brings a virtualized row into view, or null when it is already
 * fully visible. Mirrors the calculateVirtualRowRange scroll mapping, including the
 * sticky header band and the capped-spacer scale for very large frames.
 */
export function scrollTopToRevealRow(
  rowIndex: number,
  rowCount: number,
  scrollTop: number,
  viewportHeight: number,
  rowHeight = TABLE_ROW_HEIGHT,
  headerHeight = TABLE_HEADER_HEIGHT
): number | null {
  if (rowCount <= 0 || rowIndex < 0 || rowIndex >= rowCount) return null;
  const scale = Math.max(1, (rowCount * rowHeight) / MAX_VIRTUAL_SCROLL_HEIGHT);
  const rowTop = headerHeight + (rowIndex * rowHeight) / scale;
  const rowBottom = rowTop + rowHeight / scale;
  if (rowTop < scrollTop + headerHeight) return Math.max(0, rowTop - headerHeight);
  if (rowBottom > scrollTop + viewportHeight) return rowBottom - viewportHeight;
  return null;
}

/**
 * Horizontal scroll offset that reveals a cell spanning [left, left + width), or null
 * when it is already visible. stickyLeadingWidth is the width of a sticky leading
 * column that overlays the left edge of the viewport.
 */
export function scrollLeftToRevealColumn(
  left: number,
  width: number,
  scrollLeft: number,
  viewportWidth: number,
  stickyLeadingWidth = 0
): number | null {
  if (left < scrollLeft + stickyLeadingWidth)
    return Math.max(0, left - stickyLeadingWidth);
  if (left + width > scrollLeft + viewportWidth) return left + width - viewportWidth;
  return null;
}
