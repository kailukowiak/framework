import { useCallback, useEffect, useRef, useState, type PointerEvent, type RefObject } from "react";
import { automaticColumnBasis, readHeaderWidth, type ColumnWidthReader } from "../lib/columnBasis";
import { measureColumnFit, type ColumnFitMeasurer } from "../lib/columnFit";
import {
  clampColumnWidth,
  columnLayoutOperation,
  resizeTargets,
  storedColumnWidth,
} from "../lib/columnLayout";
import type { RecordsAsRowsFrameCardProps } from "../FrameCardProps";
import type { OperationHandler } from "../lib/handlers";
import type { FrameObject } from "../lib/types";

export type ColumnResize = {
  /** Widths of a drag or keyboard nudge in flight, by column id; null at rest. */
  preview: Record<string, number> | null;
  beginDrag: (event: PointerEvent, columnId: string) => void;
  fit: (columnId: string) => void;
  nudge: (columnId: string, delta: number, th: HTMLElement | null) => void;
  commitNudge: () => void;
};

/**
 * Resizing columns by hand: drag an edge, double-click it to fit, or step it
 * with the arrow keys.
 *
 * A drag previews locally and writes once, on release, so a gesture is one
 * undo entry and the document is not rewritten sixty times a second. The
 * preview is dropped only after the write settles — clearing it on release
 * would flash the old width for the round trip to the engine. When the
 * dragged column sits inside a whole-column selection the whole selection
 * takes the width, as in a spreadsheet; an unrelated column's edge resizes
 * only itself.
 *
 * The first resize on a table that sizes itself from its content measures
 * every column and writes them all with the one that changed (see
 * `automaticColumnBasis`), so the table never flips to fixed layout under
 * the person's hand and nothing else moves. Once widths are stored only the
 * columns being resized are written.
 *
 * `measure` and `measureWidth` are parameters because measuring needs layout,
 * which jsdom has not got; tests hand them numbers.
 */
export function useColumnResize({
  frame,
  selectedColumns,
  onOperation,
  scrollRef,
  measure = measureColumnFit,
  measureWidth = readHeaderWidth,
}: {
  frame: FrameObject;
  /** The columns of a whole-column selection, or null for any other selection. */
  selectedColumns: { left: number; right: number } | null;
  onOperation: OperationHandler;
  scrollRef: RefObject<HTMLDivElement | null>;
  measure?: ColumnFitMeasurer;
  measureWidth?: ColumnWidthReader;
}): ColumnResize {
  const [preview, setPreview] = useState<Record<string, number> | null>(null);
  const live = useRef({ frame, selectedColumns, onOperation });
  live.current = { frame, selectedColumns, onOperation };
  const pending = useRef<Record<string, number> | null>(null);
  const dragCleanup = useRef<(() => void) | null>(null);
  useEffect(() => () => dragCleanup.current?.(), []);

  const commit = useCallback((widths: Record<string, number>) => {
    const { frame: current, onOperation: send } = live.current;
    const changes = Object.entries(widths).map(([columnId, width]) => ({
      columnId,
      width,
    }));
    if (changes.length === 0) return Promise.resolve();
    return Promise.resolve(send(columnLayoutOperation(current, changes))).then(
      () => undefined
    );
  }, []);

  const targetsOf = (columnId: string) =>
    resizeTargets(live.current.frame, columnId, live.current.selectedColumns);

  const basisOf = () =>
    automaticColumnBasis(
      live.current.frame,
      scrollRef.current?.querySelector("table"),
      measureWidth
    );

  const beginDrag = (event: PointerEvent, columnId: string) => {
    if (event.button !== 0) return;
    // The header cell under the handle selects its column on pointer down and
    // the name button below starts a rearrange drag; this is neither.
    event.preventDefault();
    event.stopPropagation();
    const th = (event.currentTarget as HTMLElement).closest("th");
    const startX = event.clientX;
    const startWidth = th?.offsetWidth || storedColumnWidth(live.current.frame, columnId) || 0;
    const rendered = th?.getBoundingClientRect().width ?? 0;
    // The canvas can be zoomed: a pixel of pointer travel is less than a
    // layout pixel, exactly as the summary drawer's resize accounts for.
    const scale = th?.offsetWidth && rendered ? rendered / th.offsetWidth : 1;
    const targets = targetsOf(columnId);
    const basis = basisOf();
    let latest: Record<string, number> | null = null;
    const move = (moveEvent: PointerEvent | globalThis.PointerEvent) => {
      const width = clampColumnWidth(startWidth + (moveEvent.clientX - startX) / scale);
      latest = { ...basis, ...Object.fromEntries(targets.map((id) => [id, width])) };
      setPreview(latest);
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", cancel);
      dragCleanup.current = null;
    };
    const end = () => {
      stop();
      if (!latest) return;
      void commit(latest).finally(() => setPreview(null));
    };
    const cancel = () => {
      stop();
      setPreview(null);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", cancel);
    dragCleanup.current = stop;
  };

  const fit = (columnId: string) => {
    const table = scrollRef.current?.querySelector("table");
    if (!table) return;
    const fitted: Record<string, number> = {};
    for (const id of targetsOf(columnId)) {
      const natural = measure(table, id);
      if (natural !== null) fitted[id] = clampColumnWidth(natural);
    }
    if (Object.keys(fitted).length === 0) return;
    void commit({ ...basisOf(), ...fitted });
  };

  const nudge = (columnId: string, delta: number, th: HTMLElement | null) => {
    const base =
      pending.current?.[columnId] ??
      storedColumnWidth(live.current.frame, columnId) ??
      (th?.offsetWidth || 0);
    const width = clampColumnWidth(base + delta);
    pending.current = {
      ...(pending.current ?? basisOf()),
      ...Object.fromEntries(targetsOf(columnId).map((id) => [id, width])),
    };
    setPreview(pending.current);
  };

  const commitNudge = () => {
    const widths = pending.current;
    pending.current = null;
    if (!widths) return;
    void commit(widths).finally(() => setPreview(null));
  };

  return { preview, beginDrag, fit, nudge, commitNudge };
}

/**
 * `useColumnResize` for a records card. A whole-column selection is what
 * makes a resize apply to several columns; a cell range across the same
 * columns is not a request to resize them.
 */
export function useFrameColumnResize(
  model: Pick<
    RecordsAsRowsFrameCardProps,
    "frame" | "gridFocus" | "selectionRange" | "onOperation"
  >,
  scrollRef: RefObject<HTMLDivElement | null>
): ColumnResize {
  const range = model.selectionRange;
  return useColumnResize({
    frame: model.frame,
    selectedColumns:
      model.gridFocus?.span === "column" && range
        ? { left: range.left, right: range.right }
        : null,
    onOperation: model.onOperation,
    scrollRef,
  });
}
