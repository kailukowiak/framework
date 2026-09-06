import { describe, expect, it } from "vitest";
import { CARD_SIZES, frameCardSize, placeNewCard, type Rect } from "./cardPlacement";

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
    expect(capped.height).toBeLessThanOrEqual(600);
    expect(capped.height).toBe(frameCardSize(40, 12).height);
  });
});
