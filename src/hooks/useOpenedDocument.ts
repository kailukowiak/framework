import { useCallback, useLayoutEffect, useState, type RefObject } from "react";
import { drawnCanvasCards } from "../lib/canvasCards";
import { contentOriginScroll } from "../lib/canvasReveal";
import type { DocumentView } from "../lib/types";

/**
 * Everything that has to be true the moment a different document arrives in
 * this window, in one place.
 *
 * There are five ways in — the Data library, a recent, File ▸ Open, New, and
 * the events the desktop shell sends when the Finder or a second launch hands
 * this window a file — and each had grown its own list of things to clear.
 * The lists had drifted, and what was left over was not harmless:
 *
 * - The canvas kept the previous document's scroll offset. A scroll position
 *   belongs to a window rather than to a file, and nothing saves one, so
 *   opening the tour's Start workbook straight after its Answer key — which
 *   had been scrolled right and down — showed a document whose main table was
 *   off screen before a single gesture.
 * - `scratchFocus` kept the previous document's ⌘J request. Its null block id
 *   means "the newest block", which in the new document is a different block
 *   entirely, so a workbook opened with that block's editor already active and
 *   the formula bar announcing a pointing session nobody had started.
 *
 * A document opens looking at its own contents, with nothing selected and no
 * editor running. One list, five callers.
 */
export function useOpenedDocument({
  document,
  canvasRef,
  canvasZoomRef,
  reset,
}: {
  document: DocumentView | null;
  canvasRef: RefObject<HTMLDivElement | null>;
  canvasZoomRef: RefObject<number>;
  /** The window state a document open clears, as its owner spells it. */
  reset: (opened: { document: DocumentView; path: string | null }) => void;
}) {
  /** Where the canvas should jump to once the new document has drawn. */
  const [openedViewport, setOpenedViewport] = useState<{
    left: number;
    top: number;
  } | null>(null);

  const adoptOpenedDocument = useCallback(
    (opened: { document: DocumentView; path: string | null }) => {
      reset(opened);
      setOpenedViewport(
        contentOriginScroll(drawnCanvasCards(opened.document), canvasZoomRef.current)
      );
    },
    [canvasZoomRef, reset]
  );

  // Applied after the new document has laid its cards out, since the canvas
  // has no scroll range to move within until it has. Deliberately not a
  // smooth scroll: this is where the document begins, not somewhere it
  // travelled to.
  useLayoutEffect(() => {
    if (!openedViewport) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.scrollLeft = openedViewport.left;
    canvas.scrollTop = openedViewport.top;
    setOpenedViewport(null);
    // `document` is a dependency so that an open arriving before the canvas
    // is mounted — the first one, behind the splash — is retried once it is.
  }, [canvasRef, openedViewport, document]);

  return adoptOpenedDocument;
}
