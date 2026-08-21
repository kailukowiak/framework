export const TABLE_ROW_HEIGHT = 34;
export const TABLE_HEADER_HEIGHT = 57;
export const TABLE_ROW_OVERSCAN = 8;
export const MAX_VIRTUAL_SCROLL_HEIGHT = 8_000_000;

export interface VirtualRowRange {
  start: number;
  end: number;
  paddingTop: number;
  paddingBottom: number;
}

export function calculateVirtualRowRange(
  rowCount: number,
  scrollTop: number,
  viewportHeight: number,
  rowHeight = TABLE_ROW_HEIGHT,
  overscan = TABLE_ROW_OVERSCAN
): VirtualRowRange {
  if (rowCount <= 0) {
    return { start: 0, end: 0, paddingTop: 0, paddingBottom: 0 };
  }

  const bodyScrollTop = Math.max(0, scrollTop - TABLE_HEADER_HEIGHT);
  const naturalBodyHeight = rowCount * rowHeight;
  const scrollScale = Math.max(1, naturalBodyHeight / MAX_VIRTUAL_SCROLL_HEIGHT);
  const firstVisible = Math.min(
    rowCount - 1,
    Math.floor((bodyScrollTop * scrollScale) / rowHeight)
  );
  const visibleRows = Math.max(1, Math.ceil(viewportHeight / rowHeight));
  const start = Math.max(0, firstVisible - overscan);
  const end = Math.min(rowCount, firstVisible + visibleRows + overscan);

  return {
    start,
    end,
    paddingTop: (start * rowHeight) / scrollScale,
    paddingBottom: ((rowCount - end) * rowHeight) / scrollScale,
  };
}
