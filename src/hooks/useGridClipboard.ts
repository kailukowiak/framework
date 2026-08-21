import { useCallback, useState, type RefObject } from "react";
import {
  COPY_CONFIRM_CELLS,
  COPY_HEADERS_PREFERENCE,
  gridCellAt,
  gridRangeForFocus,
  isEditableGridColumn,
  isTextEntryTarget,
  loadedRegionRows,
  readCopyHeadersPreference,
  resolveGridContext,
  rowClipboardValues,
  rowsFromFramePage,
  selectedFrameRegion,
  type GridContext,
  type GridFocus,
  type RenderedGrid,
} from "../FrameGrid";
import { getFramePage } from "../lib/api";
import { writeClipboardText } from "../lib/clipboard";
import { formulaToken } from "../lib/formulaReferences";
import {
  hasTextSelection,
  isEmptyLiteralFrame,
  serializeGrid,
} from "../lib/gridClipboard";
import { positionsInRange } from "../lib/gridNavigation";
import { PAGED_ROW_PAGE_SIZE } from "../lib/pagedWindow";
import { parseGrid } from "../lib/parseGrid";
import type { Column, DocumentView, FrameObject, Operation } from "../lib/types";

/**
 * The grid's clipboard: building copyable text from a region (paged out of
 * the engine when it is not already loaded), the copy/cut/paste browser
 * event handlers, and the "reference another frame's column" copy that
 * materializes it first if it needs to. All of it keys off one
 * GridContext, resolved from `gridFocus` the same way everywhere.
 */
export function useGridClipboard({
  document,
  gridFocus,
  renderedRows,
  run,
  setError,
  setFrameCached,
}: {
  document: DocumentView | null;
  gridFocus: GridFocus | null;
  renderedRows: RefObject<Map<string, RenderedGrid>>;
  run: (operation: Operation) => Promise<string | null>;
  setError: (value: string | null) => void;
  setFrameCached: (
    frameId: string,
    cached: boolean,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
}) {
  const [copyIncludesHeaders, setCopyIncludesHeaders] = useState(
    readCopyHeadersPreference
  );

  const buildCopyText = useCallback(
    async (
      context: GridContext,
      region: { columns: Column[]; firstRow: number; rowCount: number },
      includeHeaders: boolean
    ) => {
      const grid: string[][] = includeHeaders
        ? [region.columns.map((column) => column.name)]
        : [];
      const loaded = loadedRegionRows(context, region);
      if (loaded) {
        for (const row of loaded)
          grid.push(rowClipboardValues(context, row, region.columns));
        return serializeGrid(grid);
      }
      for (
        let offset = 0;
        offset < region.rowCount;
        offset += PAGED_ROW_PAGE_SIZE
      ) {
        const page = await getFramePage(
          context.frame.id,
          region.firstRow + offset,
          Math.min(PAGED_ROW_PAGE_SIZE, region.rowCount - offset)
        );
        for (const row of rowsFromFramePage(context.frame, page))
          grid.push(rowClipboardValues(context, row, region.columns));
      }
      return serializeGrid(grid);
    },
    []
  );

  const copySelection = useCallback(
    async (includeHeaders: boolean) => {
      if (!document || !gridFocus) return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const region = context ? selectedFrameRegion(context, gridFocus) : null;
      if (!context || !region || !region.rowCount) return;
      const cells = region.rowCount * region.columns.length;
      if (
        cells > COPY_CONFIRM_CELLS &&
        !window.confirm(
          `Copy ${region.rowCount.toLocaleString()} rows × ${
            region.columns.length
          } columns (${cells.toLocaleString()} cells)?\n\nThat is a large clipboard payload and has to be read out of the frame first.`
        )
      )
        return;
      try {
        const written = await writeClipboardText(
          await buildCopyText(context, region, includeHeaders)
        );
        setError(written ? null : "Could not copy: the clipboard refused the write.");
      } catch (reason) {
        setError(`Could not copy: ${String(reason).replace(/^Error:\s*/, "")}`);
      }
    },
    [buildCopyText, document, gridFocus, renderedRows, setError]
  );

  /// Puts `` `Frame`.`Column` `` on the clipboard, taking a snapshot of the
  /// frame first if it has not got one.
  ///
  /// The snapshot is not a detail to make someone go and arrange: it is what
  /// a reference across frames requires, it is one operation, and doing it
  /// here is the difference between the feature being usable and being
  /// documented.
  const copyColumnReference = useCallback(
    async (frame: FrameObject, column: Column, alreadyMaterialized: boolean) => {
      if (!alreadyMaterialized) {
        const failure = await setFrameCached(frame.id, true, { inlineError: true });
        if (failure) {
          setError(`Could not materialize ${frame.name}: ${failure}`);
          return;
        }
      }
      try {
        const written = await writeClipboardText(
          `${formulaToken(frame.name)}.${formulaToken(column.name)}`
        );
        setError(written ? null : "Could not copy: the clipboard refused the write.");
      } catch (reason) {
        setError(`Could not copy: ${String(reason).replace(/^Error:\s*/, "")}`);
      }
    },
    [setError, setFrameCached]
  );

  const handleGridCopy = useCallback(
    (event: ClipboardEvent) => {
      if (
        !document ||
        !gridFocus ||
        gridFocus.mode !== "navigate" ||
        isTextEntryTarget(event.target) ||
        hasTextSelection()
      )
        return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const region = context ? selectedFrameRegion(context, gridFocus) : null;
      if (!context || !region || !region.rowCount) return;
      event.preventDefault();
      const loaded = loadedRegionRows(context, region);
      // The synchronous path is the one that can write to the event's own
      // clipboardData, so it is kept for the ordinary case. A selection
      // reaching past the loaded rows has to fetch, and by then the event is
      // long finished — hence the async clipboard for that case only.
      if (!loaded) {
        void copySelection(copyIncludesHeaders);
        return;
      }
      const grid: string[][] = copyIncludesHeaders
        ? [region.columns.map((column) => column.name)]
        : [];
      for (const row of loaded)
        grid.push(rowClipboardValues(context, row, region.columns));
      event.clipboardData?.setData("text/plain", serializeGrid(grid));
    },
    [copyIncludesHeaders, copySelection, document, gridFocus, renderedRows]
  );

  const setCopyHeadersDefault = useCallback((includeHeaders: boolean) => {
    setCopyIncludesHeaders(includeHeaders);
    try {
      window.localStorage.setItem(COPY_HEADERS_PREFERENCE, String(includeHeaders));
    } catch {
      // A browser refusing storage should not stop the copy itself; the
      // choice just does not survive the session.
    }
  }, []);

  // Cut is copy plus clear, and it clears only what the frame would let you
  // type over — a derived or imported column has no cell to empty.
  const handleGridCut = useCallback(
    (event: ClipboardEvent) => {
      if (
        !document ||
        !gridFocus ||
        gridFocus.mode !== "navigate" ||
        isTextEntryTarget(event.target) ||
        hasTextSelection()
      )
        return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const range = context ? gridRangeForFocus(context, gridFocus) : null;
      if (!context || !range) return;
      handleGridCopy(event);
      const updates = positionsInRange(range).flatMap((candidate) => {
        const target = gridCellAt(context, candidate);
        return target && isEditableGridColumn(context.computed, target.column)
          ? [{ rowId: target.row.id, columnId: target.column.id, raw: "" }]
          : [];
      });
      if (updates.length)
        void run({ type: "setCells", frameId: context.frame.id, cells: updates });
    },
    [document, gridFocus, handleGridCopy, renderedRows, run]
  );

  const handleGridPaste = useCallback(
    (event: ClipboardEvent) => {
      if (
        !document ||
        !gridFocus ||
        gridFocus.mode !== "navigate" ||
        isTextEntryTarget(event.target)
      )
        return;
      const context = resolveGridContext(document, gridFocus, renderedRows.current);
      const range = context ? gridRangeForFocus(context, gridFocus) : null;
      const source = event.clipboardData?.getData("text/plain") ?? "";
      if (!context || !range || !source.trim()) return;
      event.preventDefault();

      // An empty frame has no shape to preserve, so the clipboard gets to
      // decide it — headers, column count, and types all come from the
      // core's Polars reader, exactly as a file import would.
      if (isEmptyLiteralFrame(context.frame)) {
        void run({
          type: "setFrameFromPastedText",
          frameId: context.frame.id,
          text: source,
        });
        return;
      }

      const matrix = parseGrid(source);
      if (!matrix.length) return;
      // One value against a range fills the range — the Excel behaviour,
      // and the only case where the selection rather than the clipboard
      // decides how much gets written.
      if (
        matrix.length === 1 &&
        matrix[0].length === 1 &&
        (range.bottom > range.top || range.right > range.left)
      ) {
        const updates = positionsInRange(range).flatMap((position) => {
          const target = gridCellAt(context, position);
          return target && isEditableGridColumn(context.computed, target.column)
            ? [{ rowId: target.row.id, columnId: target.column.id, raw: matrix[0][0] }]
            : [];
        });
        if (updates.length)
          void run({ type: "setCells", frameId: context.frame.id, cells: updates });
        return;
      }

      // Otherwise the clipboard decides: the core writes from the anchor and
      // grows the frame when the block runs past the last row.
      const anchor = gridCellAt(context, { row: range.top, col: range.left });
      if (!anchor) return;
      void run({
        type: "pasteCells",
        frameId: context.frame.id,
        rowId: anchor.row.id,
        columnId: anchor.column.id,
        grid:
          context.orientation === "fieldsAsRows"
            ? // Fields as rows means the clipboard's rows are this frame's
              // columns, so the block is transposed back before it is sent.
              (matrix[0] ?? []).map((_, index) => matrix.map((line) => line[index] ?? ""))
            : matrix,
      });
    },
    [document, gridFocus, renderedRows, run]
  );

  return {
    copyIncludesHeaders,
    setCopyHeadersDefault,
    copySelection,
    copyColumnReference,
    handleGridCopy,
    handleGridCut,
    handleGridPaste,
  };
}
