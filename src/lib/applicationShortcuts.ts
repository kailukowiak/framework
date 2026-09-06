export type ApplicationShortcut =
  | "undo"
  | "redo"
  | "new"
  | "new-window"
  | "open"
  | "save"
  | "save-as"
  | "settings"
  | "shortcuts"
  | "find"
  | "quick-commands"
  | "reference"
  | "scratchpad"
  | "library"
  | "arrange"
  | "fit"
  | "collapse"
  | "inspector-toggle"
  | "inspector-selection"
  | "inspector-format"
  | "inspector-wrangle"
  | "add-variable"
  | "add-block"
  | "add-text"
  | "add-matrix"
  | "add-frame"
  | "add-container"
  | "canvas-only"
  | "zoom-in"
  | "zoom-out"
  | "zoom-reset";

/**
 * Whether this is the real desktop shell: inside Tauri, and not the menu-less
 * e2e variant. More than one thing turns on this — the platform menu owning
 * application shortcuts, and anything that calls a Tauri plugin the browser
 * dev server has no counterpart for — so it is named for the condition rather
 * than for the first caller that needed it.
 */
export const isDesktopShell = () =>
  receivesMenuCommands() && import.meta.env.VITE_FRAMEWORK_E2E !== "true";

/**
 * Whether a menu command can arrive as an event in this shell.
 *
 * Not the same question as `hasNativeMenu`, and the difference is where a
 * bug lived: the menu-less e2e build draws no menu, but the desktop side
 * still emits `framework-menu-command` — the Scratchwork pop-out forwards
 * every workbook command to its owner that way, and the e2e replay command
 * drives the same path. A window that only subscribed when it had a menu
 * dropped both. The browser dev server, where nothing can emit, stays out.
 */
export const receivesMenuCommands = () => "__TAURI_INTERNALS__" in window;

/**
 * Whether the platform menu owns application shortcuts in this build.
 * The browser dev server and menu-less e2e shell keep them in the webview.
 */
export const hasNativeMenu = isDesktopShell;

const PLAIN: Record<string, ApplicationShortcut> = {
  z: "undo", n: "new", o: "open", s: "save", ",": "settings",
  "/": "shortcuts", j: "scratchpad", f: "find", k: "reference", "1": "inspector-selection",
  "2": "inspector-format", "3": "inspector-wrangle", "=": "zoom-in",
  "+": "zoom-in", "-": "zoom-out", _: "zoom-out", "0": "zoom-reset",
};
const SHIFTED: Record<string, ApplicationShortcut> = {
  z: "redo", n: "new-window", s: "save-as", l: "library", a: "arrange", f: "fit",
  m: "collapse", p: "quick-commands", i: "inspector-toggle", c: "canvas-only",
};
const INSERT: Record<string, ApplicationShortcut> = {
  v: "add-variable", b: "add-block", t: "add-text", m: "add-matrix",
  f: "add-frame", g: "add-container",
};

/** The one canonical map shared by the menu-less dev and e2e shells. */
export function applicationShortcut(event: KeyboardEvent): ApplicationShortcut | null {
  const modifier = event.metaKey || event.ctrlKey;
  if (!modifier) return null;
  // Option changes the printable `key` on macOS (⌥B is "∫"), while `code`
  // keeps naming the physical letter promised by the shortcut.
  const key =
    event.altKey && event.code?.startsWith("Key")
      ? event.code.slice(3).toLowerCase()
      : event.key.toLowerCase();
  if (event.altKey) return INSERT[key] ?? null;
  if (event.ctrlKey && key === "y") return "redo";
  return (event.shiftKey ? SHIFTED[key] : undefined) ?? PLAIN[key] ?? null;
}

const COMMAND_IDS: Partial<Record<ApplicationShortcut, string>> = {
  undo: "undo", redo: "redo", new: "new-document", "new-window": "new-window",
  open: "open-document", "save-as": "save-document-as", settings: "preferences",
  shortcuts: "keyboard-shortcuts", find: "find", "quick-commands": "quick-commands",
  reference: "reference", scratchpad: "scratchpad",
  library: "data-library", arrange: "tidy-layout", fit: "fit-view",
  collapse: "collapse-view", "inspector-toggle": "inspector-toggle",
  "inspector-selection": "inspector-selection", "inspector-format": "inspector-format",
  "inspector-wrangle": "inspector-wrangle", "add-block": "add-block",
  "add-text": "add-text", "add-frame": "add-frame", "add-container": "add-container",
  "add-variable": "add-variable", "add-matrix": "add-matrix", "canvas-only": "canvas-only",
  "zoom-in": "zoom-in", "zoom-out": "zoom-out", "zoom-reset": "zoom-reset",
};

/** Native menus, the browser shell, and Quick Commands name the same actions. */
export const shortcutCommandId = (shortcut: ApplicationShortcut) =>
  COMMAND_IDS[shortcut] ?? null;
