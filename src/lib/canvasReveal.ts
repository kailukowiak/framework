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
