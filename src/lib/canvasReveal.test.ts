import { describe, expect, it } from "vitest";
import { revealScroll } from "./canvasReveal";

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
