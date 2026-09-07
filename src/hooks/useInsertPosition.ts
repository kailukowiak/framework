import { useCallback, type RefObject } from "react";
import { CARD_SIZES, placeNewCardInViewport, type CardSize, type Rect } from "../lib/cardPlacement";
import { drawnCanvasCards } from "../lib/canvasCards";
import { minimalRevealScroll } from "../lib/canvasReveal";
import type { DocumentView } from "../lib/types";

/**
 * Where every "make me a card" gesture puts the card: in free space on the
 * screen the person is looking at.
 *
 * Two earlier versions of this are worth remembering, because each fixed
 * the other's bug. The first handed back a fixed point just inside the
 * viewport's corner, which dropped a new card on top of whatever already
 * sat there. The second ran that point through `placeNewCard`, which
 * stepped aside from collisions but could step anywhere on the canvas: ⌘J
 * over a full-screen table put the block below the table, and focusing its
 * editor scrolled the table off the screen — the table the tutorial was
 * about to ask you to click. `placeNewCardInViewport` keeps both promises
 * at once by bounding the search to what is visible.
 *
 * The rectangle is the visible region in canvas units (scroll and client
 * size over the zoom), measured against the cards the canvas actually draws
 * — see `drawnCanvasCards`. `size` defaults to a Block's footprint since
 * most no-argument callers (Scratchwork's block, in particular) are
 * creating one.
 */
export function useInsertPosition(
  document: DocumentView | null,
  canvasRef: RefObject<HTMLDivElement | null>,
  canvasZoomRef: RefObject<number>
) {
  const insertPosition = useCallback(
    (size: CardSize = CARD_SIZES.block) => {
      const zoom = canvasZoomRef.current;
      const canvas = canvasRef.current;
      const viewport: Rect = {
        x: (canvas?.scrollLeft ?? 0) / zoom,
        y: (canvas?.scrollTop ?? 0) / zoom,
        width: (canvas?.clientWidth ?? 1200) / zoom,
        height: (canvas?.clientHeight ?? 800) / zoom,
      };
      const cards = drawnCanvasCards(document);
      const point = placeNewCardInViewport(cards, viewport, size);
      // Choosing the spot and revealing it are one decision, so they are made
      // in one place: a canvas crowded enough to have no free screen leaves
      // the new card just off the edge, and a card you asked for that you
      // cannot see has not arrived. The scroll is the least one that shows it
      // — see `minimalRevealScroll` — and the cards on screen right now are
      // handed over so that "least" is measured against them: the thing you
      // were looking at when you pressed the key stays on screen. Instant
      // rather than smooth because the editor is focused a moment later, and
      // a focus landing mid-animation makes its own competing scroll.
      if (canvas) {
        const onScreen = cards.filter(
          (card) =>
            card.x < viewport.x + viewport.width &&
            card.x + card.width > viewport.x &&
            card.y < viewport.y + viewport.height &&
            card.y + card.height > viewport.y
        );
        const scroll = minimalRevealScroll(canvas, { ...point, ...size }, zoom, onScreen);
        if (scroll) canvas.scrollTo(scroll);
      }
      return point;
    },
    [canvasRef, canvasZoomRef, document]
  );
  return insertPosition;
}
