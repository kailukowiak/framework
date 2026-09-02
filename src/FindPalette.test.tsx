// @vitest-environment jsdom

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { FindPalette, FindPaletteHost } from "./FindPalette";
import type { DocumentView } from "./lib/types";

// Nothing paged in this document, so the palette answers entirely from the
// view it was handed and the engine is never asked anything.
const searchFrameRows = vi.fn();
vi.mock("./lib/api", () => ({ searchFrameRows: (...args: unknown[]) => searchFrameRows(...args) }));

// jsdom has no layout, so it has no scrollIntoView. Keeping the selection
// visible is real behaviour; there is simply nothing here for it to scroll.
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const view = (): DocumentView =>
  ({
    objects: [
      {
        kind: "frame",
        id: "sales",
        name: "North Sales",
        columns: [{ id: "region", name: "Northing" }],
        rows: [{ id: "row-1" }],
      },
    ],
    views: [{ id: "view-sales", objectId: "sales" }],
    computedFrames: {
      sales: {
        formulas: {},
        rows: { "row-1": { region: { display: "Northolt" } } },
      },
    },
    computedBlocks: {},
  }) as unknown as DocumentView;

function open() {
  const onJumpToObject = vi.fn();
  const onFocusCell = vi.fn();
  const onFocusColumn = vi.fn();
  const onClose = vi.fn();
  render(
    <FindPalette
      document={view()}
      onJumpToObject={onJumpToObject}
      onFocusCell={onFocusCell}
      onFocusColumn={onFocusColumn}
      onClose={onClose}
    />
  );
  const input = screen.getByLabelText("Find in document");
  fireEvent.change(input, { target: { value: "north" } });
  return { input, onJumpToObject, onFocusCell, onFocusColumn, onClose };
}

describe("FindPalette", () => {
  afterEach(cleanup);

  it("lists every match under its frame and selects the first", () => {
    open();
    expect(
      screen.getAllByRole("option").map((option) => option.textContent)
    ).toEqual(["North SalesFrame", "NorthingColumn", "NorthingNortholt"]);
    expect(screen.getAllByRole("option")[0].getAttribute("aria-selected")).toBe("true");
  });

  it("moves the selection with the arrow keys", () => {
    const { input } = open();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(screen.getAllByRole("option")[1].getAttribute("aria-selected")).toBe("true");
  });

  it("sends Enter to the callback the selected hit is specific enough for", () => {
    const { input, onJumpToObject, onFocusColumn, onClose } = open();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onFocusColumn).toHaveBeenCalledOnce();
    expect(onFocusColumn.mock.calls[0][0]).toMatchObject({
      kind: "column",
      objectId: "sales",
      columnId: "region",
    });
    expect(onJumpToObject).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("takes a name hit to the object and a cell hit to the cell", () => {
    const { onJumpToObject, onFocusCell } = open();
    fireEvent.click(screen.getAllByRole("option")[0]);
    expect(onJumpToObject).toHaveBeenCalledWith("sales");

    cleanup();
    const second = open();
    fireEvent.click(screen.getAllByRole("option")[2]);
    expect(second.onFocusCell.mock.calls[0][0]).toMatchObject({
      kind: "cell",
      rowId: "row-1",
      columnId: "region",
      viewId: "view-sales",
    });
    expect(onFocusCell).not.toHaveBeenCalled();
  });

  it("closes on Escape, and says so plainly when nothing matches", () => {
    const { input, onClose } = open();
    fireEvent.change(input, { target: { value: "zzz" } });
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("No matches").textContent).toBe("No matches");

    fireEvent.keyDown(input, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("FindPaletteHost", () => {
  afterEach(cleanup);

  it("carries a paged hit's absolute row position onto the grid focus", async () => {
    searchFrameRows.mockResolvedValue([
      {
        rowIndex: 900,
        rowId: "row-900",
        columnId: "region",
        snippet: "Northolt",
      },
    ]);
    const pagedView: DocumentView = {
      objects: [
        {
          kind: "frame",
          id: "sales",
          name: "North Sales",
          columns: [{ id: "region", name: "Northing" }],
          rows: [],
        },
      ],
      views: [{ id: "view-sales", objectId: "sales" }],
      computedFrames: {
        sales: { formulas: {}, rows: {}, paged: true },
      },
      computedBlocks: {},
    } as unknown as DocumentView;
    const jumpToObject = vi.fn();
    const setSelection = vi.fn();
    const setGridFocus = vi.fn();
    render(
      <FindPaletteHost
        document={pagedView}
        jumpToObject={jumpToObject}
        setSelection={setSelection}
        setGridFocus={setGridFocus}
        onClose={vi.fn()}
      />
    );
    const input = screen.getByLabelText("Find in document");
    fireEvent.change(input, { target: { value: "north" } });
    // Flush the search debounce and the resolved promise.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
    });

    const hit = screen.getAllByRole("option").find(
      (option) => option.textContent === "NorthingNortholt"
    );
    expect(hit).toBeTruthy();
    fireEvent.click(hit!);

    expect(setGridFocus).toHaveBeenCalledOnce();
    const focus = setGridFocus.mock.calls[0][0];
    expect(focus.rowId).toBe("row-900");
    expect(focus.rowIndex).toBe(900);
  });
});
