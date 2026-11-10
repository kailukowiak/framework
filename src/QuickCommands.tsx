/**
 * Where a row came from. Only `"menu"` rows are the menu's own commands, and
 * only those are held against `menuCommands.json`: a row about the current
 * selection or a recent document names something this document happens to
 * hold, which the menu could not have declared.
 */
export type QuickCommandKind = "menu" | "selection" | "recent";

export type QuickCommand = {
  id: string;
  label: string;
  /** The menu's own group, the selected object's name, or "Recent". */
  group: string;
  kind?: QuickCommandKind;
  shortcut?: string;
  keywords?: string;
  disabled?: boolean;
  run: () => void;
};

/** The three facts that decide whether a menu row is offered or greyed out. */
type CommandAvailability = "canUndo" | "canRedo" | "hasSelectedView";

/**
 * What the selected object is, in the terms the palette needs to name its
 * verbs — not a `DataObject`, so this stays a plain description a test can
 * write out by hand. App derives it; `selectionCommands` reads it.
 */
export type SelectionTarget = {
  /** The group heading: the card, and the column after it when one is picked. */
  name: string;
  objectId: string;
  /** The object's kind, as the context menu's Delete says it ("frame"). */
  kind: string;
  /** True when the card is collapsed, so the verb can say Expand instead. */
  collapsed?: boolean;
  /** Set when the selected object is a frame. */
  frameId?: string;
  /** Set when a column of that frame is selected. */
  column?: {
    id: string;
    name: string;
    /** How it sorts today; null when it is not one of the frame's sort keys. */
    descending: boolean | null;
    /** Leading columns a pin through here would freeze; 0 unpins. */
    pinThrough: number;
    /**
     * `editing.columns` — the engine's rule, not `editing.rows`. A frame whose
     * chain fixes its schema owns rows and still hides a column rather than
     * deleting it, and is offered no insert.
     */
    editsColumns: boolean;
  };
  /**
   * The rows the grid selection names, when one does. `canInsertBetween` is
   * false on a frame read from a file, which only grows at its end; `below` is
   * the row the next one goes before (null appends).
   */
  rows?: {
    ids: string[];
    canInsertBetween: boolean;
    below: string | null;
  };
};

/**
 * The handlers behind the selection verbs. Each one is the same handler the
 * right-click menu (or the column header, for sorting) already calls — the
 * palette is another way to reach them, not a second implementation.
 */
export type SelectionActions = {
  rename: (objectId: string) => void;
  remove: (objectId: string) => void;
  fitToWindow: () => void;
  toggleCollapsed: () => void;
  createFrameFrom: (frameId: string) => void;
  sortColumn: (frameId: string, columnId: string, descending: boolean) => void;
  clearColumnSort: (frameId: string, columnId: string) => void;
  pinColumns: (frameId: string, pinnedColumns: number) => void;
  removeColumn: (frameId: string, columnId: string) => void;
  insertColumn: (frameId: string, columnId: string, side: "left" | "right") => void;
  insertRow: (frameId: string, beforeRowId: string | null) => void;
  deleteRows: (frameId: string, rowIds: string[]) => void;
};

/** A document the application has open before, as the recents list holds it. */
export type RecentEntry = { path: string; title: string };

/** How many recents are worth a row. Past this it is the Data Library's job. */
const RECENT_LIMIT = 8;

export type CommandState = {
  canUndo: boolean;
  canRedo: boolean;
  hasSelectedView: boolean;
  selection?: SelectionTarget | null;
  recents?: RecentEntry[];
};

export type CommandHandlers = {
  selection?: SelectionActions;
  openRecent?: (path: string) => void;
};

/**
 * The palette's index over the application's menu commands, keyed by the same
 * `action` id `src-tauri/src/menu.rs` declares. Exported so
 * `src/lib/menuCommands.test.ts` can hold it against the menu's own
 * definition: an item added to the menu and not to this list is missing from
 * ⇧⌘P, and a `shortcut` printed here that the menu does not bind is a promise
 * the keyboard will not keep.
 */
export const COMMANDS: Array<Omit<QuickCommand, "run" | "disabled" | "kind"> & {
  action: string;
  available?: CommandAvailability;
}> = [
  { id: "new", action: "new-document", label: "New Document", group: "Document", shortcut: "⌘N" },
  { id: "new-window", action: "new-window", label: "New Window", group: "Document", shortcut: "⇧⌘N" },
  { id: "open", action: "open-document", label: "Open…", group: "Document", shortcut: "⌘O" },
  { id: "save-as", action: "save-document-as", label: "Save As…", group: "Document", shortcut: "⇧⌘S", keywords: "save copy rename" },
  { id: "package", action: "package-document", label: "Package Document…", group: "Document" },
  { id: "compact", action: "compact-data", label: "Compact Data", group: "Document" },
  { id: "undo", action: "undo", label: "Undo", group: "Document", shortcut: "⌘Z", available: "canUndo" },
  { id: "redo", action: "redo", label: "Redo", group: "Document", shortcut: "⇧⌘Z", available: "canRedo" },
  { id: "scratchwork", action: "scratchpad", label: "Go to Scratchwork", group: "Canvas", shortcut: "⌘J", keywords: "scratchpad calculate" },
  { id: "scratchwork-window", action: "open-scratchwork-window", label: "Open Scratchwork Window", group: "View", keywords: "scratchpad pop out separate window" },
  { id: "find", action: "find", label: "Find in Document", group: "Document", shortcut: "⌘F", keywords: "search" },
  { id: "library", action: "data-library", label: "Open Data Library…", group: "Document", shortcut: "⇧⌘L", keywords: "recent projects import" },
  { id: "add-variable", action: "add-variable", label: "Add Variable", group: "Canvas", shortcut: "⌥⌘V" },
  { id: "add-block", action: "add-block", label: "Add Formula Block", group: "Canvas", shortcut: "⌥⌘B" },
  { id: "add-text", action: "add-text", label: "Add Text", group: "Canvas", shortcut: "⌥⌘T" },
  { id: "add-matrix", action: "add-matrix", label: "Add Calculation Matrix", group: "Canvas", shortcut: "⌥⌘M" },
  { id: "add-frame", action: "add-frame", label: "Add Frame", group: "Canvas", shortcut: "⌥⌘F" },
  { id: "add-container", action: "add-container", label: "Add Container", group: "Canvas", shortcut: "⌥⌘G" },
  { id: "canvas-only", action: "canvas-only", label: "Canvas Only", group: "View", shortcut: "⇧⌘C", keywords: "close panel hide sidebar" },
  { id: "data-panel", action: "toggle-sources", label: "Show or Hide Data Panel", group: "View", shortcut: "⇧⌘D" },
  { id: "inspector", action: "inspector-toggle", label: "Show or Hide Inspector", group: "View", shortcut: "⇧⌘I" },
  { id: "selection", action: "inspector-selection", label: "Show Selection Inspector", group: "View", shortcut: "⌘1" },
  { id: "format", action: "inspector-format", label: "Show Format Inspector", group: "View", shortcut: "⌘2" },
  { id: "wrangle", action: "inspector-wrangle", label: "Show Wrangle Inspector", group: "View", shortcut: "⌘3" },
  { id: "arrange", action: "tidy-layout", label: "Arrange Left to Right", group: "View", shortcut: "⇧⌘A" },
  { id: "fit", action: "fit-view", label: "Fit Selected Card to Window", group: "View", shortcut: "⇧⌘F", available: "hasSelectedView" },
  { id: "collapse", action: "collapse-view", label: "Collapse or Expand Selected Card", group: "View", shortcut: "⇧⌘M", available: "hasSelectedView" },
  { id: "reference", action: "reference", label: "Open Reference", group: "Help", shortcut: "⌘K", keywords: "formulas guide functions" },
  { id: "shortcuts", action: "keyboard-shortcuts", label: "Keyboard Shortcuts…", group: "Help", shortcut: "⌘/" },
  { id: "settings", action: "preferences", label: "Settings…", group: "Document", shortcut: "⌘," },
  { id: "updates", action: "check-for-updates", label: "Check for Updates…", group: "Help" },
];

/**
 * The verbs the selected object's own menus offer, named after it.
 *
 * Column verbs come first when a column is selected, because that is what the
 * group heading says was picked. Each row calls the handler the right-click
 * menu calls; nothing here knows what an operation is.
 */
export function selectionCommands(
  target: SelectionTarget,
  actions: SelectionActions
): QuickCommand[] {
  const commands: QuickCommand[] = [];
  const add = (id: string, label: string, run: () => void, keywords?: string) =>
    commands.push({
      id: `selection:${id}`,
      kind: "selection",
      group: target.name,
      label,
      keywords,
      run,
    });
  const { frameId, column } = target;
  if (frameId && column) {
    add("sort-ascending", "Sort ascending", () =>
      actions.sortColumn(frameId, column.id, false)
    );
    add("sort-descending", "Sort descending", () =>
      actions.sortColumn(frameId, column.id, true)
    );
    if (column.descending !== null)
      add("clear-sort", "Clear sort", () =>
        actions.clearColumnSort(frameId, column.id)
      );
    add(
      "pin-columns",
      column.pinThrough === 0 ? "Unpin columns" : "Pin columns through here",
      () => actions.pinColumns(frameId, column.pinThrough),
      "freeze"
    );
    if (column.editsColumns) {
      add("insert-column-left", "Insert column left", () =>
        actions.insertColumn(frameId, column.id, "left")
      );
      add("insert-column-right", "Insert column right", () =>
        actions.insertColumn(frameId, column.id, "right")
      );
    }
    add(
      "remove-column",
      column.editsColumns ? "Delete column" : "Hide column",
      () => actions.removeColumn(frameId, column.id),
      "drop"
    );
  }
  const { rows } = target;
  if (frameId && rows?.ids.length) {
    if (rows.canInsertBetween) {
      add("insert-row-above", "Insert row above", () =>
        actions.insertRow(frameId, rows.ids[0])
      );
      add("insert-row-below", "Insert row below", () =>
        actions.insertRow(frameId, rows.below)
      );
    } else {
      // A file-backed frame, or one shown sorted or filtered, has no slot
      // the person can point at; the one honest add is at the end.
      add("add-row", "Add row", () => actions.insertRow(frameId, null));
    }
    add(
      "delete-rows",
      rows.ids.length > 1 ? `Delete ${rows.ids.length} rows` : "Delete row",
      () => actions.deleteRows(frameId, rows.ids),
      "remove"
    );
  }
  if (frameId)
    add("create-frame", "Create frame from this", () =>
      actions.createFrameFrom(frameId)
    );
  add("rename", "Rename", () => actions.rename(target.objectId));
  add("fit", "Fit to window", actions.fitToWindow, "zoom");
  add(
    "collapse",
    target.collapsed ? "Expand" : "Collapse",
    actions.toggleCollapsed
  );
  add("delete", `Delete ${target.kind}`, () => actions.remove(target.objectId));
  return commands;
}

/**
 * Every row the palette can show: the menu's commands, then the verbs for
 * whatever is selected, then the documents this one was opened after.
 *
 * Pure, and given plain descriptions rather than a document, so a test can
 * state a selection and read back the rows it produces.
 */
export function quickCommandItems(
  actions: Record<string, () => void>,
  state: CommandState,
  handlers: CommandHandlers = {}
): QuickCommand[] {
  const menu = COMMANDS.flatMap<QuickCommand>(({ action, available, ...command }) => {
    const run = actions[action];
    if (!run) return [];
    return [
      {
        ...command,
        kind: "menu",
        disabled: available ? !state[available] : false,
        run,
      },
    ];
  });
  const selection =
    state.selection && handlers.selection
      ? selectionCommands(state.selection, handlers.selection)
      : [];
  const openRecent = handlers.openRecent;
  const recent: QuickCommand[] = openRecent
    ? (state.recents ?? []).slice(0, RECENT_LIMIT).map((entry) => ({
        id: `recent:${entry.path}`,
        kind: "recent",
        group: "Recent",
        label: entry.title,
        keywords: entry.path,
        run: () => openRecent(entry.path),
      }))
    : [];
  return [...menu, ...selection, ...recent];
}

const normalize = (value: string) => value.trim().toLocaleLowerCase();

const MODIFIER_SPELLINGS: Array<[RegExp, string]> = [
  // Canonical order, the one a Mac menu prints and `acceleratorSymbols`
  // produces: ⌃⌥⇧⌘. Building the query in the same order is what lets the
  // two be compared as plain strings.
  [/⌃|\bctrl\b|\bcontrol\b/g, "⌃"],
  [/⌥|\balt\b|\bopt\b|\boption\b/g, "⌥"],
  [/⇧|\bshift\b/g, "⇧"],
  [/⌘|\bcmd\b|\bcommand\b|\bmeta\b|\bsuper\b/g, "⌘"],
];

/**
 * A shortcut written any of the ways a person types it — `⌘3`, `cmd 3`,
 * `shift cmd p` — reduced to the symbols the palette prints. Returns "" for
 * text that names no modifier, which is how the search knows an ordinary word
 * is not a half-typed key.
 */
export function normalizeShortcut(value: string): string {
  let rest = normalize(value);
  const held: string[] = [];
  for (const [spelling, symbol] of MODIFIER_SPELLINGS) {
    const stripped = rest.replace(spelling, " ");
    if (stripped === rest) continue;
    held.push(symbol);
    rest = stripped;
  }
  if (!held.length) return "";
  const key = rest.replace(/[\s+]/g, "");
  // Every key this application binds is one character long, so anything
  // longer is a sentence that happened to contain the word "command" rather
  // than someone reaching for a key.
  if (key.length > 1) return "";
  return held.join("") + key.toLocaleUpperCase();
}

/**
 * The commands a query names: by label, group or keyword, or by a shortcut
 * typed out as `cmd 3`. Disabled commands are left out rather than shown
 * greyed, because a palette row is a promise that Enter does something.
 */
export function matchingCommands(commands: QuickCommand[], query: string): QuickCommand[] {
  const needle = normalize(query);
  const keys = normalizeShortcut(query);
  return commands.filter((command) => {
    if (command.disabled) return false;
    if (!needle) return true;
    if (
      normalize(`${command.label} ${command.group} ${command.keywords ?? ""}`).includes(needle)
    )
      return true;
    return Boolean(
      keys && command.shortcut && normalizeShortcut(command.shortcut).includes(keys)
    );
  });
}
