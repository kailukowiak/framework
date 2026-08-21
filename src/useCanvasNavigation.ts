import { useCallback, type Dispatch, type RefObject, type SetStateAction } from "react";
import {
  canvasNavigationTarget,
  selectedCanvasView,
  type CanvasNavigationDirection,
} from "./lib/canvasNavigation";
import type { GridFocus } from "./FrameGrid";
import type { DocumentView, Selection } from "./lib/types";

type CanvasNavigationOptions = {
  document: DocumentView | null;
  selection: Selection | null;
  containedIds: ReadonlySet<string>;
  canvasRef: RefObject<HTMLDivElement | null>;
  canvasZoomRef: RefObject<number>;
  setSelection: Dispatch<SetStateAction<Selection | null>>;
  setGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
};

export function useCanvasNavigation(options: CanvasNavigationOptions) {
  const {
    document, selection, containedIds, canvasRef, canvasZoomRef, setSelection, setGridFocus,
  } = options;
  return useCallback(
    (direction: CanvasNavigationDirection) => {
      const current = selectedCanvasView(document, selection);
      if (!document || !current) return false;
      const target = canvasNavigationTarget(
        document.views.filter((view) => !containedIds.has(view.objectId)),
        current.id,
        direction
      );
      if (!target) return false;
      setGridFocus(null);
      setSelection({ objectId: target.objectId, viewId: target.id });
      const zoom = canvasZoomRef.current ?? 1;
      canvasRef.current?.scrollTo({
        left: Math.max(0, target.x * zoom - 120),
        top: Math.max(0, target.y * zoom - 80),
        behavior: "smooth",
      });
      return true;
    },
    [canvasRef, canvasZoomRef, containedIds, document, selection, setGridFocus, setSelection]
  );
}
