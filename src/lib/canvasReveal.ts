/**
 * Where to scroll so a card is on screen, or nothing if it already is.
 *
 * Jumping to a card used to park its top-left corner near the viewport's
 * corner regardless of where it was, so ⌘J on a block that was in plain view
 * still slid the canvas, and a block the size of a postcard ended up alone in
 * the top-left of an otherwise empty screen. A card already fully visible is
 * left where it is; one that is not is centred, or pinned to the corner when
 * it is bigger than the viewport.
 */
export interface Viewport {
  scrollLeft: number;
  scrollTop: number;
  clientWidth: number;
  clientHeight: number;
}

export interface CanvasRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const MARGIN = 24;

export function revealScroll(
  viewport: Viewport,
  card: CanvasRect,
  zoom: number
): { left: number; top: number } | null {
  const left = card.x * zoom;
  const top = card.y * zoom;
  const width = card.width * zoom;
  const height = card.height * zoom;
  const visible =
    left >= viewport.scrollLeft + MARGIN &&
    top >= viewport.scrollTop + MARGIN &&
    left + width <= viewport.scrollLeft + viewport.clientWidth - MARGIN &&
    top + height <= viewport.scrollTop + viewport.clientHeight - MARGIN;
  if (visible) return null;
  const centre = (start: number, size: number, extent: number) =>
    size + 2 * MARGIN >= extent
      ? Math.max(0, start - MARGIN)
      : Math.max(0, start - (extent - size) / 2);
  return {
    left: centre(left, width, viewport.clientWidth),
    top: centre(top, height, viewport.clientHeight),
  };
}

/**
 * Where to scroll so a card is on screen, moving as little as possible.
 *
 * `revealScroll` centres, which is right when you are *going* somewhere — a
 * jump names a card and the card is the point. It is wrong when a card has
 * just been made beside you: ⌘J on the tour's Start workbook has no free
 * screen to put a block on, so the block lands under the text card, and
 * centring it there carried both tables off the top of the window. The thing
 * you were looking at when you pressed the key is still the thing you are
 * working on; the new card has to become visible without becoming the only
 * visible thing.
 *
 * So each axis moves only as far as it must, and not at all if that edge is
 * already comfortable. A card bigger than the viewport pins its leading edge
 * — there is no scroll that shows all of it, and its top-left is the part
 * you want.
 */
export function minimalRevealScroll(
  viewport: Viewport,
  card: CanvasRect,
  zoom: number,
  keep: readonly CanvasRect[] = []
): { left: number; top: number } | null {
  const axis = (start: number, size: number, scroll: number, extent: number) => {
    const showLeading = Math.max(0, start - MARGIN);
    if (size + 2 * MARGIN >= extent) return showLeading;
    const showTrailing = Math.max(0, start + size + MARGIN - extent);
    if (scroll > showLeading) return showLeading;
    if (scroll < showTrailing) return showTrailing;
    return scroll;
  };
  // Showing the whole new card and keeping the old ones can be at odds: on
  // the tour's Start workbook the block's only home is under the walkthrough
  // card, and the scroll that shows its bottom edge is, to the pixel, the
  // one that takes the sales table off the top. When they disagree the old
  // cards win — they are what the person was working on — and the new card
  // shows as much as it can, never less than its first lines, which is where
  // the typing goes. A kept card is held to KEEP pixels rather than "any":
  // a sliver you cannot read a header in has not been kept.
  const forward = (
    start: number,
    size: number,
    scroll: number,
    extent: number,
    ceilings: number[]
  ) => {
    const wanted = axis(start, size, scroll, extent);
    if (wanted <= scroll || !ceilings.length) return wanted;
    const floor = Math.max(scroll, start + HEADROOM - extent);
    return Math.max(floor, Math.min(wanted, ...ceilings));
  };
  const left = forward(
    card.x * zoom,
    card.width * zoom,
    viewport.scrollLeft,
    viewport.clientWidth,
    keep.map((rect) => (rect.x + rect.width) * zoom - KEEP)
  );
  const top = forward(
    card.y * zoom,
    card.height * zoom,
    viewport.scrollTop,
    viewport.clientHeight,
    keep.map((rect) => (rect.y + rect.height) * zoom - KEEP)
  );
  if (left === viewport.scrollLeft && top === viewport.scrollTop) return null;
  return { left, top };
}

/** How much of a kept card must stay on screen: a header and a row or two. */
const KEEP = 60;
/** How much of a new card must reach the screen: its name and first lines. */
const HEADROOM = 120;

/**
 * Where a freshly opened document should start: the top-left of what it
 * holds.
 *
 * A scroll position belongs to a window, not to a file — nothing saves one —
 * so a window that has been scrolled right and down keeps that offset when
 * the next document arrives in it, and the new document opens showing an
 * empty patch of canvas beside its own contents. Opening the tour's Start
 * workbook straight after its Answer key showed exactly that: a document
 * whose main table was off screen before a single gesture. Every open resets
 * to here instead.
 *
 * The corner of the cards rather than a flat (0, 0): a document laid out away
 * from the origin would otherwise open on the same empty canvas by a
 * different route. The margin leaves the topmost card off the very edge, and
 * the clamp keeps a document that starts at the origin from scrolling to a
 * negative offset it would silently round back anyway.
 */
export function contentOriginScroll(
  cards: readonly CanvasRect[],
  zoom: number
): { left: number; top: number } {
  if (cards.length === 0) return { left: 0, top: 0 };
  const x = Math.min(...cards.map((card) => card.x));
  const y = Math.min(...cards.map((card) => card.y));
  return {
    left: Math.max(0, x * zoom - MARGIN),
    top: Math.max(0, y * zoom - MARGIN),
  };
}
