import { useEffect, useRef, useState } from "react";
import { reorderColumnIds } from "./lib/pipelineStepCommands";
import type { FrameObject } from "./lib/types";
import type { GridRange } from "./lib/gridNavigation";
import { formulaToken } from "./lib/formulaReferences";
import {
  dispatchVectorDrop,
  updateVectorDragPreview,
  type VectorDrag,
} from "./lib/vectorDrag";

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
  rowCount: number,
  selectionRange: GridRange | null,
  onRearrangeColumns: (frameId: string, columnIds: string[]) => void,
  onJoinColumns: (
    primaryFrameId: string,
    primaryColumnId: string,
    lookupFrameId: string,
    lookupOutputColumnIds: string[]
  ) => void
) {
  const draggingFrameColumnRef = useRef<string | null>(null);
  const [, setDraggingFrameColumn] = useState<string | null>(null);
  const [frameColumnDrop, setFrameColumnDrop] = useState<{
    columnId: string;
    after: boolean;
  } | null>(null);

  const beginFrameColumnDrag = (event: React.PointerEvent, columnId: string) => {
    if (event.button !== 0) return;
    const start = { x: event.clientX, y: event.clientY };
    const card = event.currentTarget.closest<HTMLElement>(".canvas-object");
    let moved = false;
    let latestDrop: { columnId: string; after: boolean } | null = null;
    let latestJoin: { frameId: string; columnId: string } | null = null;
    let latestVectorTarget: HTMLElement | null = null;
    let highlighted: HTMLElement | null = null;
    let preview: HTMLElement | null = null;
    const clearHighlight = () => {
      highlighted?.classList.remove("join-column-drop", "vector-column-drop");
      highlighted = null;
    };
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
      const under = document.elementFromPoint(moveEvent.clientX, moveEvent.clientY);
      const vectorTarget = under?.closest<HTMLElement>("[data-vector-drop-target]");
      if (vectorTarget) {
        latestDrop = null;
        latestJoin = null;
        latestVectorTarget = vectorTarget;
        clearHighlight();
        highlighted = vectorTarget;
        highlighted.classList.add("vector-column-drop");
        const payload = frameColumnVector(frame, columnId, rowCount);
        if (payload)
          preview = updateVectorDragPreview(preview, payload, moveEvent, vectorTarget);
        setFrameColumnDrop(null);
        return;
      }
      latestVectorTarget = null;
      preview?.remove();
      preview = null;
      const target = under?.closest<HTMLElement>(".column-header[data-column-id]");
      const targetCard = target?.closest<HTMLElement>(".canvas-object[data-object-id]");
      if (!target || !targetCard) {
        latestDrop = null;
        latestJoin = null;
        clearHighlight();
        setFrameColumnDrop(null);
        return;
      }
      if (targetCard !== card) {
        latestDrop = null;
        latestJoin = {
          frameId: targetCard.dataset.objectId!,
          columnId: target.dataset.columnId!,
        };
        clearHighlight();
        highlighted = target;
        highlighted.classList.add("join-column-drop");
        setFrameColumnDrop(null);
        return;
      }
      latestJoin = null;
      clearHighlight();
      const bounds = target.getBoundingClientRect();
      latestDrop = {
        columnId: target.dataset.columnId!,
        after: moveEvent.clientX >= bounds.left + bounds.width / 2,
      };
      setFrameColumnDrop(latestDrop);
    };
    const end = (endEvent: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      if (moved && latestVectorTarget) {
        const payload = frameColumnVector(frame, columnId, rowCount);
        if (payload) dispatchVectorDrop(latestVectorTarget, payload, endEvent);
      } else if (moved && latestDrop) {
        const ordered = reorderColumnIds(
          frame.columns.map((column) => column.id),
          columnId,
          latestDrop.columnId,
          latestDrop.after
        );
        if (ordered.some((id, index) => id !== frame.columns[index]?.id))
          onRearrangeColumns(frame.id, ordered);
      } else if (moved && latestJoin) {
        onJoinColumns(
          latestJoin.frameId,
          latestJoin.columnId,
          frame.id,
          lookupDragColumnIds(frame, columnId, selectionRange)
        );
      }
      draggingFrameColumnRef.current = null;
      setDraggingFrameColumn(null);
      setFrameColumnDrop(null);
      clearHighlight();
      preview?.remove();
    };
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };

  return { frameColumnDrop, beginFrameColumnDrag };
}

/** The same canonical frame-column address completion inserts when typed. */
export function frameColumnVector(
  frame: Pick<FrameObject, "id" | "name" | "columns">,
  columnId: string,
  rowCount: number
): VectorDrag | null {
  const column = frame.columns.find((candidate) => candidate.id === columnId);
  if (!column || rowCount < 1) return null;
  return {
    objectId: column.id,
    name: column.name,
    formula: `${formulaToken(frame.name)}.${formulaToken(column.name)}`,
    length: rowCount,
  };
}

/** A selected header run means "bring these columns" when dragged away. */
export function lookupDragColumnIds(
  frame: Pick<FrameObject, "columns">,
  draggedColumnId: string,
  selectionRange: GridRange | null
): string[] {
  const draggedIndex = frame.columns.findIndex((column) => column.id === draggedColumnId);
  if (
    draggedIndex < 0 ||
    !selectionRange ||
    draggedIndex < selectionRange.left ||
    draggedIndex > selectionRange.right
  )
    return [draggedColumnId];
  return frame.columns
    .slice(selectionRange.left, selectionRange.right + 1)
    .map((column) => column.id);
}
