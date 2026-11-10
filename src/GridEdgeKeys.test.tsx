// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useRef, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrameCard } from "./FrameCard";
import { isTextEntryTarget, type GridFocus, type RenderedGrid } from "./FrameGrid";
import { useCardSelectionKeys } from "./hooks/useCardSelectionKeys";
import {
  announceGridReturn,
  useGridEdgeGrowth,
  useGridReturnListener,
} from "./hooks/useGridEdgeGrowth";
import { useGridKeyboardNavigation } from "./hooks/useGridKeyboardNavigation";
import type { OperationHandler } from "./lib/handlers";
import type { DocumentView, FrameObject, Selection } from "./lib/types";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "./test/support";

// The claims here are what the interface *emits* at the edges of the grid:
// which operation a keystroke past the last row, past the last column, or
// over a whole-row selection becomes. What addRow, deleteRows or addColumn
// mean is the engine's test to write (crates/framework-core/tests/
// grid_structure.rs); a mocked operation handler does not change the
// document, so nothing here computes.

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.unstubAllGlobals();
});

function pageOfLiteralRows(frame: FrameObject) {
  return {
    frameId: frame.id,
    totalRows: frame.rows.length,
    offset: 0,
    limit: frame.rows.length,
    columns: frame.columns,
    rowIds: frame.rows.map((row) => row.id),
    rows: frame.rows.map((row) =>
      frame.columns.map((column) => row.cells[column.id]?.raw ?? "")
    ),
    styleMatches: [],
  };
}

/** The fixture with `editing.columns` set as handed in by the caller. */
function withEditing(view: DocumentView, patch: { columns?: boolean; rows?: boolean }) {
  const copy = structuredClone(view);
  for (const computed of Object.values(copy.computedFrames))
    Object.assign(computed.editing, patch);
  return copy;
}

function Harness({
  documentView,
  onOperation,
  initialSelection = null,
  refresh = 0,
}: {
  documentView: DocumentView;
  onOperation: OperationHandler;
  initialSelection?: Selection | null;
  refresh?: number;
}) {
  const frame = documentView.objects.find(
    (object): object is FrameObject => object.kind === "frame"
  )!;
  const view = documentView.views.find((candidate) => candidate.objectId === frame.id)!;
  const computed = documentView.computedFrames[frame.id]!;
  const [selection, setSelection] = useState<Selection | null>(initialSelection);
  const [gridFocus, setGridFocus] = useState<GridFocus | null>(null);
  const renderedRows = useRef(new Map<string, RenderedGrid>());
  const cellFormulaToken = useRef(0);
  const transformColumnToken = useRef(0);
  const handleNavigateKey = useGridKeyboardNavigation({
    document: documentView,
    gridFocus,
    renderedRows,
    cellFormulaToken,
    transformColumnToken,
    setCellFormulaRequest: () => {},
    setTransformColumnRequest: () => {},
    clearActiveFormulaEditor: () => {},
    setGridFocus,
    setSelection,
    setInspectorSection: () => {},
    run: onOperation,
  });
  const { stepGridFocus } = useGridEdgeGrowth({
    document: documentView,
    gridFocus,
    renderedRows,
    setGridFocus,
    setSelection,
    run: onOperation,
  });
  useGridReturnListener(stepGridFocus);
  const handleCardKey = useCardSelectionKeys({
    document: documentView,
    selection,
    gridFocus,
    renderedRows,
    setGridFocus,
    setSelection,
    run: onOperation,
    navigateCanvas: () => false,
  });
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (isTextEntryTarget(event.target)) return;
      if (gridFocus?.mode === "navigate") handleNavigateKey(event);
      else if (selection) handleCardKey(event);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [gridFocus, handleCardKey, handleNavigateKey, selection]);
  return (
    <FrameCard
      view={view}
      frame={frame}
      computed={computed}
      selection={selection}
      gridFocus={gridFocus}
      onSelect={setSelection}
      onGridFocus={setGridFocus}
      onGridStep={stepGridFocus}
      onRenderedRows={(frameId, grid) => {
        if (grid) renderedRows.current.set(frameId, grid);
        else renderedRows.current.delete(frameId);
      }}
      onOperation={onOperation}
      onRearrangeColumns={() => {}}
      onApplyVector={() => {}}
      onPairVector={() => {}}
      onJoinColumns={() => {}}
      onFilterColumn={() => {}}
      onTransformColumn={() => {}}
      onEditCalculatedColumn={() => {}}
      dataRefreshRevision={refresh}
    />
  );
}

const view0 = fixtures.salesWithMargin;
const frame0 = objectNamed(view0, "frame", "Monthly sales");
const col = (name: string) => frame0.columns.find((column) => column.name === name)!;

/** Click the cell in `rowIndex` under `columnName`, giving the grid the keyboard. */
async function clickCell(
  user: ReturnType<typeof userEvent.setup>,
  rowIndex: number,
  columnName: string
) {
  const row = document.querySelector(`tr[data-row-id="${frame0.rows[rowIndex]!.id}"]`)!;
  await user.click(row.querySelector(`td[data-column-id="${col(columnName).id}"]`)!);
}

const lastIndex = frame0.rows.length - 1;
const key = (init: KeyboardEventInit) => fireEvent.keyDown(window, init);

async function mount(
  documentView = view0,
  onOperation = vi.fn().mockResolvedValue(null)
) {
  serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame0) });
  const user = userEvent.setup();
  const result = render(
    <Harness documentView={documentView} onOperation={onOperation} />
  );
  await screen.findAllByText("West");
  return { user, onOperation, ...result };
}

describe("Return and Tab at the bottom edge", () => {
  it("Return on the last row lands in the new-row line, same column, without writing a row", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, lastIndex, "Region");
    key({ key: "Enter" });
    expect(document.activeElement).toBe(screen.getByLabelText("New row Region"));
    expect(onOperation).not.toHaveBeenCalled();

    // Typing carries on from there: Return in the line is the ordinary add.
    await user.type(document.activeElement as HTMLElement, "North{Enter}");
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith({
        type: "addRow",
        frameId: frame0.id,
        values: { [col("Region").id]: "North" },
      })
    );
  });

  it("a committed edit on the last row steps into the new-row line too", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, lastIndex, "Region");
    key({ key: "F2" });
    const editor = document.querySelector<HTMLInputElement>("input.cell-editor")!;
    await user.clear(editor);
    await user.type(editor, "North{Enter}");
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith(
        expect.objectContaining({ type: "setCell", raw: "North" })
      )
    );
    expect(document.activeElement).toBe(screen.getByLabelText("New row Region"));
  });

  it("Return above the last row still just moves down", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 0, "Region");
    key({ key: "Enter" });
    expect(document.activeElement).not.toBe(screen.getByLabelText("New row Region"));
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("Tab on the very last cell adds a blank row", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, lastIndex, "Margin");
    key({ key: "Tab" });
    expect(onOperation).toHaveBeenCalledTimes(1);
    expect(onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: frame0.id,
      values: {},
    });
  });

  it("Tab anywhere else keeps wrapping and writes nothing", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 0, "Margin");
    key({ key: "Tab" });
    await clickCell(user, lastIndex, "Month");
    key({ key: "Tab" });
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("lands on the first cell of the row that arrives", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    const { user, rerender } = await mount(view0, onOperation);
    await clickCell(user, lastIndex, "Margin");
    key({ key: "Tab" });
    await waitFor(() => expect(onOperation).toHaveBeenCalled());

    // The engine's answer, handed in: the same document with one more row.
    const grown = structuredClone(view0);
    const grownFrame = grown.objects.find(
      (object): object is FrameObject => object.kind === "frame"
    )!;
    grownFrame.rows.push({ id: "row-new", cells: {} } as FrameObject["rows"][number]);
    // A new lineage hash is how a card knows its cached pages are stale.
    grown.computedFrames[grownFrame.id]!.fingerprint = "after-add-row";
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(grownFrame) });
    rerender(<Harness documentView={grown} onOperation={onOperation} refresh={1} />);
    await waitFor(() => {
      const focused = document.querySelector<HTMLTableCellElement>("td.cell-focus");
      expect(focused?.dataset.columnId).toBe(col("Month").id);
      expect(focused?.closest("tr")?.dataset.rowId).toBe("row-new");
    });
  });
});

describe("the ghost column", () => {
  it("one gesture is one operation: addColumn carrying the typed value", async () => {
    const { user, onOperation } = await mount();
    const ghost = screen.getByLabelText("New column, row 2");
    await user.type(ghost, "hello{Enter}");
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    expect(onOperation).toHaveBeenCalledWith({
      type: "addColumn",
      frameId: frame0.id,
      name: "New column",
      dataType: "string",
      afterColumnId: col("Margin").id,
      values: { [frame0.rows[1]!.id]: "hello" },
    });
  });

  it("a typed plain number makes a Number column", async () => {
    const { user, onOperation } = await mount();
    await user.type(screen.getByLabelText("New column, row 1"), "42{Enter}");
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    expect(onOperation).toHaveBeenCalledWith(
      expect.objectContaining({ type: "addColumn", dataType: "number" })
    );
  });

  it("is not drawn, and Right-arrow stops, where literal columns cannot grow", async () => {
    const view = withEditing(view0, { columns: false });
    const { user } = await mount(view);
    expect(screen.queryByLabelText("New column, row 1")).toBeNull();
    await clickCell(user, 0, "Margin");
    key({ key: "ArrowRight" });
    expect(
      document.querySelector<HTMLTableCellElement>("td.cell-focus")?.dataset.columnId
    ).toBe(col("Margin").id);
  });

  it("an untouched ghost cell writes nothing", async () => {
    const { user, onOperation } = await mount();
    const ghost = screen.getByLabelText("New column, row 1");
    await user.click(ghost);
    await user.tab();
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("Right-arrow off the last column steps onto the ghost cell of that row", async () => {
    const { user } = await mount();
    await clickCell(user, 2, "Margin");
    key({ key: "ArrowRight" });
    expect(document.activeElement).toBe(screen.getByLabelText("New column, row 3"));
  });
});

describe("structure chords", () => {
  const insert = { key: "≠", code: "Equal", altKey: true, metaKey: true };
  const remove = { key: "–", code: "Minus", altKey: true, metaKey: true };

  it("deletes every selected row in one deleteRows", async () => {
    const { user, onOperation } = await mount();
    const gutter = (index: number) =>
      document.querySelector(
        `tr[data-row-id="${frame0.rows[index]!.id}"] td.row-number`
      )!;
    fireEvent.pointerDown(gutter(1), { button: 0 });
    fireEvent.pointerDown(gutter(3), { button: 0, shiftKey: true });
    key(remove);
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    expect(onOperation).toHaveBeenCalledWith({
      type: "deleteRows",
      frameId: frame0.id,
      rowIds: [1, 2, 3].map((index) => frame0.rows[index]!.id),
    });
    void user;
  });

  it("delete does nothing for a plain cell selection", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 1, "Region");
    key(remove);
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("inserts a row above the focused row on a frame that owns its rows", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 2, "Region");
    key(insert);
    expect(onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: frame0.id,
      values: {},
      beforeRowId: frame0.rows[2]!.id,
    });
  });

  it("inserts a column before a later column too, with a single anchor", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 0, "Margin");
    key({ key: " ", ctrlKey: true });
    key(insert);
    const sent = onOperation.mock.calls[0]![0];
    expect(sent).toMatchObject({
      type: "addColumn",
      afterColumnId: null,
      beforeColumnId: col("Margin").id,
    });
  });

  it("appends instead of promising a position when the view is sorted", async () => {
    const sorted = structuredClone(view0);
    sorted.computedFrames[frame0.id]!.steps = [
      { kind: "sort", keys: [{ columnId: col("Region").id, descending: false }] },
    ] as never;
    const { user, onOperation } = await mount(sorted);
    await clickCell(user, 2, "Region");
    key(insert);
    expect(onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: frame0.id,
      values: {},
    });
  });

  it("appends instead on a file-backed frame", async () => {
    const imported = fixtures.importedSales;
    const importedFrame = objectNamed(imported, "frame", "Imported sales");
    const view = withEditing(imported, { rows: true });
    serveInvoke({
      get_frame_page: () => ({
        frameId: importedFrame.id,
        totalRows: 2,
        offset: 0,
        limit: 2,
        columns: importedFrame.columns,
        rowIds: [],
        rows: [
          ["2026-04", "West", "142000", "91000"],
          ["2026-01", "East", "118000", "76000"],
        ],
        styleMatches: [],
      }),
    });
    const onOperation = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();
    render(<Harness documentView={view} onOperation={onOperation} />);
    await user.click((await screen.findAllByText("West"))[0]!);
    key(insert);
    expect(onOperation).toHaveBeenCalledWith({
      type: "addRow",
      frameId: importedFrame.id,
      values: {},
    });
  });

  it("inserts a column before the selected one and deletes it, gated on editing.columns", async () => {
    const { user, onOperation } = await mount();
    await clickCell(user, 0, "Region");
    key({ key: " ", ctrlKey: true });
    key(insert);
    expect(onOperation).toHaveBeenCalledWith({
      type: "addColumn",
      frameId: frame0.id,
      name: "New column",
      dataType: "string",
      // Exactly one anchor: the engine refuses an add that names both a
      // column to follow and one to precede.
      afterColumnId: null,
      beforeColumnId: col("Region").id,
    });
    key(remove);
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith({
        type: "deleteColumn",
        frameId: frame0.id,
        columnId: col("Region").id,
      })
    );
  });

  it("deletes several selected columns as one batch, and shows a refusal like a single operation", async () => {
    const refusal = "Column Region feeds a formula.";
    const onOperation = vi.fn().mockResolvedValueOnce(refusal);
    const { user } = await mount(view0, onOperation);
    const header = (id: string) =>
      document.querySelector(`th[data-column-id="${id}"]`)!;
    const regionAt = frame0.columns.findIndex((column) => column.name === "Region");
    fireEvent.pointerDown(header(frame0.columns[regionAt]!.id), { button: 0 });
    fireEvent.pointerDown(header(frame0.columns[regionAt + 1]!.id), {
      button: 0,
      shiftKey: true,
    });
    key(remove);
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    void user;
    expect(onOperation).toHaveBeenCalledWith({
      type: "batch",
      operations: frame0.columns.slice(regionAt, regionAt + 2).map((column) => ({
        type: "deleteColumn",
        frameId: frame0.id,
        columnId: column.id,
      })),
    });
    // The engine's sentence comes back through the handler's return value,
    // the same channel as a refused single delete, and nothing else is sent.
    await expect(onOperation.mock.results[0].value).resolves.toBe(refusal);
    expect(onOperation).toHaveBeenCalledTimes(1);
  });

  it("refuses the column chords where columns cannot change", async () => {
    const { user, onOperation } = await mount(withEditing(view0, { columns: false }));
    await clickCell(user, 0, "Region");
    key({ key: " ", ctrlKey: true });
    key(insert);
    key(remove);
    expect(onOperation).not.toHaveBeenCalled();
  });
});

describe("a selected card, not entered", () => {
  const selected = { objectId: frame0.id };

  it("Return enters the grid at the first cell", async () => {
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame0) });
    render(
      <Harness
        documentView={view0}
        onOperation={vi.fn().mockResolvedValue(null)}
        initialSelection={selected}
      />
    );
    await screen.findAllByText("West");
    key({ key: "Enter" });
    await waitFor(() => {
      const focused = document.querySelector<HTMLTableCellElement>("td.cell-focus");
      expect(focused?.dataset.columnId).toBe(col("Month").id);
      expect(focused?.closest("tr")?.dataset.rowId).toBe(frame0.rows[0]!.id);
    });
  });

  it("Delete removes the card through deleteObject, once", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame0) });
    render(
      <Harness
        documentView={view0}
        onOperation={onOperation}
        initialSelection={selected}
      />
    );
    await screen.findAllByText("West");
    key({ key: "Delete" });
    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith({
        type: "deleteObject",
        objectId: frame0.id,
      })
    );
    expect(onOperation).toHaveBeenCalledTimes(1);
  });

  it("Delete in a text field does not take the card", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    serveInvoke({ get_frame_page: () => pageOfLiteralRows(frame0) });
    render(
      <Harness
        documentView={view0}
        onOperation={onOperation}
        initialSelection={selected}
      />
    );
    await screen.findAllByText("West");
    const title = document.querySelector<HTMLInputElement>("input.frame-name")!;
    title.focus();
    fireEvent.keyDown(title, { key: "Backspace" });
    expect(onOperation).not.toHaveBeenCalled();
  });
});

describe("a value committed outside the grid", () => {
  it("returns to the grid one row down", async () => {
    const { user } = await mount();
    await clickCell(user, 1, "Revenue");
    announceGridReturn();
    await waitFor(() => {
      const focused = document.querySelector<HTMLTableCellElement>("td.cell-focus");
      expect(focused?.closest("tr")?.dataset.rowId).toBe(frame0.rows[2]!.id);
    });
  });
});
