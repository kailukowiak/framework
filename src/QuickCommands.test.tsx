// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  QuickCommands,
  normalizeShortcut,
  quickCommandItems,
  selectionCommands,
  type QuickCommand,
  type SelectionActions,
  type SelectionTarget,
} from "./QuickCommands";

const commands = (run = vi.fn()): QuickCommand[] => [
  { id: "open", label: "Open…", group: "Document", shortcut: "⌘O", run },
  { id: "scratchwork", label: "Open Scratchwork", group: "Canvas", run: vi.fn() },
  {
    id: "disabled",
    label: "Unavailable action",
    group: "View",
    disabled: true,
    run: vi.fn(),
  },
];

afterEach(cleanup);

describe("QuickCommands", () => {
  it("indexes the workbook Scratchwork window action", () => {
    const run = vi.fn();
    const items = quickCommandItems(
      { "open-scratchwork-window": run },
      { canUndo: false, canRedo: false, hasSelectedView: false }
    );
    const item = items.find((candidate) => candidate.id === "scratchwork-window");
    expect(item?.label).toBe("Open Scratchwork Window");
    item?.run();
    expect(run).toHaveBeenCalledOnce();
  });

  it("filters commands and runs the selected action from the keyboard", () => {
    const run = vi.fn();
    const close = vi.fn();
    render(<QuickCommands commands={commands(run)} onClose={close} />);

    const search = screen.getByLabelText("Search commands");
    fireEvent.change(search, { target: { value: "open" } });
    expect(screen.getByText("Open…")).toBeTruthy();
    expect(screen.queryByText("Unavailable action")).toBeNull();

    fireEvent.keyDown(search, { key: "Enter" });
    expect(close).toHaveBeenCalledOnce();
    expect(run).toHaveBeenCalledOnce();
  });

  it("closes when the press lands outside it", () => {
    const close = vi.fn();
    const { container } = render(<QuickCommands commands={commands()} onClose={close} />);
    fireEvent.pointerDown(container.querySelector(".quick-commands-backdrop")!);
    expect(close).toHaveBeenCalledOnce();
  });

  it("moves through the visible commands and closes with Escape", () => {
    const close = vi.fn();
    render(<QuickCommands commands={commands()} onClose={close} />);
    const search = screen.getByLabelText("Search commands");

    fireEvent.keyDown(search, { key: "ArrowDown" });
    expect(screen.getByText("Open Scratchwork").closest("button")?.getAttribute("aria-selected"))
      .toBe("true");
    fireEvent.keyDown(search, { key: "Escape" });
    expect(close).toHaveBeenCalledOnce();
  });
});

const baseState = { canUndo: false, canRedo: false, hasSelectedView: false };

const selectionActions = (): SelectionActions => ({
  rename: vi.fn(),
  remove: vi.fn(),
  fitToWindow: vi.fn(),
  toggleCollapsed: vi.fn(),
  createFrameFrom: vi.fn(),
  sortColumn: vi.fn(),
  clearColumnSort: vi.fn(),
  pinColumns: vi.fn(),
  removeColumn: vi.fn(),
});

const orders: SelectionTarget = {
  name: "Orders · Amount",
  objectId: "frame-1",
  kind: "frame",
  collapsed: false,
  frameId: "frame-1",
  column: {
    id: "column-1",
    name: "Amount",
    descending: false,
    pinThrough: 2,
    ownsRows: true,
  },
};

const labels = (commands: QuickCommand[]) => commands.map((command) => command.label);

describe("selection commands", () => {
  it("names the group after the selected column and offers its verbs", () => {
    const actions = selectionActions();
    const commands = selectionCommands(orders, actions);
    expect(new Set(commands.map((command) => command.group))).toEqual(
      new Set(["Orders · Amount"])
    );
    expect(labels(commands)).toEqual([
      "Sort ascending",
      "Sort descending",
      "Clear sort",
      "Pin columns through here",
      "Delete column",
      "Create frame from this",
      "Rename",
      "Fit to window",
      "Collapse",
      "Delete frame",
    ]);
    commands.find((command) => command.label === "Sort descending")?.run();
    expect(actions.sortColumn).toHaveBeenCalledWith("frame-1", "column-1", true);
    commands.find((command) => command.label === "Delete column")?.run();
    expect(actions.removeColumn).toHaveBeenCalledWith("frame-1", "column-1");
  });

  it("says what a computed frame and an unsorted, pinned column can do", () => {
    const actions = selectionActions();
    const commands = selectionCommands(
      {
        ...orders,
        collapsed: true,
        column: {
          id: "column-1",
          name: "Amount",
          descending: null,
          pinThrough: 0,
          ownsRows: false,
        },
      },
      actions
    );
    expect(labels(commands)).toContain("Hide column");
    expect(labels(commands)).toContain("Unpin columns");
    expect(labels(commands)).toContain("Expand");
    expect(labels(commands)).not.toContain("Clear sort");
    commands.find((command) => command.label === "Unpin columns")?.run();
    expect(actions.pinColumns).toHaveBeenCalledWith("frame-1", 0);
  });

  it("offers only the card's own verbs when no column is selected", () => {
    const commands = selectionCommands(
      { name: "Notes", objectId: "text-1", kind: "text" },
      selectionActions()
    );
    expect(labels(commands)).toEqual([
      "Rename",
      "Fit to window",
      "Collapse",
      "Delete text",
    ]);
  });

  it("appears in the palette's rows, marked as a selection command", () => {
    const items = quickCommandItems(
      {},
      { ...baseState, selection: orders },
      { selection: selectionActions() }
    );
    expect(items.every((item) => item.kind === "selection")).toBe(true);
    expect(labels(items)).toContain("Sort ascending");
  });
});

describe("recent documents", () => {
  const recents = Array.from({ length: 10 }, (_, index) => ({
    path: `/tmp/model-${index}.fw`,
    title: `Model ${index}`,
  }));

  it("lists the last eight and opens one by path", () => {
    const openRecent = vi.fn();
    const items = quickCommandItems({}, { ...baseState, recents }, { openRecent });
    expect(items).toHaveLength(8);
    expect(items.every((item) => item.group === "Recent")).toBe(true);
    expect(items.every((item) => item.kind === "recent")).toBe(true);
    items[0].run();
    expect(openRecent).toHaveBeenCalledWith("/tmp/model-0.fw");
  });

  it("finds a recent document by its path as well as its name", () => {
    const openRecent = vi.fn();
    const items = quickCommandItems({}, { ...baseState, recents }, { openRecent });
    render(<QuickCommands commands={items} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Search commands"), {
      target: { value: "model-3.fw" },
    });
    expect(screen.getByText("Model 3")).toBeTruthy();
    expect(screen.queryByText("Model 4")).toBeNull();
  });
});

describe("shortcut text matching", () => {
  const shortcuts: QuickCommand[] = [
    { id: "wrangle", label: "Show Wrangle Inspector", group: "View", shortcut: "⌘3", run: vi.fn() },
    { id: "block", label: "Add Formula Block", group: "Canvas", shortcut: "⌥⌘B", run: vi.fn() },
    { id: "save", label: "Save As…", group: "Document", shortcut: "⇧⌘S", run: vi.fn() },
  ];

  it("reads a shortcut however it is spelled", () => {
    expect(normalizeShortcut("cmd 3")).toBe("⌘3");
    expect(normalizeShortcut("⌘3")).toBe("⌘3");
    expect(normalizeShortcut("shift cmd s")).toBe("⇧⌘S");
    expect(normalizeShortcut("option+command+b")).toBe("⌥⌘B");
    // A word that names no modifier is a word, not half a key.
    expect(normalizeShortcut("save")).toBe("");
  });

  it.each(["⌘3", "cmd 3", "command+3"])("matches %s on the shortcut", (query) => {
    render(<QuickCommands commands={shortcuts} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Search commands"), {
      target: { value: query },
    });
    expect(screen.getByText("Show Wrangle Inspector")).toBeTruthy();
    expect(screen.queryByText("Save As…")).toBeNull();
    cleanup();
  });

  it("matches a two-modifier shortcut typed as words", () => {
    render(<QuickCommands commands={shortcuts} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Search commands"), {
      target: { value: "shift cmd s" },
    });
    expect(screen.getByText("Save As…")).toBeTruthy();
    expect(screen.queryByText("Add Formula Block")).toBeNull();
  });
});

describe("a query that names no command", () => {
  it("hands it to Find rather than answering it", () => {
    const search = vi.fn();
    const close = vi.fn();
    render(
      <QuickCommands
        commands={commands()}
        onSearchDocument={search}
        onClose={close}
      />
    );
    const field = screen.getByLabelText("Search commands");
    fireEvent.change(field, { target: { value: "Acme Corp" } });
    expect(screen.getByText("Search document for “Acme Corp”")).toBeTruthy();
    fireEvent.keyDown(field, { key: "Enter" });
    expect(search).toHaveBeenCalledWith("Acme Corp");
    expect(close).toHaveBeenCalledOnce();
  });

  it("says nothing was found when Find is not on offer", () => {
    render(<QuickCommands commands={commands()} onClose={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Search commands"), {
      target: { value: "Acme Corp" },
    });
    expect(screen.getByText("No commands found")).toBeTruthy();
  });
});
