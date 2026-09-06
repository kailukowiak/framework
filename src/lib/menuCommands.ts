import type { MenuCommand } from "./bindings/MenuCommand";
import definition from "./menuCommands.json";

// ---------------------------------------------------------------------------
// The native menu's own definition, read back on this side.
//
// `src-tauri/src/menu.rs` holds the one table of application commands; its
// `export_menu_commands` test writes `menuCommands.json` next to this file the
// same way ts-rs writes the bindings, so a menu item added in Rust is visible
// to a vitest run without launching the app. Regenerate with
//
//     cargo test -p framework-desktop export_menu_commands
//
// Nothing at runtime depends on this file — the palette and the pop-out still
// name their own actions — but `menuCommands.test.ts` refuses to let those
// lists drift from it, which is what stops a new item being enabled and inert.
// ---------------------------------------------------------------------------

export type { MenuCommand };

export const menuCommands: MenuCommand[] = definition as MenuCommand[];

export const menuCommand = (id: string): MenuCommand | undefined =>
  menuCommands.find((command) => command.id === id);

const MODIFIER_SYMBOLS: Array<[string, string]> = [
  // macOS order, which is also the order the palette prints: ⌃⌥⇧⌘.
  ["Control", "⌃"],
  ["Ctrl", "⌃"],
  ["Alt", "⌥"],
  ["Option", "⌥"],
  ["Shift", "⇧"],
  ["CmdOrCtrl", "⌘"],
  ["Cmd", "⌘"],
  ["Command", "⌘"],
  ["Super", "⌘"],
];

const KEY_SYMBOLS: Record<string, string> = {
  Comma: ",",
  Slash: "/",
  Equal: "=",
  Plus: "+",
  Minus: "-",
  Period: ".",
  Space: "Space",
  Enter: "↩",
  Escape: "⎋",
  Backspace: "⌫",
  Delete: "⌦",
  Tab: "⇥",
};

const keySymbol = (key: string) => {
  if (key in KEY_SYMBOLS) return KEY_SYMBOLS[key];
  const digit = /^Digit(\d)$/.exec(key);
  if (digit) return digit[1];
  const letter = /^Key([A-Za-z])$/.exec(key);
  if (letter) return letter[1].toUpperCase();
  return key.length === 1 ? key.toUpperCase() : key;
};

/**
 * A Tauri accelerator (`CmdOrCtrl+Shift+P`) as the palette prints it (`⇧⌘P`).
 *
 * One direction only, and deliberately: the accelerator is what the platform
 * actually binds, so the symbols are the derived form. A palette row whose
 * `shortcut` disagrees with this is a row promising a key the menu does not
 * hold — which is exactly what the test using this function catches.
 */
export function acceleratorSymbols(accelerator: string): string {
  const parts = accelerator.split("+");
  const key = parts.pop() ?? "";
  const held = new Set(parts);
  const modifiers = MODIFIER_SYMBOLS.filter(([name]) => held.has(name)).map(
    ([, symbol]) => symbol
  );
  return [...new Set(modifiers)].join("") + keySymbol(key);
}
