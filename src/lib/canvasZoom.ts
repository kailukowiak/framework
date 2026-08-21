// Zooming the canvas, which is not the same thing as zooming the app.
//
// The interface scale in Preferences resizes the whole window — chrome,
// panels, type — because a screen or a pair of eyes needs everything bigger.
// It is set once and forgotten. This is the other thing entirely: the canvas
// is a space the document is laid out in, and zooming it is how you see more
// of that space at once. Cards keep their size in canvas units; only the view
// of them changes.
//
// Everything here is arithmetic on purpose. The scroll maths in particular is
// the difference between a zoom that keeps your place and one that throws it
// away, and that is worth being able to test without a browser.

export const MIN_CANVAS_ZOOM = 0.25;
export const MAX_CANVAS_ZOOM = 2;
export const DEFAULT_CANVAS_ZOOM = 1;

/**
 * Below this a card stops drawing its rows and says what it is instead.
 *
 * Set where a 14px cell lands on 7px of screen: past that the grid is texture
 * rather than text, and the card's space buys more as a label than as an
 * unreadable picture of a frame.
 */
export const CANVAS_OUTLINE_ZOOM = 0.5;

/** The notches Cmd+/- steps through, chosen to read as round percentages. */
const ZOOM_STOPS = [0.25, 0.33, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2];

/** Floating-point slack, so a stop equal to the current zoom is not "past" it. */
const EPSILON = 1e-6;

export function clampCanvasZoom(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_CANVAS_ZOOM;
  return Math.min(MAX_CANVAS_ZOOM, Math.max(MIN_CANVAS_ZOOM, value));
}

/**
 * The next notch up or down.
 *
 * Stepping through fixed stops rather than multiplying by a constant: a
 * keyboard zoom is read off the screen as a percentage, and 121% is not a
 * number anybody wants to land on.
 */
export function nudgeCanvasZoom(value: number, direction: 1 | -1): number {
  const current = clampCanvasZoom(value);
  const ordered = direction > 0 ? ZOOM_STOPS : [...ZOOM_STOPS].reverse();
  const next = ordered.find((stop) =>
    direction > 0 ? stop > current + EPSILON : stop < current - EPSILON
  );
  return next ?? current;
}

/**
 * What one wheel or pinch event multiplies the zoom by.
 *
 * Exponential rather than additive, so a notch means the same proportion at
 * every zoom — going 0.5 → 0.55 feels like 1 → 1.1, which is what the hand
 * expects. The clamp keeps one violent flick of a mouse wheel from crossing
 * most of the range in a single event.
 */
export function wheelZoomFactor(deltaY: number, deltaMode = 0): number {
  // Line and page deltas are counts, not pixels; a line is about a row of
  // text and a page about a screen of them.
  const pixels = deltaMode === 1 ? deltaY * 16 : deltaMode === 2 ? deltaY * 400 : deltaY;
  const bounded = Math.max(-120, Math.min(120, pixels));
  return Math.exp(-bounded * 0.0025);
}

/** A zoom in progress: where the view was, and the point that must not move. */
export interface ZoomAnchor {
  scrollLeft: number;
  scrollTop: number;
  /** The fixed point, in pixels from the viewport's top-left corner. */
  pointerX: number;
  pointerY: number;
  from: number;
  to: number;
}

/**
 * Where to scroll so the point under the pointer stays under the pointer.
 *
 * Zooming around the viewport's top-left corner instead is the single thing
 * that makes a canvas zoom feel broken: the content you were looking at slides
 * off the screen every time you change the magnification.
 */
export function zoomAnchoredScroll(anchor: ZoomAnchor): { left: number; top: number } {
  const ratio = anchor.to / anchor.from;
  return {
    left: Math.max(0, (anchor.scrollLeft + anchor.pointerX) * ratio - anchor.pointerX),
    top: Math.max(0, (anchor.scrollTop + anchor.pointerY) * ratio - anchor.pointerY),
  };
}

/**
 * A point on the canvas, from a point on the screen.
 *
 * Every placement the canvas does — insert here, drop the tab there, open the
 * menu about this spot — starts as a client coordinate and has to end as a
 * canvas one. There is one of these rather than seven copies because the
 * seventh copy is where the zoom division gets forgotten.
 */
export function canvasPoint(
  client: { x: number; y: number },
  viewport: { left: number; top: number; scrollLeft: number; scrollTop: number },
  zoom: number
): { x: number; y: number } {
  return {
    x: Math.max(0, (client.x - viewport.left + viewport.scrollLeft) / zoom),
    y: Math.max(0, (client.y - viewport.top + viewport.scrollTop) / zoom),
  };
}

/** How much of itself a card can say, given how big it actually is on screen. */
export type OutlineDetail = "full" | "counts" | "name";

/**
 * What still fits.
 *
 * An outline's type holds its size on screen while the card shrinks under it,
 * so the writing takes up more and more of the card the further out the canvas
 * goes. This is where that has to stop. A line that no longer fits is dropped
 * rather than shrunk — half a legible line beats four unreadable ones — and
 * the last card standing shows the one fact always worth the space: its name.
 *
 * Measured in screen pixels, not canvas units, because legibility is a
 * property of the screen. A big card and a small one at the same zoom get
 * different answers, which is the point.
 */
export function outlineDetail(
  screenWidth: number,
  screenHeight: number
): OutlineDetail {
  if (screenWidth >= 240 && screenHeight >= 132) return "full";
  if (screenWidth >= 170 && screenHeight >= 84) return "counts";
  return "name";
}

/** Formats a zoom for display: "75%". */
export function formatCanvasZoom(value: number): string {
  return `${Math.round(clampCanvasZoom(value) * 100)}%`;
}
