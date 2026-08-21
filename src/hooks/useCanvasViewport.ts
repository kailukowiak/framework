import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";
import {
  DEFAULT_CANVAS_ZOOM,
  clampCanvasZoom,
  wheelZoomFactor,
  zoomAnchoredScroll,
  type ZoomAnchor,
} from "../lib/canvasZoom";

/**
 * How far out the canvas is, and how much of it is on screen. Zoom is
 * session state rather than a preference — it's where you are looking, not
 * how you like the app set up, the same reason a scroll position is never
 * saved either.
 */
export function useCanvasViewport({
  canvasRef,
  documentOpened,
}: {
  canvasRef: RefObject<HTMLDivElement | null>;
  documentOpened: boolean;
}) {
  const [canvasZoom, setCanvasZoom] = useState(DEFAULT_CANVAS_ZOOM);
  // The wheel handler is a plain listener rather than a React one, so it
  // reads the zoom from here instead of from a closure that would be one
  // gesture stale.
  const canvasZoomRef = useRef(DEFAULT_CANVAS_ZOOM);
  const pendingZoomAnchor = useRef<ZoomAnchor | null>(null);

  const zoomCanvas = useCallback(
    (next: number, focus?: { x: number; y: number }) => {
      const from = canvasZoomRef.current;
      const to = clampCanvasZoom(next);
      if (to === from) return;
      const element = canvasRef.current;
      if (element) {
        // Several wheel events can land before React paints any of them, so
        // the anchor keeps the scroll and zoom it started from and only
        // moves its destination. Re-reading the scroll per event would
        // measure a viewport that has not been re-laid-out yet.
        const pending = pendingZoomAnchor.current;
        pendingZoomAnchor.current = {
          scrollLeft: pending?.scrollLeft ?? element.scrollLeft,
          scrollTop: pending?.scrollTop ?? element.scrollTop,
          // No pointer means the keyboard, and the keyboard zooms about the
          // middle of the screen — the part you are looking at.
          pointerX: focus?.x ?? element.clientWidth / 2,
          pointerY: focus?.y ?? element.clientHeight / 2,
          from: pending?.from ?? from,
          to,
        };
      }
      canvasZoomRef.current = to;
      setCanvasZoom(to);
    },
    [canvasRef]
  );

  // Before the paint, not after: correcting the scroll in a passive effect
  // shows one frame of the canvas at the new zoom and the old position, and
  // that frame reads as a jump.
  useLayoutEffect(() => {
    const anchor = pendingZoomAnchor.current;
    const element = canvasRef.current;
    if (!anchor || !element) return;
    pendingZoomAnchor.current = null;
    const { left, top } = zoomAnchoredScroll(anchor);
    element.scrollLeft = left;
    element.scrollTop = top;
  }, [canvasRef, canvasZoom]);

  useEffect(() => {
    const element = canvasRef.current;
    if (!element) return;
    // Non-passive on purpose. A ctrl-wheel that reaches the webview zooms the
    // whole app, and this app already drives that zoom from Preferences — the
    // two would fight over one gesture and neither would win cleanly.
    //
    // Trackpad pinch arrives here too: macOS delivers it as a wheel event
    // with ctrlKey set, so the pinch and the modifier are one code path.
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      const bounds = element.getBoundingClientRect();
      zoomCanvas(
        canvasZoomRef.current * wheelZoomFactor(event.deltaY, event.deltaMode),
        { x: event.clientX - bounds.left, y: event.clientY - bounds.top }
      );
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [canvasRef, documentOpened, zoomCanvas]);

  // How much canvas is on screen at once. The canvas sizes itself to the
  // cards plus a share of this, so the lowest card can be scrolled into the
  // middle of the viewport instead of stopping against the bottom of the
  // scroll range.
  const [viewportSize, setViewportSize] = useState({ width: 1200, height: 800 });
  useEffect(() => {
    const element = canvasRef.current;
    if (!element) return;
    const measure = () =>
      setViewportSize((current) =>
        current.width === element.clientWidth && current.height === element.clientHeight
          ? current
          : { width: element.clientWidth, height: element.clientHeight }
      );
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
    // The viewport only exists once the document has opened, so the
    // observer is attached then rather than on the first render.
  }, [canvasRef, documentOpened]);

  return { canvasZoom, canvasZoomRef, zoomCanvas, viewportSize };
}
