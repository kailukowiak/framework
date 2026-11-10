// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { ColumnWidthMenuItems } from "./ColumnWidthMenuItems";
import { FrameCard } from "./FrameCard";
import { useColumnResize } from "./hooks/useColumnResize";
import { columnWidthFor, usesFixedColumns } from "./lib/columnLayout";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import { RecordsFrameHeader } from "./RecordsFrameHeader";
import type { Column, ComputedFrame, DocumentView, FrameObject } from "./lib/types";
import { clearMocks, fixtures, serveInvoke } from "./test/support";

beforeAll(() => {
  // jsdom has no layout: every header cell is 100px wide.
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
    configurable: true,
    get: () => 100,
  });
  if (!("PointerEvent" in window))
    (window as unknown as { PointerEvent: typeof MouseEvent }).PointerEvent = MouseEvent;
});
afterEach(() => {
  cleanup();
  clearMocks();
});

const columns = ["a", "b", "c"].map((id) => ({
  id,
  name: id.toUpperCase(),
  dataType: "string",
  formula: null,
})) as unknown as Column[];

function makeFrame(columnLayout?: Record<string, { width?: number | null; wrap?: boolean }>) {
  return {
    id: "frame-1",
    columns,
    display: { columnLayout },
    uniqueKeys: [],
  } as unknown as FrameObject;
}

function Harness({
  frame,
  onOperation,
  selectedColumns = null,
  measure,
  measureWidth,
}: {
  frame: FrameObject;
  onOperation: (op: unknown) => Promise<null>;
  selectedColumns?: { left: number; right: number } | null;
  measure?: (table: HTMLTableElement, id: string) => number | null;
  measureWidth?: (table: HTMLTableElement, id: string) => number | null;
}) {
  const scrollRef = createRef<HTMLDivElement>();
  const resize = useColumnResize({
    frame,
    selectedColumns,
    onOperation: onOperation as never,
    scrollRef,
    measure,
    measureWidth,
  });
  const model = {
    frame,
    computed: { formulas: {}, columnErrors: {} } as unknown as ComputedFrame,
    virtualRange: { start: 0, end: 0, paddingTop: 0, paddingBottom: 0 },
    canAddColumns: false,
    selection: null,
    selectionRange: null,
    frameColumnDrop: null,
    filterPredicates: [],
    sortKeys: [],
    onOperation,
    selectWholeColumn: vi.fn(),
    beginFrameColumnDrag: vi.fn(),
  } as unknown as RecordsAsRowsFrameCardProps;
  return (
    <div ref={scrollRef}>
      <table className={usesFixedColumns(frame, resize.preview) ? "fixed-columns" : undefined}>
        <colgroup>
          {frame.columns.map((column) => (
            <col
              key={column.id}
              data-testid={`col-${column.id}`}
              style={{ width: columnWidthFor(frame, column.id, resize.preview) ?? undefined }}
            />
          ))}
        </colgroup>
        <RecordsFrameHeader model={model} pinned={[]} resize={resize} />
      </table>
    </div>
  );
}

describe("resize handle", () => {
  it("previews while dragging and sends one operation on release, keeping wrap", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(
      <Harness frame={makeFrame({ a: { width: null, wrap: true } })} onOperation={send} />
    );
    const handle = screen.getByRole("separator", { name: "Resize A" });
    fireEvent.pointerDown(handle, { button: 0, clientX: 100 });
    fireEvent.pointerMove(window, { clientX: 140 });
    fireEvent.pointerMove(window, { clientX: 160 });
    expect(send).not.toHaveBeenCalled();
    fireEvent.pointerUp(window);
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send).toHaveBeenCalledWith({
      type: "setFrameDisplayColumnLayout",
      frameId: "frame-1",
      columns: [{ columnId: "a", width: 160, wrap: true }],
    });
  });

  it("does not select the column or write anything on a press without movement", () => {
    const send = vi.fn().mockResolvedValue(null);
    render(<Harness frame={makeFrame()} onOperation={send} />);
    const handle = screen.getByRole("separator", { name: "Resize B" });
    fireEvent.pointerDown(handle, { button: 0, clientX: 10 });
    fireEvent.pointerUp(window);
    expect(send).not.toHaveBeenCalled();
  });

  it("gives every selected column the width in the same operation", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(
      <Harness
        frame={makeFrame({ b: { width: 50, wrap: true } })}
        selectedColumns={{ left: 0, right: 1 }}
        onOperation={send}
      />
    );
    fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize A" }), {
      button: 0,
      clientX: 0,
    });
    fireEvent.pointerMove(window, { clientX: 30 });
    fireEvent.pointerUp(window);
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][0].columns).toEqual([
      { columnId: "a", width: 130, wrap: false },
      { columnId: "b", width: 130, wrap: true },
    ]);
  });

  it("fits the selection from the injected measurer in one operation", () => {
    const send = vi.fn().mockResolvedValue(null);
    const measure = vi.fn((_: HTMLTableElement, id: string) => (id === "a" ? 88 : 140));
    render(
      <Harness
        frame={makeFrame()}
        selectedColumns={{ left: 0, right: 1 }}
        onOperation={send}
        measure={measure}
      />
    );
    fireEvent.doubleClick(screen.getByRole("separator", { name: "Resize B" }));
    expect(send).toHaveBeenCalledTimes(1);
    expect(send.mock.calls[0][0].columns).toEqual([
      { columnId: "a", width: 88, wrap: false },
      { columnId: "b", width: 140, wrap: false },
      // Not fitted, but pinned where it already is so it does not jump.
      { columnId: "c", width: 100, wrap: false },
    ]);
  });

  it("steps with the arrow keys and writes once on key up", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(<Harness frame={makeFrame({ c: { width: 200, wrap: false } })} onOperation={send} />);
    const handle = screen.getByRole("separator", { name: "Resize C" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(send).not.toHaveBeenCalled();
    fireEvent.keyUp(handle, { key: "ArrowRight" });
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][0].columns).toEqual([
      { columnId: "c", width: 216, wrap: false },
    ]);
  });
});

describe("the first resize on an automatic table", () => {
  const measured = (_: HTMLTableElement, id: string) =>
    ({ a: 80, b: 120, c: 200 })[id] ?? null;

  it("previews every column at its measured width, so nothing jumps", () => {
    render(<Harness frame={makeFrame()} onOperation={vi.fn()} measureWidth={measured} />);
    const widths = () =>
      ["a", "b", "c"].map((id) => screen.getByTestId(`col-${id}`).style.width);
    expect(widths()).toEqual(["", "", ""]);
    fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize A" }), {
      button: 0,
      clientX: 100,
    });
    fireEvent.pointerMove(window, { clientX: 140 });
    expect(widths()).toEqual(["140px", "120px", "200px"]);
    expect(document.querySelector("table")?.classList.contains("fixed-columns")).toBe(true);
  });

  it("sends every column in one operation, with the dragged one changed", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(<Harness frame={makeFrame()} onOperation={send} measureWidth={measured} />);
    fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize A" }), {
      button: 0,
      clientX: 100,
    });
    fireEvent.pointerMove(window, { clientX: 140 });
    fireEvent.pointerUp(window);
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][0].columns).toEqual([
      { columnId: "a", width: 140, wrap: false },
      { columnId: "b", width: 120, wrap: false },
      { columnId: "c", width: 200, wrap: false },
    ]);
  });

  it("does the same for a keyboard nudge and for fit", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(
      <Harness
        frame={makeFrame()}
        onOperation={send}
        measureWidth={measured}
        measure={() => 90}
      />
    );
    const handle = screen.getByRole("separator", { name: "Resize C" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    fireEvent.keyUp(handle, { key: "ArrowRight" });
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][0].columns.map((c: { width: number }) => c.width)).toEqual([
      80, 120, 108,
    ]);
    fireEvent.doubleClick(screen.getByRole("separator", { name: "Resize B" }));
    expect(send.mock.calls[1][0].columns.map((c: { width: number }) => c.width)).toEqual([
      80, 90, 200,
    ]);
  });

  it("sends only the dragged column once widths are stored", async () => {
    const send = vi.fn().mockResolvedValue(null);
    render(
      <Harness
        frame={makeFrame({
          a: { width: 80, wrap: false },
          b: { width: 120, wrap: false },
          c: { width: 200, wrap: false },
        })}
        onOperation={send}
        measureWidth={measured}
      />
    );
    fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize B" }), {
      button: 0,
      clientX: 100,
    });
    fireEvent.pointerMove(window, { clientX: 130 });
    fireEvent.pointerUp(window);
    await waitFor(() => expect(send).toHaveBeenCalledTimes(1));
    expect(send.mock.calls[0][0].columns).toEqual([{ columnId: "b", width: 130, wrap: false }]);
  });
});

describe("leaving fixed layout", () => {
  it("returns to automatic once the last stored width is reset", () => {
    expect(usesFixedColumns(makeFrame({ a: { width: null, wrap: false } }))).toBe(false);
    expect(usesFixedColumns(makeFrame({ a: { width: 90, wrap: false } }))).toBe(true);
  });
});

describe("column menu", () => {
  it("offers Reset all column widths only when several are stored, in one operation", () => {
    const send = vi.fn();
    const { rerender } = render(
      <ColumnWidthMenuItems
        frame={makeFrame({
          a: { width: 80, wrap: true },
          b: { width: 120, wrap: false },
        })}
        column={columns[0]}
        onOperation={send}
      />
    );
    fireEvent.click(screen.getByText("Reset all column widths"));
    expect(send).toHaveBeenCalledWith({
      type: "setFrameDisplayColumnLayout",
      frameId: "frame-1",
      columns: [
        { columnId: "a", width: null, wrap: true },
        { columnId: "b", width: null, wrap: false },
      ],
    });
    rerender(
      <ColumnWidthMenuItems
        frame={makeFrame({ a: { width: 80, wrap: false } })}
        column={columns[0]}
        onOperation={send}
      />
    );
    expect(screen.queryByText("Reset all column widths")).toBeNull();
  });

  it("resets the width to automatic, keeping wrap", () => {
    const send = vi.fn();
    render(
      <ColumnWidthMenuItems
        frame={makeFrame({ a: { width: 120, wrap: true } })}
        column={columns[0]}
        onOperation={send}
      />
    );
    fireEvent.click(screen.getByText("Reset column width"));
    expect(send).toHaveBeenCalledWith({
      type: "setFrameDisplayColumnLayout",
      frameId: "frame-1",
      columns: [{ columnId: "a", width: null, wrap: true }],
    });
  });

  it("toggles wrap, keeping width, and offers no reset for an automatic column", () => {
    const send = vi.fn();
    const { rerender } = render(
      <ColumnWidthMenuItems
        frame={makeFrame({ a: { width: 120, wrap: false } })}
        column={columns[0]}
        onOperation={send}
      />
    );
    fireEvent.click(screen.getByText("Wrap text"));
    expect(send.mock.calls[0][0].columns).toEqual([
      { columnId: "a", width: 120, wrap: true },
    ]);
    rerender(<ColumnWidthMenuItems frame={makeFrame()} column={columns[1]} onOperation={send} />);
    expect(screen.queryByText("Reset column width")).toBeNull();
  });
});

describe("stored widths in the card", () => {
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
  afterEach(() => vi.unstubAllGlobals());

  it("reach the table's columns, and wrapped columns mark their cells", async () => {
    const view = structuredClone(fixtures.salesWithMargin) as DocumentView;
    const frame = view.objects.find(
      (object): object is FrameObject => object.kind === "frame"
    )!;
    const [first, second] = frame.columns;
    frame.display = {
      ...frame.display,
      columnLayout: {
        [first.id]: { width: 120, wrap: false },
        [second.id]: { width: 80, wrap: true },
      },
    } as FrameObject["display"];
    serveInvoke({
      get_frame_page: () => ({
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
      }),
    });
    const { container } = render(
      <FrameCard
        view={view.views.find((candidate) => candidate.objectId === frame.id)!}
        frame={frame}
        computed={view.computedFrames[frame.id]!}
        selection={null}
        gridFocus={null}
        onSelect={() => {}}
        onGridFocus={() => {}}
        onGridStep={() => {}}
        onRenderedRows={() => {}}
        onOperation={vi.fn().mockResolvedValue(null)}
        onRearrangeColumns={() => {}}
        onApplyVector={() => {}}
        onPairVector={() => {}}
        onJoinColumns={() => {}}
        onFilterColumn={() => {}}
        onTransformColumn={() => {}}
        onEditCalculatedColumn={() => {}}
        dataRefreshRevision={0}
      />
    );
    await waitFor(() => expect(container.querySelectorAll("col").length).toBeGreaterThan(2));
    const cols = Array.from(container.querySelectorAll<HTMLElement>("col"));
    expect(cols[1].style.width).toBe("120px");
    expect(cols[2].style.width).toBe("80px");
    expect(container.querySelector("table")?.classList.contains("fixed-columns")).toBe(true);
    await waitFor(() =>
      expect(
        container.querySelectorAll(`tbody td.wrap-cell[data-column-id="${second.id}"]`).length
      ).toBeGreaterThan(0)
    );
    expect(container.querySelectorAll(`td.wrap-cell[data-column-id="${first.id}"]`).length).toBe(0);
  });
});
