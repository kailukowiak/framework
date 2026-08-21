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
  | "scratchpad"
  | "library"
  | "arrange"
  | "fit"
  | "collapse"
  | "inspector-selection"
  | "inspector-format"
  | "inspector-wrangle"
  | "add-block"
  | "add-text"
  | "add-frame"
  | "add-container"
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
  "__TAURI_INTERNALS__" in window &&
  import.meta.env.VITE_FRAMEWORK_E2E !== "true";

/**
 * Whether the platform menu owns application shortcuts in this build.
 * The browser dev server and menu-less e2e shell keep them in the webview.
 */
export const hasNativeMenu = isDesktopShell;

const PLAIN: Record<string, ApplicationShortcut> = {
  z: "undo", n: "new", o: "open", s: "save", ",": "settings",
  "/": "shortcuts", j: "scratchpad", "1": "inspector-selection",
  "2": "inspector-format", "3": "inspector-wrangle", "=": "zoom-in",
  "+": "zoom-in", "-": "zoom-out", _: "zoom-out", "0": "zoom-reset",
};
const SHIFTED: Record<string, ApplicationShortcut> = {
  z: "redo", n: "new-window", s: "save-as", l: "library", a: "arrange", f: "fit",
  m: "collapse",
};
const INSERT: Record<string, ApplicationShortcut> = {
  b: "add-block", t: "add-text", f: "add-frame", g: "add-container",
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
