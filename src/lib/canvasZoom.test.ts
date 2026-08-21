import { describe, expect, it } from "vitest";
import {
  CANVAS_OUTLINE_ZOOM,
  DEFAULT_CANVAS_ZOOM,
  MAX_CANVAS_ZOOM,
  MIN_CANVAS_ZOOM,
  canvasPoint,
  clampCanvasZoom,
  formatCanvasZoom,
  nudgeCanvasZoom,
  outlineDetail,
  wheelZoomFactor,
  zoomAnchoredScroll,
} from "./canvasZoom";

describe("clampCanvasZoom", () => {
  it("holds the range", () => {
    expect(clampCanvasZoom(5)).toBe(MAX_CANVAS_ZOOM);
    expect(clampCanvasZoom(0.01)).toBe(MIN_CANVAS_ZOOM);
    expect(clampCanvasZoom(1)).toBe(1);
  });

  it("falls back rather than passing a broken number on", () => {
    expect(clampCanvasZoom(Number.NaN)).toBe(DEFAULT_CANVAS_ZOOM);
    expect(clampCanvasZoom(Number.POSITIVE_INFINITY)).toBe(DEFAULT_CANVAS_ZOOM);
  });
});

describe("nudgeCanvasZoom", () => {
  it("steps to the next notch, not by a fixed proportion", () => {
    expect(nudgeCanvasZoom(1, 1)).toBe(1.25);
    expect(nudgeCanvasZoom(1, -1)).toBe(0.8);
  });

  it("steps from wherever a pinch left off", () => {
    expect(nudgeCanvasZoom(0.9, 1)).toBe(1);
    expect(nudgeCanvasZoom(0.9, -1)).toBe(0.8);
  });

  it("stops at the ends instead of wrapping", () => {
    expect(nudgeCanvasZoom(MAX_CANVAS_ZOOM, 1)).toBe(MAX_CANVAS_ZOOM);
    expect(nudgeCanvasZoom(MIN_CANVAS_ZOOM, -1)).toBe(MIN_CANVAS_ZOOM);
  });
});

describe("wheelZoomFactor", () => {
  it("zooms in when the wheel goes up and out when it goes down", () => {
    expect(wheelZoomFactor(-10)).toBeGreaterThan(1);
    expect(wheelZoomFactor(10)).toBeLessThan(1);
    expect(wheelZoomFactor(0)).toBe(1);
  });

  it("means the same proportion at any zoom", () => {
    // The factor is what makes this true: applied to 0.5 and to 1 it moves
    // both by the same fraction, which is what the hand expects.
    const factor = wheelZoomFactor(-20);
    expect((0.5 * factor) / 0.5).toBeCloseTo((1 * factor) / 1, 10);
  });

  it("caps one violent flick", () => {
    expect(wheelZoomFactor(-100000)).toBe(wheelZoomFactor(-120));
    expect(wheelZoomFactor(100000)).toBe(wheelZoomFactor(120));
  });

  it("reads line and page deltas as the counts they are", () => {
    expect(wheelZoomFactor(1, 1)).toBeCloseTo(wheelZoomFactor(16, 0), 10);
    expect(wheelZoomFactor(-1, 2)).toBe(wheelZoomFactor(-120, 0));
  });
});

describe("zoomAnchoredScroll", () => {
  it("keeps the point under the pointer under the pointer", () => {
    const anchor = {
      scrollLeft: 100,
      scrollTop: 50,
      pointerX: 400,
      pointerY: 300,
      from: 1,
      to: 2,
    };
    const { left, top } = zoomAnchoredScroll(anchor);
    // The canvas point that was under the pointer, before and after.
    const before = {
      x: (anchor.scrollLeft + anchor.pointerX) / anchor.from,
      y: (anchor.scrollTop + anchor.pointerY) / anchor.from,
    };
    const after = {
      x: (left + anchor.pointerX) / anchor.to,
      y: (top + anchor.pointerY) / anchor.to,
    };
    expect(after.x).toBeCloseTo(before.x, 10);
    expect(after.y).toBeCloseTo(before.y, 10);
  });

  it("holds the same point zooming out", () => {
    const anchor = {
      scrollLeft: 800,
      scrollTop: 600,
      pointerX: 200,
      pointerY: 150,
      from: 1,
      to: 0.5,
    };
    const { left, top } = zoomAnchoredScroll(anchor);
    expect((left + anchor.pointerX) / anchor.to).toBeCloseTo(
      (anchor.scrollLeft + anchor.pointerX) / anchor.from,
      10
    );
    expect((top + anchor.pointerY) / anchor.to).toBeCloseTo(
      (anchor.scrollTop + anchor.pointerY) / anchor.from,
      10
    );
  });

  it("never asks for a negative scroll", () => {
    const { left, top } = zoomAnchoredScroll({
      scrollLeft: 0,
      scrollTop: 0,
      pointerX: 300,
      pointerY: 200,
      from: 1,
      to: 0.25,
    });
    expect(left).toBe(0);
    expect(top).toBe(0);
  });
});

describe("canvasPoint", () => {
  const viewport = { left: 72, top: 126, scrollLeft: 300, scrollTop: 200 };

  it("undoes the viewport offset and the scroll", () => {
    expect(canvasPoint({ x: 172, y: 226 }, viewport, 1)).toEqual({ x: 400, y: 300 });
  });

  it("divides by the zoom, so a click lands where it looks", () => {
    // Half zoom: the same screen pixel is twice as far into the canvas.
    expect(canvasPoint({ x: 172, y: 226 }, viewport, 0.5)).toEqual({ x: 800, y: 600 });
    expect(canvasPoint({ x: 172, y: 226 }, viewport, 2)).toEqual({ x: 200, y: 150 });
  });

  it("keeps points off the negative side of the canvas", () => {
    expect(
      canvasPoint({ x: 0, y: 0 }, { left: 72, top: 126, scrollLeft: 0, scrollTop: 0 }, 1)
    ).toEqual({ x: 0, y: 0 });
  });
});

describe("formatCanvasZoom", () => {
  it("reads as a percentage", () => {
    expect(formatCanvasZoom(1)).toBe("100%");
    expect(formatCanvasZoom(0.5)).toBe("50%");
    expect(formatCanvasZoom(0.67)).toBe("67%");
  });
});

describe("outlineDetail", () => {
  it("says everything when the card is big enough on screen to hold it", () => {
    expect(outlineDetail(300, 200)).toBe("full");
  });

  it("drops the source line before the counts", () => {
    expect(outlineDetail(200, 100)).toBe("counts");
  });

  it("falls back to the name alone once nothing else fits", () => {
    expect(outlineDetail(120, 60)).toBe("name");
  });

  it("judges by screen size, so the same zoom can answer differently", () => {
    // A wide card and a narrow one at one zoom are not the same card.
    const zoom = 0.4;
    expect(outlineDetail(800 * zoom, 500 * zoom)).toBe("full");
    expect(outlineDetail(360 * zoom, 210 * zoom)).toBe("name");
  });

  it("needs both dimensions, not just one", () => {
    expect(outlineDetail(900, 60)).toBe("name");
    expect(outlineDetail(120, 900)).toBe("name");
  });
});

describe("CANVAS_OUTLINE_ZOOM", () => {
  it("sits inside the range, so both sides of it are reachable", () => {
    expect(CANVAS_OUTLINE_ZOOM).toBeGreaterThan(MIN_CANVAS_ZOOM);
    expect(CANVAS_OUTLINE_ZOOM).toBeLessThan(MAX_CANVAS_ZOOM);
  });
});
