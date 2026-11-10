// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ContextMenuFrameActions, ContextMenuFrameEditItems } from "./ContextMenuFrameItems";
import type { GridContext, GridFocus } from "./FrameGrid";
import type { FrameObject, Operation } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

function frameWithMonth() {
  const sales = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const month = {
    ...sales.columns[0],
    id: "month-column",
    name: "Month",
    dataType: "date",
  } as FrameObject["columns"][number];
  return { ...sales, columns: [...sales.columns, month] };
}

function menuProps(frame: FrameObject, run: (operation: Operation) => Promise<string | null>) {
  return {
    contextMenu: {
      screenX: 100,
      screenY: 200,
      canvasX: 100,
      canvasY: 200,
      frameId: frame.id,
    },
    contextFrame: frame,
    contextColumn: null,
    askOnImport: false,
    importMode: "stored" as const,
    runAppendImport: vi.fn(),
    run,
    setAppendImport: vi.fn(),
    setContextMenu: vi.fn(),
    setInspectorSection: vi.fn(),
    setJoin: vi.fn(),
    setSelection: vi.fn(),
  };
}

it("declares a date column as the period with one choice", () => {
  const frame = frameWithMonth();
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  fireEvent.change(screen.getByLabelText("Period column"), {
    target: { value: "month-column" },
  });
  expect(run).toHaveBeenCalledWith({
    type: "setFramePeriod",
    frameId: frame.id,
    period: { columnId: "month-column", partitionColumnIds: [] },
  });
});

it("clears the declaration back to none", () => {
  const frame = {
    ...frameWithMonth(),
    period: { columnId: "month-column", partitionColumnIds: [] },
  };
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  expect(
    (screen.getByLabelText("Period column") as HTMLSelectElement).value
  ).toBe("month-column");
  fireEvent.change(screen.getByLabelText("Period column"), {
    target: { value: "" },
  });
  expect(run).toHaveBeenCalledWith({
    type: "setFramePeriod",
    frameId: frame.id,
    period: null,
  });
});

it("offers no period choice without a date column", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  expect(screen.queryByLabelText("Period column")).toBeNull();
});

type EditingFlags = { rows: boolean; columns: boolean };

/** Renders the structural items for a "Monthly sales" row/cell. The editing
 *  answer is handed in, not computed: the engine owns what each state means. */
function editItems(
  editing: EditingFlags,
  options: {
    fileBacked?: boolean;
    withRow?: boolean;
    selectedRows?: number;
    calculatedColumn?: boolean;
    sorted?: boolean;
  } = {}
) {
  const document = fixtures.salesWithMargin;
  const base = objectNamed(document, "frame", "Monthly sales");
  const frame: FrameObject = {
    ...base,
    ...(options.fileBacked
      ? { sourceFile: { path: "/tmp/sales.csv" } as unknown as FrameObject["sourceFile"] }
      : {}),
    columns: options.calculatedColumn
      ? base.columns.map((column, index) =>
          index === 0 ? ({ ...column, formula: "1 + 1" } as unknown as typeof column) : column
        )
      : base.columns,
  };
  const computed = document.computedFrames[frame.id];
  const rowId = frame.rows[1].id;
  const contextGrid = {
    frame,
    computed,
    displayedRows: frame.rows,
    rowOffset: 0,
    totalRows: frame.rows.length,
    orientation: "recordsAsRows",
    viewportRows: 20,
  } as unknown as GridContext;
  const view = document.views.find((candidate) => candidate.objectId === frame.id)!;
  const selected = options.selectedRows ?? 1;
  const gridFocus: GridFocus = {
    viewId: view.id,
    objectId: frame.id,
    rowId: frame.rows[1].id,
    columnId: frame.columns[0].id,
    mode: "navigate",
    editSeed: null,
    anchor:
      selected > 1
        ? { rowId: frame.rows[selected].id, columnId: frame.columns[0].id }
        : null,
    span: null,
  };
  const props = {
    contextMenu: {
      ...menuProps(frame, vi.fn()).contextMenu,
      rowId: options.withRow === false ? undefined : rowId,
    },
    contextFrame: frame,
    contextColumn: frame.columns[1],
    document: {
      ...document,
      computedFrames: {
        ...document.computedFrames,
        [frame.id]: {
          ...computed,
          editing: { ...computed.editing, ...editing },
          ...(options.sorted
            ? {
                steps: [
                  {
                    kind: "sort" as const,
                    keys: [{ columnId: frame.columns[0].id, descending: false }],
                  },
                ],
              }
            : {}),
        },
      },
    },
    deleteContextColumn: vi.fn(),
    deleteFromContext: vi.fn(),
    run: vi.fn(async (): Promise<string | null> => null),
    setContextMenu: vi.fn(),
    contextGrid,
    gridFocus,
  };
  render(<ContextMenuFrameEditItems {...props} />);
  return { props, frame, rowId };
}

const has = (name: string) => screen.queryByText(name) !== null;

it("offers every structural item on a frame that owns its rows and columns", () => {
  const { props, frame } = editItems({ rows: true, columns: true });
  for (const name of [
    "Insert row above",
    "Insert row below",
    "Insert column left",
    "Insert column right",
    "Column type",
    "Delete row",
    "Delete column",
  ])
    expect(has(name), name).toBe(true);
  expect(has("Add empty row")).toBe(false);
  expect(has("Hide column")).toBe(false);

  fireEvent.click(screen.getByText("Insert row above"));
  expect(props.run).toHaveBeenLastCalledWith({
    type: "addRow",
    frameId: frame.id,
    values: {},
    beforeRowId: frame.rows[1].id,
  });
  fireEvent.click(screen.getByText("Insert row below"));
  expect(props.run).toHaveBeenLastCalledWith({
    type: "addRow",
    frameId: frame.id,
    values: {},
    beforeRowId: frame.rows[2].id,
  });
  fireEvent.click(screen.getByText("Insert column left"));
  expect(props.run).toHaveBeenLastCalledWith(
    expect.objectContaining({
      type: "addColumn",
      afterColumnId: null,
      beforeColumnId: frame.columns[1].id,
    })
  );
  fireEvent.click(screen.getByText("Insert column right"));
  expect(props.run).toHaveBeenLastCalledWith(
    expect.objectContaining({ type: "addColumn", afterColumnId: frame.columns[1].id })
  );
  fireEvent.click(screen.getByText("Delete column"));
  expect(props.deleteContextColumn).toHaveBeenCalled();
});

it("still offers the column items beside a calculated column", () => {
  editItems({ rows: true, columns: true }, { calculatedColumn: true });
  expect(has("Insert column left")).toBe(true);
  expect(has("Delete column")).toBe(true);
});

it("does not offer column edits the engine would refuse when the chain fixes the schema", () => {
  editItems({ rows: true, columns: false });
  for (const name of [
    "Insert column left",
    "Insert column right",
    "Column type",
    "Delete column",
    "Add column",
  ])
    expect(has(name), name).toBe(false);
  expect(has("Hide column")).toBe(true);
  expect(has("Insert row above")).toBe(true);
  expect(has("Delete row")).toBe(true);
});

it("only appends rows to a file-backed frame, and still deletes them", () => {
  const { props, frame } = editItems(
    { rows: true, columns: false },
    { fileBacked: true }
  );
  expect(has("Insert row above")).toBe(false);
  expect(has("Insert row below")).toBe(false);
  fireEvent.click(screen.getByText("Add empty row"));
  expect(props.run).toHaveBeenCalledWith({
    type: "addRow",
    frameId: frame.id,
    values: {},
  });
  expect(has("Delete row")).toBe(true);
  expect(has("Hide column")).toBe(true);
});

it("offers only Add empty row on a sorted view, where above and below cannot be kept", () => {
  const { props, frame } = editItems({ rows: true, columns: true }, { sorted: true });
  expect(has("Insert row above")).toBe(false);
  expect(has("Insert row below")).toBe(false);
  fireEvent.click(screen.getByText("Add empty row"));
  expect(props.run).toHaveBeenCalledWith({
    type: "addRow",
    frameId: frame.id,
    values: {},
  });
  expect(has("Delete row")).toBe(true);
});

it("adds a row at the end when the click is not on a row", () => {
  editItems({ rows: true, columns: true }, { withRow: false });
  expect(has("Add empty row")).toBe(true);
  expect(has("Delete row")).toBe(false);
});

it("deletes every selected row with one operation and says how many", () => {
  const { props, frame } = editItems({ rows: true, columns: true }, { selectedRows: 3 });
  fireEvent.click(screen.getByText("Delete 3 rows"));
  expect(props.deleteFromContext).toHaveBeenCalledWith({
    type: "deleteRows",
    frameId: frame.id,
    rowIds: [frame.rows[1].id, frame.rows[2].id, frame.rows[3].id],
  });
});

it("offers nothing structural on a read-only grid except Hide", () => {
  editItems({ rows: false, columns: false });
  expect(has("Hide column")).toBe(true);
  expect(has("Insert row above")).toBe(false);
  expect(has("Add empty row")).toBe(false);
  expect(has("Delete row")).toBe(false);
});
