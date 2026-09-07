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
 *
 * The block's width is the one number here that is a product decision rather
 * than a guess, and it is spelled the same way in `prepare_add_block`. A
 * Scratchwork card spends 150px on the answer gutter, 34px on the card's own
 * padding, 18px on the editor's and 2px of border before a character is
 * drawn, and DM Mono at 13px advances about 7.8px. At 520px a line was cut at
 * about forty-one characters — the tutorials' own first line, `Total revenue
 * = ...Revenue.sum()`, lost its `um()` as it was typed — so the first thing
 * after ⌘J was reaching for the resize handle. 600px carries a fifty-
 * character reference line with its answer still beside it. The card resizes
 * like any other (house rule 7); this is only where it starts.
 */
export const CARD_SIZES = {
  block: { width: 600, height: 220 },
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

/** How far inside the viewport's edges a new card is allowed to sit. */
const VIEWPORT_INSET = 24;
/** The scan's resolution: fine enough to find a real gap, coarse enough to be cheap. */
const SCAN_STEP = 20;

/**
 * A free spot for a new card *on the screen you are looking at*.
 *
 * `placeNewCard` answers "somewhere that is not on top of anything", which
 * was the whole question while the canvas was small. It is not the whole
 * question once the canvas is bigger than the window: ⌘J with a table filling
 * the viewport put the new Scratchwork block just below that table — free
 * space, correctly — and then focusing its editor scrolled the canvas down to
 * it, so the table the tutorial was about to ask you to click had left the
 * screen. A card you asked for should arrive where you are looking, and the
 * things you were looking at should still be there.
 *
 * So the search is bounded by the viewport, in canvas units, and every free
 * position is scored by how far the canvas would have to scroll to show the
 * card whole. A position on screen scores zero, and among those the one
 * nearest the viewport's top-left corner wins — which is exactly the old
 * behaviour, now as the zero-cost case of a wider question.
 *
 * The wider question matters because "somewhere on screen" is often not
 * available. The tour's Start workbook fills a 1400×1000 window with a tall
 * text card down the left and two tables stacked on the right, and there is
 * no free 600×220 rectangle on it at all. Falling back to "under the lowest
 * card you can see" put the block below *everything*, so the scroll that
 * revealed it took both tables with it. Scoring by scroll distance instead
 * tucks the block just under the text card — a shorter journey, and one that
 * leaves the tables in the corner of your eye. The sweep runs to the far
 * edge of the cards you can see and no further: past their corner the canvas
 * is uniformly empty, so nothing out there can beat the nearest point in.
 *
 * Finding the spot is only half of it — see `minimalRevealScroll` in
 * `canvasReveal.ts` for the scroll that then brings it into view, which has
 * to be the same "least movement" rule or this arithmetic is wasted.
 *
 * `viewport` is the visible region in canvas coordinates (scroll offset and
 * client size, both divided by the zoom), which is what makes this pure
 * arithmetic testable without a DOM.
 */
export function placeNewCardInViewport(
  existing: readonly Rect[],
  viewport: Rect,
  size: CardSize
): Point {
  const anchor = { x: viewport.x + VIEWPORT_INSET, y: viewport.y + VIEWPORT_INSET };
  const visible = existing.filter((rect) => overlaps(rect, viewport));
  const rights = visible.map((rect) => rect.x + rect.width + GAP_X);
  const bottoms = visible.map((rect) => rect.y + rect.height + GAP_Y);
  const furthest = (edges: readonly number[], floor: number) =>
    edges.reduce((far, edge) => Math.max(far, edge), floor);
  const limitX = furthest(
    rights,
    Math.max(anchor.x, viewport.x + viewport.width - VIEWPORT_INSET - size.width)
  );
  const limitY = furthest(
    bottoms,
    Math.max(anchor.y, viewport.y + viewport.height - VIEWPORT_INSET - size.height)
  );
  const xs = sweep(anchor.x, limitX, rights);
  const ys = sweep(anchor.y, limitY, bottoms);

  // How far the canvas would have to move to show the whole card. The sweep
  // never starts above or left of the anchor, so only the far edges can fall
  // outside and only a forward scroll is ever needed.
  const scrollCost = (point: Point) =>
    Math.max(0, point.x + size.width + VIEWPORT_INSET - (viewport.x + viewport.width)) +
    Math.max(0, point.y + size.height + VIEWPORT_INSET - (viewport.y + viewport.height));

  let best: Point | null = null;
  let bestCost = Infinity;
  let bestDistance = Infinity;
  for (const y of ys) {
    for (const x of xs) {
      const candidate = { x, y };
      if (!isFree(candidate, size, existing)) continue;
      const cost = scrollCost(candidate);
      const away = distance(candidate, anchor);
      if (cost > bestCost || (cost === bestCost && away >= bestDistance)) continue;
      best = candidate;
      bestCost = cost;
      bestDistance = away;
    }
  }
  // Every swept position collided — a canvas tiled edge to edge, or a card
  // bigger than the window it is being asked to fit inside. The ordinary
  // cascade is the honest answer for both.
  return best ?? placeNewCard(existing, anchor, size);
}

/**
 * The coordinates to try along one axis: a regular step, plus the exact
 * "just past that card" edges so a gap the step happens to straddle is still
 * found, and found flush against its neighbour rather than up to 19px away.
 */
function sweep(start: number, limit: number, edges: readonly number[]): number[] {
  const values = new Set<number>();
  for (let value = start; value <= limit; value += SCAN_STEP) values.add(value);
  values.add(limit);
  for (const edge of edges) if (edge > start && edge < limit) values.add(edge);
  return [...values].sort((a, b) => a - b);
}

/**
 * A frame card sized to what it holds, for the moments the content arrives
 * all at once -- a paste into an empty table, a join. The card used to keep
 * the size of the empty stub it replaced and clip the result both ways, so
 * the first thing after "paste" was reaching for the resize handle. Capped,
 * because a thousand-row frame is a thing to scroll, not a card to unroll.
 */
export function frameCardSize(columnCount: number, rowCount: number): CardSize {
  const ROW_NUMBER_GUTTER = 48;
  const COLUMN_WIDTH = 150;
  return {
    width: Math.max(420, Math.min(900, ROW_NUMBER_GUTTER + columnCount * COLUMN_WIDTH)),
    height: frameCardHeight(rowCount),
  };
}

/**
 * Everything above and below the rows: the card's border (2), the drag handle
 * (32), the tab strip (36), the title row (50), the grid's bottom padding
 * (12), and inside the scroller the column header (34), the type row (26),
 * the draft "type here" row an owned frame always shows (34) and the four
 * gaps the table leaves around them.
 *
 * Those gaps are the part that keeps getting missed. The grid's table is in
 * the separated-border model with the user agent's own 2px `border-spacing`
 * — the stylesheet's reset for it names `frame` rather than `table`, so it
 * has never applied — which puts 2px above every row and 2px below the last.
 * A row therefore costs 36, not 34, and there is one spacing left over at the
 * bottom. The tally was 184 for a long time (no draft row, no rules) and then
 * 230 (no spacing), and each time the symptom was the same: a six-row paste
 * arriving as five rows and an in-card scrollbar. A number wrong in this
 * direction is not a cosmetic near-miss — it is the difference between seeing
 * your data and being told there is more of it.
 */
const FRAME_CHROME_HEIGHT = 234;
const FRAME_ROW_HEIGHT = 36;
/**
 * How many rows a card sizes itself to before it becomes a thing you scroll.
 * A dozen is about a screenful of a small table and comfortably over the six
 * rows the tutorials' tables carry.
 */
export const FRAME_AUTOMATIC_ROW_CAP = 12;
const FRAME_MIN_HEIGHT = 300;
const FRAME_MAX_HEIGHT = FRAME_CHROME_HEIGHT + FRAME_AUTOMATIC_ROW_CAP * FRAME_ROW_HEIGHT;

/** The height that shows `rowCount` rows outright, within the cap. */
export function frameCardHeight(rowCount: number): number {
  const rows = Math.max(0, Math.min(rowCount, FRAME_AUTOMATIC_ROW_CAP));
  return Math.max(
    FRAME_MIN_HEIGHT,
    Math.min(FRAME_MAX_HEIGHT, FRAME_CHROME_HEIGHT + rows * FRAME_ROW_HEIGHT)
  );
}

/** How many rows a card of this height actually draws — the inverse. */
export function frameCardVisibleRows(height: number): number {
  return Math.max(1, Math.floor((height - FRAME_CHROME_HEIGHT) / FRAME_ROW_HEIGHT));
}

/**
 * Whether a card's height is one this module chose rather than one a person
 * dragged to.
 *
 * There is no flag on `CanvasView` saying "the user sized this", and adding
 * one is a document-format change for a question the geometry can already
 * answer: an automatic height is always the chrome plus a whole number of
 * rows, or one of the two clamps. A hand-dragged card lands on that ladder
 * about one time in thirty-four, and the cost of that coincidence is a card
 * that grows one row taller than the person left it — not lost work.
 */
export function isAutomaticFrameCardHeight(height: number): boolean {
  if (height === FRAME_MIN_HEIGHT || height === FRAME_MAX_HEIGHT) return true;
  const rows = (height - FRAME_CHROME_HEIGHT) / FRAME_ROW_HEIGHT;
  return Number.isInteger(rows) && rows >= 0 && rows <= FRAME_AUTOMATIC_ROW_CAP;
}

/**
 * The height a card should grow to now that it holds `rowCount` rows, or
 * `null` if it should be left alone.
 *
 * Growing only: a paste that adds rows should not have to be followed by a
 * resize, but a card someone made room in stays as big as they made it, and a
 * card that has been sized by hand is never touched at all.
 */
export function grownFrameCardHeight(
  currentHeight: number,
  rowCount: number
): number | null {
  if (!isAutomaticFrameCardHeight(currentHeight)) return null;
  const wanted = frameCardHeight(rowCount);
  return wanted > currentHeight ? wanted : null;
}
