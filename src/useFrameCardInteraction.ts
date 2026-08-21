import { useEffect, useRef, useState } from "react";
import { reorderColumnIds } from "./PipelineEditor";
import type { FrameObject } from "./lib/types";

export function useFrameScrollState() {
  const [scrollState, setScrollState] = useState({ top: 0, height: 300 });
  const scrollRef = useRef<HTMLDivElement>(null);
  const pendingScrollTop = useRef(0);
  const scrollFrame = useRef<number | null>(null);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;
    const updateHeight = () =>
      setScrollState((current) =>
        current.height === element.clientHeight
          ? current
          : { ...current, height: element.clientHeight }
      );
    const cancelPendingFrame = () => {
      if (scrollFrame.current !== null) cancelAnimationFrame(scrollFrame.current);
    };
    updateHeight();
    const observer = new ResizeObserver(updateHeight);
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelPendingFrame();
    };
  }, []);

  return {
    scrollState,
    setScrollState,
    scrollRef,
    pendingScrollTop,
    scrollFrame,
  };
}

export function useFrameColumnDrag(
  frame: FrameObject,
  onRearrangeColumns: (frameId: string, columnIds: string[]) => void
) {
  const draggingFrameColumnRef = useRef<string | null>(null);
  const [, setDraggingFrameColumn] = useState<string | null>(null);
  const [frameColumnDrop, setFrameColumnDrop] = useState<{
    columnId: string;
    after: boolean;
  } | null>(null);

  const beginFrameColumnDrag = (event: React.PointerEvent, columnId: string) => {
    if (event.button !== 0 || frame.columns.length < 2) return;
    const start = { x: event.clientX, y: event.clientY };
    const grid = event.currentTarget.closest("frame");
    let moved = false;
    let latestDrop: { columnId: string; after: boolean } | null = null;
    const move = (moveEvent: PointerEvent) => {
      if (
        !moved &&
        Math.hypot(moveEvent.clientX - start.x, moveEvent.clientY - start.y) < 3
      )
        return;
      moved = true;
      moveEvent.preventDefault();
      draggingFrameColumnRef.current = columnId;
      setDraggingFrameColumn(columnId);
      const target = document
        .elementFromPoint(moveEvent.clientX, moveEvent.clientY)
        ?.closest<HTMLElement>(".column-header[data-column-id]");
      if (!target || target.closest("frame") !== grid) {
        latestDrop = null;
        setFrameColumnDrop(null);
        return;
      }
      const bounds = target.getBoundingClientRect();
      latestDrop = {
        columnId: target.dataset.columnId!,
        after: moveEvent.clientX >= bounds.left + bounds.width / 2,
      };
      setFrameColumnDrop(latestDrop);
    };
    const end = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      if (moved && latestDrop) {
        const ordered = reorderColumnIds(
          frame.columns.map((column) => column.id),
          columnId,
          latestDrop.columnId,
          latestDrop.after
        );
        if (ordered.some((id, index) => id !== frame.columns[index]?.id))
          onRearrangeColumns(frame.id, ordered);
      }
      draggingFrameColumnRef.current = null;
      setDraggingFrameColumn(null);
      setFrameColumnDrop(null);
    };
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };

  return { frameColumnDrop, beginFrameColumnDrag };
}
