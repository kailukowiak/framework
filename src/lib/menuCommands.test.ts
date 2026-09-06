import { describe, expect, it, vi } from "vitest";
import {
  COMMANDS,
  quickCommandItems,
  type SelectionActions,
} from "../QuickCommands";
import {
  FORWARDED_COMMANDS,
  LOCALLY_HANDLED_COMMANDS,
} from "../hooks/useScratchworkWindowMenu";
import { acceleratorSymbols, menuCommands } from "./menuCommands";

// ---------------------------------------------------------------------------
// One definition, checked three ways.
//
// The application's commands are declared once, in `src-tauri/src/menu.rs`,
// and exported to `menuCommands.json`. Two frontend lists index those same
// commands — the palette and the pop-out's forwarding table — and neither is
// derived from the menu, because each is answering a different question
// (what to show, and who runs it). What must never happen is a menu item
// existing in only one of them: that ships an enabled item that does nothing,
// which is exactly the inert Undo the pop-out was found with by hand.
//
// So the lists stay hand-written and these tests hold them against the menu.
// An exclusion is allowed, in writing, with the reason next to it.
//
// The palette also shows rows the menu could not have declared: the verbs of
// whatever is selected, and the documents opened before this one. Those are a
// category of their own — `kind` says which — and the rule below holds only
// the menu-backed rows against `menuCommands.json`, without loosening it for
// them.
// ---------------------------------------------------------------------------

/** Menu commands the palette deliberately does not list, and why. */
const NOT_IN_PALETTE: Record<string, string> = {
  // The palette cannot offer the gesture that opened it.
  "quick-commands": "this command is the palette",
  // Zooming is a repeated gesture — press, look, press again — and the
  // palette closes when a command runs, so a row here would be a slower way
  // to zoom once. The accelerators and the View menu keep them.
  "zoom-in": "held-repeat gesture; the palette closes after each run",
  "zoom-out": "held-repeat gesture; the palette closes after each run",
  "zoom-reset": "held-repeat gesture; the palette closes after each run",
};

/** Menu commands the Scratchwork window neither runs nor forwards, and why. */
const NOT_IN_SCRATCHWORK_WINDOW: Record<string, string> = {
  // Nothing yet. A command added here needs a reason a person would accept
  // for the item being present in the pop-out's menu and doing nothing.
};

describe("menu command definition", () => {
  it("is what the palette indexes", () => {
    const listed = new Set(COMMANDS.map((command) => command.action));
    const missing = menuCommands
      .map((command) => command.id)
      .filter((id) => !listed.has(id) && !(id in NOT_IN_PALETTE));
    expect(missing).toEqual([]);
  });

  it("names every action the palette offers", () => {
    const declared = new Set(menuCommands.map((command) => command.id));
    const unknown = COMMANDS.map((command) => command.action).filter(
      (action) => !declared.has(action)
    );
    expect(unknown).toEqual([]);
  });

  it("lists no palette exclusion the menu has stopped declaring", () => {
    const declared = new Set(menuCommands.map((command) => command.id));
    expect(Object.keys(NOT_IN_PALETTE).filter((id) => !declared.has(id))).toEqual(
      []
    );
  });

  it("is handled or forwarded by the Scratchwork window", () => {
    const answered = new Set<string>([
      ...FORWARDED_COMMANDS,
      ...LOCALLY_HANDLED_COMMANDS,
    ]);
    const inert = menuCommands
      .map((command) => command.id)
      .filter((id) => !answered.has(id) && !(id in NOT_IN_SCRATCHWORK_WINDOW));
    expect(inert).toEqual([]);
  });

  it("names every command the Scratchwork window answers", () => {
    const declared = new Set(menuCommands.map((command) => command.id));
    const unknown = [...FORWARDED_COMMANDS, ...LOCALLY_HANDLED_COMMANDS].filter(
      (id) => !declared.has(id)
    );
    expect(unknown).toEqual([]);
  });

  it("keeps a command in one of the pop-out's two lists, never both", () => {
    const forwarded = new Set<string>(FORWARDED_COMMANDS);
    expect(
      LOCALLY_HANDLED_COMMANDS.filter((id) => forwarded.has(id))
    ).toEqual([]);
  });

  it("binds the keys the palette prints", () => {
    const printed = COMMANDS.filter((command) => command.shortcut).map(
      (command) => [command.action, command.shortcut] as const
    );
    const declared = new Map(
      menuCommands.map((command) => [command.id, command.accelerator])
    );
    const wrong = printed.filter(([action, shortcut]) => {
      const accelerator = declared.get(action);
      return !accelerator || acceleratorSymbols(accelerator) !== shortcut;
    });
    expect(wrong).toEqual([]);
  });

  it("prints a shortcut for every command the menu gives a key", () => {
    const printed = new Map(
      COMMANDS.map((command) => [command.action, command.shortcut])
    );
    const silent = menuCommands.filter(
      (command) =>
        command.accelerator &&
        printed.has(command.id) &&
        !printed.get(command.id)
    );
    expect(silent.map((command) => command.id)).toEqual([]);
  });
});

describe("acceleratorSymbols", () => {
  it("prints modifiers in the order a Mac menu does", () => {
    expect(acceleratorSymbols("CmdOrCtrl+Shift+P")).toBe("⇧⌘P");
    expect(acceleratorSymbols("CmdOrCtrl+Alt+B")).toBe("⌥⌘B");
    expect(acceleratorSymbols("CmdOrCtrl+Digit1")).toBe("⌘1");
    expect(acceleratorSymbols("CmdOrCtrl+Comma")).toBe("⌘,");
    expect(acceleratorSymbols("CmdOrCtrl+Slash")).toBe("⌘/");
  });
});

/** A selection's verbs, wired to nothing: this is about which rows exist. */
const noSelectionActions = (): SelectionActions => ({
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

describe("rows the menu could not declare", () => {
  const items = quickCommandItems(
    Object.fromEntries(menuCommands.map((command) => [command.id, vi.fn()])),
    {
      canUndo: true,
      canRedo: true,
      hasSelectedView: true,
      selection: {
        name: "Orders",
        objectId: "frame-1",
        kind: "frame",
        frameId: "frame-1",
      },
      recents: [{ path: "/tmp/model.fw", title: "Model" }],
    },
    { selection: noSelectionActions(), openRecent: vi.fn() }
  );

  it("is where the selection's verbs and the recents live", () => {
    const kinds = new Set(items.map((item) => item.kind));
    expect(kinds).toEqual(new Set(["menu", "selection", "recent"]));
  });

  it("leaves every menu row traceable to the menu's own definition", () => {
    const declared = new Set(menuCommands.map((command) => command.id));
    const byId = new Map(COMMANDS.map((command) => [command.id, command.action]));
    const untraceable = items
      .filter((item) => item.kind === "menu")
      .map((item) => byId.get(item.id))
      .filter((action) => !action || !declared.has(action));
    expect(untraceable).toEqual([]);
  });

  it("exempts nothing else from that rule", () => {
    const exempt = items
      .filter((item) => item.kind !== "menu")
      .map((item) => item.id);
    expect(exempt.every((id) => /^(selection|recent):/.test(id))).toBe(true);
  });
});
