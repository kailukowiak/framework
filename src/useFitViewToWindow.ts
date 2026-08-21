import { useCallback, type RefObject } from "react";
import type { CanvasView, Operation } from "./lib/types";

const MIN_WIDTH = 360;
const MIN_HEIGHT = 210;

export function useFitViewToWindow(
  canvasRef: RefObject<HTMLDivElement | null>,
  zoomRef: RefObject<number>,
  onOperation: (operation: Operation) => Promise<string | null>
) {
  return useCallback(async (view: CanvasView) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const inset = 24;
    const width = Math.max(MIN_WIDTH, (canvas.clientWidth - inset * 2) / zoomRef.current);
    const height = Math.max(MIN_HEIGHT, (canvas.clientHeight - inset * 2) / zoomRef.current);
    if (view.collapsed)
      await onOperation({ type: "setViewCollapsed", viewId: view.id, collapsed: false });
    await onOperation({ type: "resizeView", viewId: view.id, width, height });
    canvas.scrollTo({
      left: Math.max(0, view.x * zoomRef.current - inset),
      top: Math.max(0, view.y * zoomRef.current - inset),
      behavior: "smooth",
    });
  }, [canvasRef, onOperation, zoomRef]);
}
