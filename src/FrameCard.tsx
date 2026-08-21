import { useEffect, useMemo, useRef, useState } from "react";
import type { FrameCardProps } from "./FrameCardProps";
import { commitFrameCellEdit } from "./lib/commitFrameCellEdit";
import {
  FieldsAsRowsFrameCard,
  FramePageControls,
  FramePagedStatus,
} from "./FieldsAsRowsFrameCard";
import { RecordsAsRowsFrameCard } from "./RecordsAsRowsFrameCard";
import {
  useFrameColumnDrag,
  useFrameScrollState,
} from "./useFrameCardInteraction";
import {
  chainFilterCount,
  chainSteps,
  filterWeight,
  isEditableGridColumn,
  isTextEntryTarget,
  nextColumnName,
  pipelineSortKeys,
  frameStyleRules,
  rowsFromFramePage,
  styleMatchesFromFramePage,
  frameOrientation,
  type FrameStyleMatches,
  type GridFocusMode,
} from "./FrameGrid";
import { getFramePage, type FramePage } from "./lib/api";
import {
  normalizeRange,
  scrollLeftToRevealColumn,
  scrollTopToRevealRow,
  type GridDirection,
  type GridPosition,
} from "./lib/gridNavigation";
import { expandRangeForSpan } from "./lib/gridSpan";
import {
  PAGED_ROW_PAGE_SIZE,
  PagedRowCache,
  directionBetween,
  isStaleRequest,
  pageIndexForRow,
  pageRowOffset,
  pagedGenerationSignature,
  planPageFetch,
} from "./lib/pagedWindow";
import { calculateVirtualRowRange } from "./lib/frameVirtualization";

/**
 * One cached page: the values as they came back, and what the rules made of
 * each of those rows. Cached together because they are one read — a page
 * whose styles were fetched separately would draw the old colors on the new
 * rows for as long as the second request was in flight.
 */
type PagedRows = { values: string[][]; styleMatches: FrameStyleMatch[][] };
import type {
  Column,
  ComputedFrame,
  FrameStyleMatch,
  RenderedFrameStep,
  Row,
} from "./lib/types";

/** What a frame's authored chain does to its source, in three words or fewer. */
function frameTransformationLabels(computed: ComputedFrame): Array<string | null> {
  if (!computed.derivation) return [];
  const steps = chainSteps(computed);
  const filterCount = chainFilterCount(computed);
  const summarize = steps.find(
    (step): step is Extract<RenderedFrameStep, { kind: "summarize" }> =>
      step.kind === "summarize"
  );
  return [
    filterCount ? `${filterCount} filter${filterCount === 1 ? "" : "s"}` : null,
    steps.some((step) => step.kind === "join")
      ? "joined"
      : summarize?.aggregates.length
        ? summarize.groupKeys.length
          ? "grouped"
          : "total"
        : "linked",
    steps.some((step) => step.kind === "sort") ? "sorted" : null,
  ].filter(Boolean);
}

export function FrameCard({
  view,
  frame,
  computed,
  selection,
  gridFocus,
  onSelect,
  onGridFocus,
  onGridStep,
  onRenderedRows,
  onOperation,
  onRearrangeColumns,
  onFilterColumn,
  onTransformColumn,
  onEditCalculatedColumn,
  dataRefreshRevision,
}: FrameCardProps) {
  const isDerived = Boolean(frame.derivation);
  const isFileBacked = Boolean(computed.paged);
  // Asked, not inferred. The backend refuses these edits, so the grid has
  // no business having its own opinion about which ones it offers.
  const isReadOnly = !computed.editing.cells;
  // Three different questions that used to share one answer. A frame whose
  // values live in a parquet it owns takes typed values but grows neither a
  // row nor a column: those change the file's shape rather than one of its
  // values, and that is a different write than this one.
  const canAddRows = computed.editing.rows;
  const canAddColumns = !isReadOnly && !isFileBacked;
  // Fields-as-rows scrolls records horizontally through its own windowing
  // (see FieldsAsRowsFrameCard) rather than through the row virtualizer, so
  // it keeps the simpler single-page fetch below; only records-as-rows gets
  // scroll-driven windowed fetching.
  const isTransposed = frameOrientation(frame) === "fieldsAsRows";
  const transformationLabels = frameTransformationLabels(computed);
  const [draftRow, setDraftRow] = useState<Record<string, string>>({});
  const {
    scrollState,
    setScrollState,
    scrollRef,
    pendingScrollTop,
    scrollFrame,
  } = useFrameScrollState();
  const { frameColumnDrop, beginFrameColumnDrag } = useFrameColumnDrag(
    frame,
    onRearrangeColumns
  );
  const editCalculatedColumn = (column: Column, rowIndex: number) =>
    onEditCalculatedColumn(frame, column, rowIndex);
  const filterColumn = (column: Column) => onFilterColumn(frame, column);

  // --- Transposed (fields-as-rows) paging: unchanged single-page fetch + Previous/Next. ---
  const [pageOffset, setPageOffset] = useState(0);
  const [page, setPage] = useState<FramePage | null>(null);
  const [transposedPageLoading, setTransposedPageLoading] = useState(false);
  const [transposedPageError, setTransposedPageError] = useState<string | null>(null);
  const transposedPageSize = 200;

  useEffect(() => {
    setPageOffset(0);
    setPage(null);
    setTransposedPageError(null);
  }, [dataRefreshRevision, frame.id]);

  // Changing the predicate re-selects which rows exist at all, so a scroll
  // position or page offset taken under the old one addresses rows that are
  // gone. Left alone, the paged path plans its next fetch from a range past
  // the new end -- a wasted round trip and a flash of skeletons before the
  // returned count shrinks the content and the browser clamps the scroll.
  // Sorting and upstream refreshes keep their position: they reorder or
  // revalue the same rows, so where you are still means something.
  const filter = {
    predicates: chainSteps(computed).flatMap((step) =>
      step.kind === "filter" ? step.predicates : []
    ),
    matchAll: true,
  };
  const filterMark = filterWeight(computed);
  const sortKeys = pipelineSortKeys(computed);
  const filterSignature = useMemo(
    () => JSON.stringify([filter.predicates, filter.matchAll]),
    [filter.matchAll, filter.predicates]
  );
  const lastFilterSignatureRef = useRef(filterSignature);
  useEffect(() => {
    if (lastFilterSignatureRef.current === filterSignature) return;
    lastFilterSignatureRef.current = filterSignature;
    setPageOffset(0);
    // Set the scroll state alongside the element: assigning scrollTop only
    // emits an event when the position actually moves, and the handler
    // applies it a frame later, so the virtual range would otherwise plan
    // one more fetch from the stale offset first.
    pendingScrollTop.current = 0;
    if (scrollRef.current) scrollRef.current.scrollTop = 0;
    setScrollState((current) => (current.top === 0 ? current : { ...current, top: 0 }));
  }, [filterSignature, pendingScrollTop, scrollRef, setScrollState]);

  useEffect(() => {
    if (!isFileBacked || !isTransposed) return;
    let disposed = false;
    setTransposedPageLoading(true);
    setTransposedPageError(null);
    void getFramePage(frame.id, pageOffset, transposedPageSize)
      .then((nextPage) => {
        if (!disposed) setPage(nextPage);
      })
      .catch((reason) => {
        if (!disposed) setTransposedPageError(String(reason).replace(/^Error:\s*/, ""));
      })
      .finally(() => {
        if (!disposed) setTransposedPageLoading(false);
      });
    return () => {
      disposed = true;
    };
    // The frame's own lineage hash, for the same reason the paged generation
    // uses it: the fields-as-rows page is just as stale after a derivation
    // edit, and just as untouched by an edit somewhere else.
  }, [
    dataRefreshRevision,
    computed.fingerprint,
    isFileBacked,
    isTransposed,
    pageOffset,
    frame.id,
    filterSignature,
    sortKeys,
  ]);

  // --- Records-as-rows paging: scroll-driven windowed fetch over an LRU page cache. ---
  // Cache, in-flight set, last-settled position, and the debounce timer all
  // live in refs so they survive renders without themselves triggering one;
  // pagedCacheTick is the render trigger once new pages actually land.
  const pagedCacheRef = useRef(new PagedRowCache<PagedRows>());
  const pagedGenerationRef = useRef({ counter: 0, signature: "" });
  const pagedInFlightRef = useRef<Set<number>>(new Set());
  const pagedLastStartRef = useRef<number | null>(null);
  const pagedFetchTimerRef = useRef<number | null>(null);
  const [pagedTotalRows, setPagedTotalRows] = useState<number | null>(null);
  const [pagedLoading, setPagedLoading] = useState(false);
  const [pagedError, setPagedError] = useState<string | null>(null);
  const [pagedCacheTick, setPagedCacheTick] = useState(0);

  const pagedGeneration = useMemo(
    // This frame's lineage hash, not `dataRefreshRevision` and not the
    // document's revision. The first is too narrow: it counts only connector
    // refreshes and cache toggles, so an edit that changes what a page
    // contains -- adding a group key, retyping an aggregate, editing an
    // upstream cell -- left every cached page readable under it.
    //
    // The revision was too wide, which is worse to sit in front of: it moves
    // for every edit anywhere in the document, so typing a line in a
    // scratchpad threw away every page of every frame and fetched them all
    // again. The hash moves only when this frame's own lineage does.
    () =>
      pagedGenerationSignature({
        revision: computed.fingerprint,
        sort: sortKeys,
        filters: filter.predicates,
        // The rules ride in the signature even though they are not lineage:
        // they decide how a page is drawn, and the drawing arrives on the
        // page. Without them, editing a rule leaves every cached page
        // painted by the rule it replaced.
        styleRules: frameStyleRules(frame),
      }),
    [computed.fingerprint, filter.predicates, frame, sortKeys]
  );

  // A sort click, a filter change, or an upstream edit (dataRefreshRevision)
  // changes the generation signature: every previously cached page becomes
  // unreachable immediately (the cache's generation guard rejects it on
  // read), and any fetch already in flight under the old generation is
  // discarded when it resolves via the counter guard below. No stale row
  // is ever rendered.
  useEffect(() => {
    if (!isFileBacked || isTransposed) return;
    if (pagedGenerationRef.current.signature === pagedGeneration) return;
    pagedGenerationRef.current = {
      counter: pagedGenerationRef.current.counter + 1,
      signature: pagedGeneration,
    };
    pagedCacheRef.current.clear();
    pagedInFlightRef.current.clear();
    pagedLastStartRef.current = null;
    setPagedTotalRows(null);
    setPagedError(null);
    setPagedCacheTick((tick) => tick + 1);
  }, [isFileBacked, isTransposed, pagedGeneration]);

  const regularRows = frame.rows;
  // Null until something reports it: an artifact records its own count, but
  // a derived frame's is only known once a page has been read.
  const knownTotalRows = pagedTotalRows ?? computed.totalRows ?? null;
  const pagedTotalRowsResolved = knownTotalRows ?? 0;
  const totalRows = isFileBacked ? pagedTotalRowsResolved : frame.rows.length;
  const virtualRowCount =
    isFileBacked && !isTransposed ? totalRows : regularRows.length;

  const virtualRange = useMemo(
    () =>
      calculateVirtualRowRange(virtualRowCount, scrollState.top, scrollState.height),
    [virtualRowCount, scrollState.height, scrollState.top]
  );

  // Fetch whatever the settled virtual range needs, plus one page of
  // prefetch in the scroll direction. Debounced so a scrollbar fling only
  // fetches where the user lands, never every page it scrolled past.
  useEffect(() => {
    if (!isFileBacked || isTransposed) return;
    const generation = pagedGenerationRef.current;
    const startRow = virtualRange.start;
    const endRow = virtualRange.end;
    if (pagedFetchTimerRef.current !== null)
      window.clearTimeout(pagedFetchTimerRef.current);
    pagedFetchTimerRef.current = window.setTimeout(() => {
      pagedFetchTimerRef.current = null;
      const direction = directionBetween(pagedLastStartRef.current, startRow);
      pagedLastStartRef.current = startRow;
      const pages = planPageFetch({
        visibleStartRow: startRow,
        visibleEndRow: endRow,
        knownTotalRows,
        direction,
        pageSize: PAGED_ROW_PAGE_SIZE,
        isCached: (pageIndex) =>
          pagedCacheRef.current.has(frame.id, pageIndex, generation.signature) ||
          pagedInFlightRef.current.has(pageIndex),
      });
      if (pages.length === 0) return;
      setPagedLoading(true);
      setPagedError(null);
      const requestCounter = generation.counter;
      for (const pageIndex of pages) {
        pagedInFlightRef.current.add(pageIndex);
        void getFramePage(frame.id, pageRowOffset(pageIndex), PAGED_ROW_PAGE_SIZE)
          .then((response) => {
            if (isStaleRequest(requestCounter, pagedGenerationRef.current.counter))
              return;
            pagedCacheRef.current.set(frame.id, pageIndex, generation.signature, {
              values: response.rows,
              styleMatches: response.styleMatches ?? [],
            });
            setPagedTotalRows(response.totalRows);
            setPagedCacheTick((tick) => tick + 1);
          })
          .catch((reason) => {
            if (isStaleRequest(requestCounter, pagedGenerationRef.current.counter))
              return;
            setPagedError(String(reason).replace(/^Error:\s*/, ""));
          })
          .finally(() => {
            pagedInFlightRef.current.delete(pageIndex);
            if (pagedInFlightRef.current.size === 0) setPagedLoading(false);
          });
      }
    }, 60);
    return () => {
      if (pagedFetchTimerRef.current !== null) {
        window.clearTimeout(pagedFetchTimerRef.current);
        pagedFetchTimerRef.current = null;
      }
    };
    // pagedGeneration is a dependency even though the body reads the ref:
    // clearing the cache leaves the visible range needing pages nobody has
    // asked for, and the other dependencies can all be unchanged across a
    // filter or sort change (`knownTotalRows` in particular falls back to
    // `computed.totalRows`, which is the same number the cleared page had
    // reported). Without it the refetch waits for the next scroll.
  }, [
    isFileBacked,
    isTransposed,
    knownTotalRows,
    pagedGeneration,
    frame.id,
    virtualRange.end,
    virtualRange.start,
  ]);

  // The rendered window for the records-as-rows paged path: exactly the
  // rows the (debounced) virtual range needs, built from whatever the
  // cache currently has under the live generation. Missing rows render as
  // fixed-height placeholders (empty cells, flagged via placeholderOffsets)
  // so scroll geometry never jumps while their page is in flight.
  const pagedVisibleWindow = useMemo(() => {
    if (!isFileBacked || isTransposed) return null;
    const generation = pagedGenerationRef.current.signature;
    const rows: Row[] = [];
    const placeholderOffsets = new Set<number>();
    const styleMatches: FrameStyleMatches = {};
    for (
      let rowIndex = virtualRange.start;
      rowIndex < virtualRange.end;
      rowIndex += 1
    ) {
      const pageIndex = pageIndexForRow(rowIndex);
      const cachedPage = pagedCacheRef.current.get(frame.id, pageIndex, generation);
      const offsetInPage = rowIndex - pageRowOffset(pageIndex);
      const raw = cachedPage?.values[offsetInPage];
      if (!raw) placeholderOffsets.add(rowIndex - virtualRange.start);
      const id = `source:${frame.id}:${rowIndex}`;
      // The rules were run over the page these values came from, so the
      // answers arrive with them rather than being recomputed here -- a
      // formula is the core's to evaluate, on this path as on every other.
      const matched = cachedPage?.styleMatches[offsetInPage];
      if (matched?.length) styleMatches[id] = matched;
      rows.push({
        id,
        cells: Object.fromEntries(
          frame.columns.map((column, columnIndex) => [
            column.id,
            { raw: raw?.[columnIndex] ?? "", overrideFormula: null },
          ])
        ),
      });
    }
    return { rows, placeholderOffsets, styleMatches };
    // pagedCacheTick is the signal that new pages landed; the cache itself
    // lives in a ref so it isn't a dependency React can compare.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    isFileBacked,
    isTransposed,
    virtualRange.start,
    virtualRange.end,
    frame.id,
    frame.columns,
    frame.id,
    pagedCacheTick,
  ]);

  const displayedRows = useMemo(() => {
    if (isFileBacked && isTransposed) return page ? rowsFromFramePage(frame, page) : [];
    if (isFileBacked) return pagedVisibleWindow?.rows ?? [];
    return regularRows;
  }, [isFileBacked, isTransposed, page, pagedVisibleWindow, regularRows, frame]);
  // What the conditional-formatting rules made of those rows, keyed the same
  // way they are. Three row paths, one shape: the frame that holds its own
  // rows has its answers on the view, and both paged paths carry theirs on
  // the page the rows arrived in.
  const styleMatches = useMemo(() => {
    if (isFileBacked && isTransposed)
      return page ? styleMatchesFromFramePage(frame, page) : {};
    if (isFileBacked) return pagedVisibleWindow?.styleMatches ?? {};
    return computed.styleMatches ?? {};
  }, [
    computed.styleMatches,
    frame,
    isFileBacked,
    isTransposed,
    page,
    pagedVisibleWindow,
  ]);
  // What this card has on screen, for the window-level keyboard and
  // clipboard handlers. An imported frame keeps no rows in the document, so
  // this is the only place they exist on the client.
  useEffect(() => {
    onRenderedRows(frame.id, {
      rows: displayedRows,
      offset: isFileBacked && !isTransposed ? virtualRange.start : 0,
      totalRows,
    });
    return () => onRenderedRows(frame.id, null);
  }, [
    displayedRows,
    isFileBacked,
    isTransposed,
    onRenderedRows,
    frame.id,
    totalRows,
    virtualRange.start,
  ]);

  const gridFocusHere =
    gridFocus && gridFocus.objectId === frame.id ? gridFocus : null;

  // `extend` is shift-click and drag-select: the focus moves to this cell
  // while the anchor stays put, so the two corners describe a rectangle. The
  // first extend seeds the anchor from wherever the focus already was, which
  // is what makes shift-click work without a prior shift-arrow.
  const focusCell = (
    row: Row,
    column: Column,
    mode: GridFocusMode,
    options?: { extend?: boolean; span?: "row" | "column" | null }
  ) => {
    onSelect({ objectId: frame.id, rowId: row.id, columnId: column.id });
    onGridFocus((current) => ({
      viewId: view.id,
      objectId: frame.id,
      rowId: row.id,
      columnId: column.id,
      mode:
        mode === "edit" && !isEditableGridColumn(computed, column, frame)
          ? "navigate"
          : mode,
      editSeed: null,
      anchor:
        options?.extend && current?.objectId === frame.id
          ? current.anchor ?? { rowId: current.rowId, columnId: current.columnId }
          : null,
      // Extending an axis selection keeps that axis; an ordinary click
      // through a cell drops back to a plain rectangle.
      span:
        options?.span !== undefined
          ? options.span
          : options?.extend && current?.objectId === frame.id
          ? current.span
          : null,
    }));
  };

  // A drag is only a range once it leaves the cell it started on, so the
  // press itself sets a plain single-cell focus and `onPointerEnter` does
  // the extending. Tracking the press in a ref keeps every cell from
  // re-rendering as the pointer crosses it.
  const dragSelecting = useRef(false);
  const beginCellSelection = (
    event: React.PointerEvent,
    row: Row,
    column: Column
  ) => {
    if (event.button !== 0 || isTextEntryTarget(event.target)) return;
    // Selection follows the press, not the click, so the cell a drag starts
    // on is the anchor from the first pixel of movement.
    focusCell(row, column, "navigate", { extend: event.shiftKey });
    if (event.shiftKey) return;
    dragSelecting.current = true;
    const end = () => {
      dragSelecting.current = false;
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
    };
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };
  const extendCellSelection = (
    event: React.PointerEvent,
    row: Row,
    column: Column
  ) => {
    // `buttons` rather than the ref alone: a pointerup delivered outside the
    // window leaves the ref set, and this notices the button is long gone.
    if (!dragSelecting.current || event.buttons !== 1) return;
    focusCell(row, column, "navigate", { extend: true });
  };

  // Clicking a column header or a row's index selects that whole line of the
  // frame, not just the part currently on screen — an imported frame only
  // ever has a window of rows loaded, and "the whole column" should not mean
  // "as much of it as you happened to have scrolled past". The span rides on
  // the focus; copy resolves it against the real row count.
  const selectWholeColumn = (event: React.PointerEvent, column: Column) => {
    if (event.button !== 0) return;
    const row = displayedRows[0];
    if (!row) return;
    focusCell(row, column, "navigate", { extend: event.shiftKey, span: "column" });
  };
  const selectWholeRow = (event: React.PointerEvent, row: Row) => {
    if (event.button !== 0) return;
    const column = frame.columns[0];
    if (!column) return;
    focusCell(row, column, "navigate", { extend: event.shiftKey, span: "row" });
  };
  // Leaves edit mode in place (blur or Escape) without touching the selection,
  // so a click that carried focus elsewhere is not overridden.
  const settleCellEdit = (row: Row, column: Column) => {
    onGridFocus((current) =>
      current &&
      current.objectId === frame.id &&
      current.rowId === row.id &&
      current.columnId === column.id
        ? { ...current, mode: "navigate", editSeed: null }
        : current
    );
  };
  const commitCellEdit = (
    row: Row,
    column: Column,
    raw: string,
    move: GridDirection | null
  ) =>
    commitFrameCellEdit({
      frame,
      row,
      column,
      raw,
      move,
      onOperation,
      onTransformColumn,
      onGridStep,
      onSettle: settleCellEdit,
    });

  // Keep the active cell inside the virtual window and the horizontal viewport.
  useEffect(() => {
    if (!gridFocusHere || isTransposed) return;
    const element = scrollRef.current;
    if (!element) return;
    const rowIndex = displayedRows.findIndex((row) => row.id === gridFocusHere.rowId);
    if (rowIndex >= 0) {
      const nextTop = scrollTopToRevealRow(
        rowIndex,
        displayedRows.length,
        element.scrollTop,
        element.clientHeight
      );
      if (nextTop !== null) element.scrollTop = nextTop;
    }
    const animationFrame = requestAnimationFrame(() => {
      const active = element.querySelector<HTMLElement>("td.cell-focus");
      if (!active) return;
      const nextLeft = scrollLeftToRevealColumn(
        active.offsetLeft,
        active.offsetWidth,
        element.scrollLeft,
        element.clientWidth
      );
      if (nextLeft !== null) element.scrollLeft = nextLeft;
    });
    return () => cancelAnimationFrame(animationFrame);
    // Only re-scroll when the active cell moves, not on unrelated document
    // refreshes. frame/displayedRows/scrollRef are still read fresh whenever
    // this fires, since a gridFocusHere change always comes with a render
    // carrying current props — they just should not themselves retrigger it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gridFocusHere?.rowId, gridFocusHere?.columnId, isTransposed]);

  // A paged frame renders a window from partway down itself, so the row
  // numbers its cells carry are absolute — `virtualRange.start` plus their
  // place in the window. Positions resolved from row ids have to be counted
  // the same way or they describe a different frame: the selection was
  // computed in window coordinates and compared against absolute ones, which
  // matched for the focused cell (compared by id) and never for the range,
  // so an imported frame would not select past one cell.
  const rowIndexBase = isFileBacked && !isTransposed ? virtualRange.start : 0;
  const selectableRowCount =
    isFileBacked && !isTransposed ? totalRows : displayedRows.length;
  const gridPositionOf = (rowId: string, columnId: string): GridPosition | null => {
    const rowIndex = displayedRows.findIndex((row) => row.id === rowId);
    const columnIndex = frame.columns.findIndex((column) => column.id === columnId);
    return rowIndex >= 0 && columnIndex >= 0
      ? { row: rowIndexBase + rowIndex, col: columnIndex }
      : null;
  };
  const rangeAnchor = gridFocusHere?.anchor
    ? gridPositionOf(gridFocusHere.anchor.rowId, gridFocusHere.anchor.columnId)
    : null;
  const rangeFocus = gridFocusHere
    ? gridPositionOf(gridFocusHere.rowId, gridFocusHere.columnId)
    : null;
  // A span has no far corner to anchor against — "every row of this column"
  // reaches past what is loaded — so it paints to the edge of what is on
  // screen instead. Copy resolves the same span against the real row count.
  const selectionRange =
    rangeFocus && (rangeAnchor || gridFocusHere?.span)
      ? expandRangeForSpan(
          normalizeRange(rangeAnchor ?? rangeFocus, rangeFocus),
          gridFocusHere?.span ?? null,
          { rowCount: selectableRowCount, columnCount: frame.columns.length }
        )
      : null;

  // For the records-as-rows paged path, displayedRows *is* the visible
  // window (already sized to virtualRange) rather than the full row set,
  // so it's rendered as-is instead of re-sliced by absolute row indices.
  const visibleRows =
    isFileBacked && !isTransposed
      ? displayedRows
      : displayedRows.slice(virtualRange.start, virtualRange.end);
  const commitDraftRow = (allowEmpty = false) => {
    const values = Object.fromEntries(
      Object.entries(draftRow).filter(([, value]) => value.length > 0)
    );
    if (!allowEmpty && Object.keys(values).length === 0) return;
    setDraftRow({});
    onOperation({ type: "addRow", frameId: frame.id, values });
  };
  const addColumn = (afterColumnId: string | null) => {
    if (isFileBacked) return;
    void onOperation({
      type: "addColumn",
      frameId: frame.id,
      name: nextColumnName(frame),
      dataType: "string",
      afterColumnId,
    });
  };

  // Transposed paged frames keep Previous/Next -- records scroll
  // horizontally there, outside the row virtualizer this task drives.
  const transposedPageControls =
    isFileBacked && isTransposed ? (
      <FramePageControls
        offset={page?.offset ?? pageOffset}
        loaded={page?.limit ?? 0}
        total={totalRows}
        loading={transposedPageLoading}
        error={transposedPageError}
        onPrevious={() => {
          setPageOffset((offset) => Math.max(0, offset - transposedPageSize));
          scrollRef.current?.scrollTo({ top: 0 });
        }}
        onNext={() => {
          setPageOffset((offset) =>
            Math.min(Math.max(0, totalRows - 1), offset + transposedPageSize)
          );
          scrollRef.current?.scrollTo({ top: 0 });
        }}
      />
    ) : null;
  // Records-as-rows paged frames scroll freely through the virtualizer now,
  // so there's no Previous/Next -- just a status line for load errors.
  const pagedStatus =
    isFileBacked && !isTransposed ? (
      <FramePagedStatus total={totalRows} loading={pagedLoading} error={pagedError} />
    ) : null;

  if (frameOrientation(frame) === "fieldsAsRows") {
    // Click-to-sort column headers are not offered here: with fields as
    // rows, each "column" header is a single record and the header cells
    // that would carry the sort affordance are the transposed field labels
    // rendered by FieldsAsRowsFrameCard, not per-field columns. A view's
    // sort keys still apply core-side (frameView.sort persists regardless
    // of orientation), so switching back to records-as-rows shows the
    // sorted order; this orientation just doesn't expose the toggle UI.
    return (
      <FieldsAsRowsFrameCard
        frame={frame}
        rows={displayedRows}
        styleMatches={styleMatches}
        computed={computed}
        selection={selection}
        gridFocus={gridFocusHere}
        transformationLabels={transformationLabels}
        draftRow={draftRow}
        setDraftRow={setDraftRow}
        commitDraftRow={commitDraftRow}
        addColumn={addColumn}
        onSelect={onSelect}
        onFocusCell={focusCell}
        onCellPointerDown={beginCellSelection}
        onCellPointerEnter={extendCellSelection}
        onCommitCell={commitCellEdit}
        onSettleCell={settleCellEdit}
        onEditCalculatedColumn={editCalculatedColumn}
        onOperation={onOperation}
        readOnly={isReadOnly}
        rowOffset={page?.offset ?? 0}
        totalRows={totalRows}
        footer={transposedPageControls}
      />
    );
  }

  return (
    <RecordsAsRowsFrameCard
      frame={frame}
      computed={computed}
      selection={selection}
      gridFocus={gridFocusHere}
      displayedRows={displayedRows}
      styleMatches={styleMatches}
      visibleRows={visibleRows}
      virtualRange={virtualRange}
      selectionRange={selectionRange}
      filterMark={filterMark}
      filterPredicateCount={filter.predicates.length}
      filterPredicates={filter.predicates}
      transformationLabels={transformationLabels}
      sortKeys={sortKeys}
      totalRows={totalRows}
      isDerived={isDerived}
      isFileBacked={isFileBacked}
      isTransposed={isTransposed}
      isReadOnly={isReadOnly}
      canAddRows={canAddRows}
      canAddColumns={canAddColumns}
      pagedLoading={pagedLoading}
      placeholderOffsets={pagedVisibleWindow?.placeholderOffsets ?? new Set()}
      pagedStatus={pagedStatus}
      draftRow={draftRow}
      setDraftRow={setDraftRow}
      scrollRef={scrollRef}
      pendingScrollTop={pendingScrollTop}
      scrollFrame={scrollFrame}
      setScrollState={setScrollState}
      frameColumnDrop={frameColumnDrop}
      onOperation={onOperation}
      onSelect={onSelect}
      selectWholeColumn={selectWholeColumn}
      beginFrameColumnDrag={beginFrameColumnDrag}
      selectWholeRow={selectWholeRow}
      beginCellSelection={beginCellSelection}
      extendCellSelection={extendCellSelection}
      focusCell={focusCell}
      commitCellEdit={commitCellEdit}
      settleCellEdit={settleCellEdit}
      commitDraftRow={commitDraftRow}
      addColumn={addColumn}
      editCalculatedColumn={editCalculatedColumn}
      filterColumn={filterColumn}
    />
  );
}
