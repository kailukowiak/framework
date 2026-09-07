import { describe, expect, it } from "vitest";
import { contentOriginScroll, minimalRevealScroll, revealScroll } from "./canvasReveal";

const viewport = { scrollLeft: 0, scrollTop: 0, clientWidth: 1000, clientHeight: 800 };

describe("revealScroll", () => {
  it("leaves a card that is already in view where it is", () => {
    expect(revealScroll(viewport, { x: 100, y: 100, width: 300, height: 200 }, 1)).toBeNull();
  });

  it("centres a card that is off screen", () => {
    expect(revealScroll(viewport, { x: 2000, y: 1500, width: 300, height: 200 }, 1)).toEqual({
      left: 2000 - 350,
      top: 1500 - 300,
    });
  });

  it("pins a card larger than the viewport to its corner", () => {
    expect(revealScroll(viewport, { x: 2000, y: 100, width: 1400, height: 200 }, 1)).toEqual({
      left: 2000 - 24,
      top: 0,
    });
  });

  it("measures in screen pixels at the current zoom", () => {
    expect(revealScroll(viewport, { x: 1500, y: 100, width: 300, height: 200 }, 0.5)).toBeNull();
  });
});

describe("contentOriginScroll", () => {
  it("is the origin for a document with nothing on the canvas", () => {
    expect(contentOriginScroll([], 1)).toEqual({ left: 0, top: 0 });
  });

  it("shows the top-left card of a document whose cards start near the corner", () => {
    const cards = [
      { x: 110, y: 100, width: 420, height: 300 },
      { x: 900, y: 640, width: 420, height: 300 },
    ];
    // The margin off the very edge, never a negative offset.
    expect(contentOriginScroll(cards, 1)).toEqual({ left: 86, top: 76 });
  });

  it("follows a document laid out far from the origin", () => {
    const cards = [{ x: 2000, y: 1500, width: 420, height: 300 }];
    expect(contentOriginScroll(cards, 1)).toEqual({ left: 1976, top: 1476 });
  });

  it("is measured in screen pixels, so it follows the zoom", () => {
    const cards = [{ x: 1000, y: 1000, width: 420, height: 300 }];
    expect(contentOriginScroll(cards, 0.5)).toEqual({ left: 476, top: 476 });
  });

  it("clamps to the origin rather than scrolling negative", () => {
    expect(contentOriginScroll([{ x: 0, y: 0, width: 10, height: 10 }], 1)).toEqual({
      left: 0,
      top: 0,
    });
  });
});

describe("minimalRevealScroll", () => {
  it("leaves a card that is already comfortably in view alone", () => {
    expect(
      minimalRevealScroll(viewport, { x: 100, y: 100, width: 300, height: 200 }, 1)
    ).toBeNull();
  });

  it("moves only as far as it must, rather than centring", () => {
    // A card just past the bottom edge: the least scroll puts its bottom edge
    // a margin inside the window, and leaves the horizontal scroll untouched.
    const scroll = minimalRevealScroll(
      viewport,
      { x: 100, y: 900, width: 600, height: 220 },
      1
    );
    expect(scroll).toEqual({ left: 0, top: 900 + 220 + 24 - 800 });
  });

  it("keeps what was on screen on screen — the ⌘J case", () => {
    // The tour's Start workbook: the table sits at y 70..370 and the block
    // lands flush under the walkthrough card at y 890. Centring took the table off
    // the top; the least scroll leaves it in view.
    const scroll = minimalRevealScroll(
      { scrollLeft: 46, scrollTop: 46, clientWidth: 1100, clientHeight: 860 },
      { x: 70, y: 890, width: 600, height: 220 },
      1
    );
    expect(scroll).toEqual({ left: 46, top: 274 });
    expect(scroll!.top).toBeLessThan(370);
  });

  it("gives the cards already on screen the last word — the crowded ⌘J case", () => {
    // The harness's real window: 1128×758, the tour's three cards all on
    // screen, and the block's only home under the walkthrough card at y 890.
    // Showing the whole block wants 376, which is exactly the sales table's
    // bottom edge; the table keeps 60px instead and the block shows its top.
    const tour = [
      { x: 70, y: 70, width: 580, height: 820 },
      { x: 720, y: 70, width: 420, height: 306 },
      { x: 720, y: 500, width: 420, height: 450 },
    ];
    const scroll = minimalRevealScroll(
      { scrollLeft: 0, scrollTop: 0, clientWidth: 1128, clientHeight: 758 },
      { x: 70, y: 890, width: 600, height: 220 },
      1,
      tour
    );
    expect(scroll).toEqual({ left: 0, top: 376 - 60 });
    // The block's first lines are on screen even so.
    expect(890 + 120).toBeLessThanOrEqual(scroll!.top + 758);
  });

  it("never lets a kept card push the new card entirely off screen", () => {
    // A kept card that ends high up would cap the scroll below what shows
    // the new card at all; the new card's headroom is the floor.
    const scroll = minimalRevealScroll(
      { scrollLeft: 0, scrollTop: 0, clientWidth: 1000, clientHeight: 500 },
      { x: 0, y: 900, width: 600, height: 220 },
      1,
      [{ x: 0, y: 0, width: 300, height: 100 }]
    );
    expect(scroll).toEqual({ left: 0, top: 900 + 120 - 500 });
  });

  it("pins a card larger than the viewport to its leading edge", () => {
    expect(
      minimalRevealScroll(viewport, { x: 200, y: 200, width: 2000, height: 1600 }, 1)
    ).toEqual({ left: 176, top: 176 });
  });

  it("measures in screen pixels at the current zoom", () => {
    const scroll = minimalRevealScroll(
      viewport,
      { x: 100, y: 500, width: 300, height: 200 },
      2
    );
    expect(scroll).toEqual({ left: 0, top: 1000 + 400 + 24 - 800 });
  });

  it("never scrolls to a negative offset", () => {
    const scroll = minimalRevealScroll(
      { scrollLeft: 100, scrollTop: 100, clientWidth: 1000, clientHeight: 800 },
      { x: 0, y: 0, width: 300, height: 200 },
      1
    );
    expect(scroll).toEqual({ left: 0, top: 0 });
  });
});
