/**
 * Where a brand-new canvas card goes.
 *
 * Every "add a thing" path — the ADD rail, a keyboard shortcut, an import,
 * the Scratchwork toggle — used to hand the backend a fixed point computed
 * only from the viewport's current scroll position, with no idea what else
 * was already on the canvas. That is fine on an empty canvas and wrong on
 * every other one: importing a CSV landed the new frame exactly on top of
 * Frame 1, hiding it outright; clicking Matrix right after an Arrange
 * dropped the new card behind the frame that Arrange had just placed there.
 * The fix is not "compute a better fixed point" — no fixed point survives
 * every layout — it is to check the spot against what is actually there and
 * step aside if it collides.
 *
 * This module is deliberately just geometry: it knows rectangles and a
 * preferred point, not documents, operations, or object kinds. That keeps
 * it trivially unit-testable (no store, no React, no jsdom) and reusable
 * from every spawn path without any of them depending on each other.
 */

/** Anything with a canvas rectangle — `CanvasView` satisfies this structurally. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CardSize {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

/**
 * Approximate footprints for each card kind, mirroring the defaults each
 * `prepare_add_*` sets server-side in
 * `crates/framework-core/src/operation/prepare/{objects,blocks,calculation_matrix}.rs`.
 * They only need to be close: this is an anchor guess to avoid an overlap,
 * not the authoritative size, and the server decides the real one (a frame
 * in particular grows wider with its column count). If those defaults move,
 * nudge these to match — a stale guess just makes the placement slightly
 * less precise, it does not make it wrong.
 */
export const CARD_SIZES = {
  block: { width: 340, height: 220 },
  calculationMatrix: { width: 440, height: 240 },
  container: { width: 260, height: 240 },
  text: { width: 480, height: 280 },
  variable: { width: 320, height: 46 },
  frame: { width: 420, height: 300 },
} as const satisfies Record<string, CardSize>;

/** The breathing room between a new card and its nearest neighbour. */
const GAP_X = 64;
const GAP_Y = 28;

/** How far, and how many times, the cascade sweep steps before giving up. */
const CASCADE_STEP = 40;
const MAX_CASCADE_STEPS = 24;

function rectAt(point: Point, size: CardSize): Rect {
  return { x: point.x, y: point.y, width: size.width, height: size.height };
}

function overlaps(a: Rect, b: Rect): boolean {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  );
}

function isFree(point: Point, size: CardSize, existing: readonly Rect[]): boolean {
  const candidate = rectAt(point, size);
  return !existing.some((rect) => overlaps(candidate, rect));
}

function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/**
 * A free spot for a new card of `size`, starting from `anchor` and
 * displacing only as far as it has to.
 *
 * The search order mirrors how a person would find room by eye: try where
 * you meant to put it; if something is there, look just to the right of it
 * or just below it (the two directions this canvas already grows in — see
 * `ROW_GAP`/`COLUMN_GAP` in `model/layout.rs`, which this mirrors for
 * visual consistency with Arrange); if the neighbourhood is that crowded,
 * step outward in a fixed, bounded sweep until something opens up. The
 * bound is what keeps a fully tiled canvas from spinning forever — it
 * returns the last spot it tried rather than loop without end, which is a
 * card someone can drag off a pile, not a hang.
 *
 * Deterministic: the same `existing` rectangles and `anchor` always produce
 * the same point, so this needs no seeding or mocking to test.
 */
export function placeNewCard(
  existing: readonly Rect[],
  anchor: Point,
  size: CardSize
): Point {
  if (isFree(anchor, size, existing)) return anchor;

  const preferredRect = rectAt(anchor, size);
  const blockers = existing.filter((rect) => overlaps(preferredRect, rect));

  const candidates: Point[] = [];
  for (const blocker of blockers) {
    candidates.push({ x: blocker.x + blocker.width + GAP_X, y: anchor.y });
    candidates.push({ x: anchor.x, y: blocker.y + blocker.height + GAP_Y });
  }
  for (let step = 1; step <= MAX_CASCADE_STEPS; step++) {
    candidates.push({
      x: anchor.x + step * CASCADE_STEP,
      y: anchor.y + step * CASCADE_STEP,
    });
  }

  let nearestFree: Point | null = null;
  for (const candidate of candidates) {
    if (!isFree(candidate, size, existing)) continue;
    if (!nearestFree || distance(candidate, anchor) < distance(nearestFree, anchor)) {
      nearestFree = candidate;
    }
  }
  // Every candidate collided (a fully tiled region wider than the cascade's
  // reach) — hand back the sweep's furthest, still-deterministic point
  // rather than leave the card exactly where it started.
  return nearestFree ?? candidates[candidates.length - 1];
}
