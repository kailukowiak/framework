import { describe, expect, it } from "vitest";
import {
  CARD_SIZES,
  FRAME_AUTOMATIC_ROW_CAP,
  frameCardHeight,
  frameCardSize,
  frameCardVisibleRows,
  grownFrameCardHeight,
  isAutomaticFrameCardHeight,
  placeNewCard,
  placeNewCardInViewport,
  type Rect,
} from "./cardPlacement";

const SIZE = CARD_SIZES.block;
const ANCHOR = { x: 110, y: 100 };

describe("placeNewCard", () => {
  it("returns the preferred spot on an empty canvas", () => {
    expect(placeNewCard([], ANCHOR, SIZE)).toEqual(ANCHOR);
  });

  it("returns the preferred spot when nothing there overlaps it", () => {
    const farAway: Rect = { x: 2000, y: 2000, width: 300, height: 200 };
    expect(placeNewCard([farAway], ANCHOR, SIZE)).toEqual(ANCHOR);
  });

  it("displaces a card whose preferred spot overlaps an existing view", () => {
    // Exactly the reported bug: an existing card sits right on the anchor.
    const frame1: Rect = { x: ANCHOR.x, y: ANCHOR.y, width: 420, height: 300 };
    const placed = placeNewCard([frame1], ANCHOR, SIZE);

    expect(placed).not.toEqual(ANCHOR);
    const placedRect: Rect = { ...placed, width: SIZE.width, height: SIZE.height };
    expect(rectsOverlap(placedRect, frame1)).toBe(false);
  });

  it("prefers a spot beside the blocker over a far cascade jump", () => {
    const blocker: Rect = { x: ANCHOR.x, y: ANCHOR.y, width: 420, height: 300 };
    const placed = placeNewCard([blocker], ANCHOR, SIZE);

    // The nearest free candidate is just to the right of, or just below,
    // the one thing in the way — not several cascade steps out.
    expect(placed.x - ANCHOR.x < 500 || placed.y - ANCHOR.y < 500).toBe(true);
  });

  it("does not overlap any existing view among several scattered cards", () => {
    const existing: Rect[] = [
      { x: 110, y: 100, width: 420, height: 300 }, // sits on the anchor
      { x: 590, y: 100, width: 300, height: 200 }, // blocks "right of"
      { x: 110, y: 456, width: 300, height: 200 }, // blocks "below"
    ];
    const placed = placeNewCard(existing, ANCHOR, SIZE);
    const placedRect: Rect = { ...placed, width: SIZE.width, height: SIZE.height };
    for (const rect of existing) {
      expect(rectsOverlap(placedRect, rect)).toBe(false);
    }
  });

  it("falls back to the cascade sweep in a fully crowded region, terminating instead of looping forever", () => {
    // Tile a dense grid of cards over a much larger area than "right of" or
    // "below" the anchor's immediate blocker could escape in one hop, so
    // the only way out is the bounded cascade sweep.
    const existing: Rect[] = [];
    for (let row = 0; row < 12; row++) {
      for (let col = 0; col < 12; col++) {
        existing.push({
          x: col * 60,
          y: row * 60,
          width: 56,
          height: 56,
        });
      }
    }

    const start = performance.now();
    const placed = placeNewCard(existing, ANCHOR, SIZE);
    const elapsed = performance.now() - start;

    // The point of the bound: this returns promptly, not eventually.
    expect(elapsed).toBeLessThan(50);
    expect(Number.isFinite(placed.x)).toBe(true);
    expect(Number.isFinite(placed.y)).toBe(true);
  });

  it("is deterministic for the same inputs", () => {
    const existing: Rect[] = [
      { x: 110, y: 100, width: 420, height: 300 },
      { x: 620, y: 100, width: 300, height: 200 },
    ];
    const first = placeNewCard(existing, ANCHOR, SIZE);
    const second = placeNewCard(existing, ANCHOR, SIZE);
    expect(first).toEqual(second);
  });
});

function rectsOverlap(a: Rect, b: Rect): boolean {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  );
}

describe("frameCardSize", () => {
  it("grows with columns and rows, within a cap", () => {
    const small = frameCardSize(2, 2);
    const wide = frameCardSize(6, 6);
    expect(wide.width).toBeGreaterThan(small.width);
    expect(wide.height).toBeGreaterThan(small.height);
    const capped = frameCardSize(40, 5000);
    expect(capped.width).toBe(900);
    expect(capped.height).toBe(frameCardSize(40, FRAME_AUTOMATIC_ROW_CAP).height);
  });
});

describe("placeNewCardInViewport", () => {
  // A window's worth of canvas, scrolled somewhere off the origin.
  const VIEWPORT: Rect = { x: 400, y: 300, width: 1400, height: 800 };

  it("places a card inside the viewport when the viewport is empty", () => {
    const placed = placeNewCardInViewport([], VIEWPORT, SIZE);
    expect(placed.x).toBeGreaterThanOrEqual(VIEWPORT.x);
    expect(placed.y).toBeGreaterThanOrEqual(VIEWPORT.y);
    expect(placed.x + SIZE.width).toBeLessThanOrEqual(VIEWPORT.x + VIEWPORT.width);
    expect(placed.y + SIZE.height).toBeLessThanOrEqual(VIEWPORT.y + VIEWPORT.height);
  });

  it("keeps the card on screen and off the table beside it — the ⌘J case", () => {
    // A table filling the left two thirds of the window, as the tour's does.
    const table: Rect = { x: 420, y: 320, width: 700, height: 700 };
    const placed = placeNewCardInViewport([table], VIEWPORT, SIZE);

    expect(rectsOverlap({ ...placed, ...SIZE }, table)).toBe(false);
    // Still on screen: the table the tutorial is about to point at has not
    // been scrolled away to make room for the block.
    expect(placed.x + SIZE.width).toBeLessThanOrEqual(VIEWPORT.x + VIEWPORT.width);
    expect(placed.y + SIZE.height).toBeLessThanOrEqual(VIEWPORT.y + VIEWPORT.height);
  });

  it("avoids every card on screen, not just the one under the corner", () => {
    const existing: Rect[] = [
      { x: 420, y: 320, width: 500, height: 300 },
      { x: 420, y: 640, width: 500, height: 300 },
      { x: 960, y: 320, width: 400, height: 260 },
    ];
    const placed = placeNewCardInViewport(existing, VIEWPORT, SIZE);
    for (const rect of existing) {
      expect(rectsOverlap({ ...placed, ...SIZE }, rect)).toBe(false);
    }
  });

  // The tour's Start workbook, in canvas units, straight out of
  // `tutorials/grand-tour/grand-tour-start.fw`: a tall walkthrough card down
  // the left, two tables stacked to its right. In a 1400×1000 window there is
  // no free 600×220 rectangle on screen at all, which is what makes it the
  // case worth pinning: the answer has to be the *least* off-screen spot, not
  // merely a free one.
  const TOUR_START: Rect[] = [
    { x: 70, y: 70, width: 580, height: 820 },
    { x: 720, y: 70, width: 420, height: 300 },
    { x: 720, y: 500, width: 420, height: 434 },
  ];
  // Where `contentOriginScroll` leaves a freshly opened document: the corner
  // of its cards, less the reveal margin.
  const TOUR_VIEWPORT: Rect = { x: 46, y: 46, width: 1100, height: 860 };

  it("tucks a block under the nearest card when no screen is free — the tour case", () => {
    const placed = placeNewCardInViewport(TOUR_START, TOUR_VIEWPORT, SIZE);
    for (const card of TOUR_START)
      expect(rectsOverlap({ ...placed, ...SIZE }, card)).toBe(false);

    // Under the walkthrough card, not under the whole document: the tables
    // end at y 934, and landing below *them* would cost 300px of scrolling.
    // The scroll this asks for is what the promise is really about — after
    // it, the Monthly sales table (y 70..370) is still on screen.
    const scrollTop = placed.y + SIZE.height + 24 - TOUR_VIEWPORT.height;
    expect(scrollTop).toBeLessThan(370);
    expect(scrollTop).toBeLessThan(934 + 28 + SIZE.height + 24 - TOUR_VIEWPORT.height);
  });

  it("prefers a spot that needs no scrolling over a nearer one that does", () => {
    // A single card leaves the anchor blocked but the space to its right
    // free and on screen; the sweep must not settle for "below everything"
    // just because that lands closer to the anchor.
    const blocker: Rect = { x: 400, y: 300, width: 600, height: 300 };
    const placed = placeNewCardInViewport([blocker], VIEWPORT, SIZE);
    expect(placed.x + SIZE.width).toBeLessThanOrEqual(VIEWPORT.x + VIEWPORT.width);
    expect(placed.y + SIZE.height).toBeLessThanOrEqual(VIEWPORT.y + VIEWPORT.height);
  });

  it("falls just below the lowest visible card when the screen is full", () => {
    const full: Rect[] = [{ x: 0, y: 0, width: 4000, height: 1400 }];
    const placed = placeNewCardInViewport(full, VIEWPORT, SIZE);
    // Under the card that is in the way, not under everything in the
    // document: one nudge of scrolling reveals it.
    expect(placed.y).toBeGreaterThanOrEqual(1400);
    expect(placed.y).toBeLessThan(1400 + 200);
    expect(placed.x).toBeGreaterThanOrEqual(VIEWPORT.x);
  });

  it("falls back to the free cascade for a card larger than the window", () => {
    const huge = { width: 4000, height: 3000 };
    const placed = placeNewCardInViewport([], VIEWPORT, huge);
    expect(Number.isFinite(placed.x)).toBe(true);
    expect(Number.isFinite(placed.y)).toBe(true);
  });

  it("is deterministic", () => {
    const existing: Rect[] = [{ x: 420, y: 320, width: 700, height: 400 }];
    expect(placeNewCardInViewport(existing, VIEWPORT, SIZE)).toEqual(
      placeNewCardInViewport(existing, VIEWPORT, SIZE)
    );
  });
});

describe("frame card height", () => {
  it("counts the table's own border spacing, not just its rows", () => {
    // The tally in prose: 2 border + 32 drag handle + 36 tab strip + 50 title
    // + 12 grid padding outside the scroller; 34 column header + 26 type row
    // + 34 draft row + 8 of border spacing inside it. A row costs 36, because
    // the grid's table is in the separated-border model and the user agent's
    // 2px `border-spacing` sits above every one of them. Missing those eight
    // pixels of spacing is what left a six-row paste 16px short of its last
    // row.
    expect(frameCardHeight(6)).toBe(234 + 6 * 36);
    expect(frameCardHeight(7) - frameCardHeight(6)).toBe(36);
  });

  it("shows every row of a six-row table without an in-card scroll", () => {
    // The regression this exists for: a six-row answer-key table showed five
    // rows, because the chrome tally left out the draft row and half the
    // rules. Six rows have to fit inside the height six rows are given.
    expect(frameCardVisibleRows(frameCardHeight(6))).toBeGreaterThanOrEqual(6);
  });

  it("fits what it is given, up to the dozen-row cap", () => {
    for (const rows of [1, 2, 5, 6, 9, 12]) {
      expect(frameCardVisibleRows(frameCardHeight(rows))).toBeGreaterThanOrEqual(
        Math.min(rows, FRAME_AUTOMATIC_ROW_CAP)
      );
    }
    expect(frameCardHeight(500)).toBe(frameCardHeight(FRAME_AUTOMATIC_ROW_CAP));
  });

  it("recognises its own heights and leaves a hand-dragged one alone", () => {
    expect(isAutomaticFrameCardHeight(frameCardHeight(6))).toBe(true);
    expect(isAutomaticFrameCardHeight(frameCardHeight(0))).toBe(true);
    expect(isAutomaticFrameCardHeight(frameCardHeight(6) + 7)).toBe(false);
  });

  it("grows an automatic card to fit a paste, and never shrinks one", () => {
    const sixRows = frameCardHeight(6);
    expect(grownFrameCardHeight(sixRows, 9)).toBe(frameCardHeight(9));
    expect(grownFrameCardHeight(sixRows, 3)).toBeNull();
    expect(grownFrameCardHeight(sixRows, 6)).toBeNull();
  });

  it("does not resize a card someone has sized themselves", () => {
    expect(grownFrameCardHeight(frameCardHeight(6) + 7, 12)).toBeNull();
  });
});
