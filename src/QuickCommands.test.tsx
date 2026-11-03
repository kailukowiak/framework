import { describe, expect, it, vi } from "vitest";
import {
  matchingCommands,
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

describe("QuickCommands", () => {
  it("matches by label, group, keyword or a shortcut typed out, never a disabled command", () => {
    const rows = commands();
    expect(matchingCommands(rows, "open").map((command) => command.id)).toEqual([
      "open",
      "scratchwork",
    ]);
    expect(matchingCommands(rows, "cmd o").map((command) => command.id)).toEqual(["open"]);
    expect(matchingCommands(rows, "unavailable")).toEqual([]);
  });

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
    expect(labels(matchingCommands(items, "model-3.fw"))).toEqual(["Model 3"]);
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
    expect(labels(matchingCommands(shortcuts, query))).toEqual(["Show Wrangle Inspector"]);
  });

  it("matches a two-modifier shortcut typed as words", () => {
    expect(labels(matchingCommands(shortcuts, "shift cmd s"))).toEqual(["Save As…"]);
  });
});
