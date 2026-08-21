import type { CanvasView, DocumentView, Selection } from "./types";

export type CanvasNavigationDirection =
  | "left"
  | "right"
  | "up"
  | "down"
  | "next"
  | "previous";

export function selectedCanvasView(
  document: DocumentView | null,
  selection: Selection | null
): CanvasView | undefined {
  if (!document || !selection) return undefined;
  return selection.viewId
    ? document.views.find((view) => view.id === selection.viewId)
    : document.views.find(
        (view) =>
          view.objectId === selection.objectId ||
          view.tabObjectIds?.includes(selection.objectId)
      );
}

export function withCanvasView(
  view: CanvasView | undefined,
  action: (view: CanvasView) => void
) {
  if (view) action(view);
}

const visibleHeight = (view: CanvasView) => (view.collapsed ? 29 : view.height);

const intervalGap = (aStart: number, aEnd: number, bStart: number, bEnd: number) =>
  Math.max(0, aStart - bEnd, bStart - aEnd);

function spatialDistance(
  current: CanvasView,
  target: CanvasView,
  direction: Exclude<CanvasNavigationDirection, "next" | "previous">
): { primary: number; cross: number } | null {
  const currentBottom = current.y + visibleHeight(current);
  const targetBottom = target.y + visibleHeight(target);
  if (direction === "right" && target.x >= current.x + current.width)
    return {
      primary: target.x - current.x - current.width,
      cross: intervalGap(current.y, currentBottom, target.y, targetBottom),
    };
  if (direction === "left" && target.x + target.width <= current.x)
    return {
      primary: current.x - target.x - target.width,
      cross: intervalGap(current.y, currentBottom, target.y, targetBottom),
    };
  if (direction === "down" && target.y >= currentBottom)
    return {
      primary: target.y - currentBottom,
      cross: intervalGap(current.x, current.x + current.width, target.x, target.x + target.width),
    };
  if (direction === "up" && targetBottom <= current.y)
    return {
      primary: current.y - targetBottom,
      cross: intervalGap(current.x, current.x + current.width, target.x, target.x + target.width),
    };
  return null;
}

/**
 * Finds the card a person means by an arrow key in canvas mode.
 *
 * Directional navigation first respects the requested half-plane, then
 * favours alignment over a merely short diagonal. That is the convention
 * spatial interfaces use because Right should keep walking a row when it can;
 * a card whose corner happens to be closer should not pull the cursor away.
 * Cycling follows the arranged canvas's dependency reading order: columns
 * from left to right, then cards from top to bottom within each column.
 */
export function canvasNavigationTarget(
  views: readonly CanvasView[],
  currentViewId: string,
  direction: CanvasNavigationDirection
): CanvasView | null {
  if (views.length === 0) return null;
  const ordered = [...views].sort(
    (left, right) => left.x - right.x || left.y - right.y || left.id.localeCompare(right.id)
  );
  const current = views.find((view) => view.id === currentViewId);
  if (!current) return direction === "previous" ? ordered.at(-1) ?? null : ordered[0];

  if (direction === "next" || direction === "previous") {
    const index = ordered.findIndex((view) => view.id === current.id);
    const offset = direction === "next" ? 1 : -1;
    return ordered[(index + offset + ordered.length) % ordered.length];
  }

  const candidates = views.flatMap((view) => {
    if (view.id === current.id) return [];
    const distance = spatialDistance(current, view, direction);
    if (!distance) return [];
    const { primary, cross } = distance;
    return [{ view, score: primary + cross * 2, primary, cross }];
  });

  candidates.sort(
    (left, right) =>
      left.score - right.score ||
      left.cross - right.cross ||
      left.primary - right.primary ||
      left.view.id.localeCompare(right.view.id)
  );
  return candidates[0]?.view ?? null;
}
