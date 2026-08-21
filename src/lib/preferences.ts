// Settings that describe the person rather than the document.
//
// A .fw file opened on another machine should look like itself; how large it
// is drawn is a fact about the desk it is sitting on. So none of this goes
// near the document — it lives in local storage and stays on the machine
// that chose it.

/**
 * How large the window draws everything, as a multiplier.
 *
 * Deliberately page zoom rather than a font size. Growing the type on its
 * own leaves the padding, the row heights and the icons exactly where they
 * were, so the result is a cramped interface rather than a legible one.
 * Zoom scales the layout the way a browser's ⌘+ does: a bigger interface is
 * still the same interface.
 */
export const DEFAULT_INTERFACE_SCALE = 1;

/**
 * The window has a minimum width it can lay out in, and zooming in spends
 * the width it has, so the top of the range is where the panels would start
 * fighting each other rather than where the pixels run out.
 */
export const MIN_INTERFACE_SCALE = 0.8;
export const MAX_INTERFACE_SCALE = 1.5;

/** Slider granularity: fine enough to settle on a size, coarse enough to land on one. */
export const INTERFACE_SCALE_STEP = 0.05;

/** What ⌘+ and ⌘- move by — a visible jump rather than a nudge. */
const KEYBOARD_STEP = 0.1;

/**
 * The nearest scale this app will actually draw at.
 *
 * Everything that can produce a scale goes through here — the slider, the
 * keyboard, and whatever was in storage from a previous version — so no
 * caller has to think about the bounds, and unreadable input lands on 100%
 * instead of on a blank window.
 */
export function clampInterfaceScale(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_INTERFACE_SCALE;
  const stepped = Math.round(value / INTERFACE_SCALE_STEP) * INTERFACE_SCALE_STEP;
  const bounded = Math.min(
    MAX_INTERFACE_SCALE,
    Math.max(MIN_INTERFACE_SCALE, stepped)
  );
  // Stepping is float arithmetic, and 0.9000000000000001 is a scale nobody
  // asked for and a readout nobody wants to see.
  return Number(bounded.toFixed(2));
}

/** One keyboard step larger (`1`) or smaller (`-1`), stopping at the bounds. */
export function nudgeInterfaceScale(value: number, direction: 1 | -1): number {
  return clampInterfaceScale(clampInterfaceScale(value) + direction * KEYBOARD_STEP);
}

/** The scale as people say it: "120%". */
export function formatInterfaceScale(value: number): string {
  return `${Math.round(clampInterfaceScale(value) * 100)}%`;
}

/**
 * What an import does with the file it reads.
 *
 * `stored` copies the values beside the document and forgets where they came
 * from; `linked` keeps a connector, so the frame can be refreshed and a
 * refresh replaces whatever is there.
 *
 * Stored is the default because it is the one that cannot surprise anybody:
 * the numbers stay as they were read, they can be edited, and the document
 * does not depend on a file that may be moved, changed, or on a machine
 * nobody else has. Linking is the more powerful option and the one worth
 * choosing deliberately.
 */
export type ImportMode = "stored" | "linked";

export const DEFAULT_IMPORT_MODE: ImportMode = "stored";

export function parseImportMode(raw: string | null): ImportMode {
  return raw === "linked" || raw === "stored" ? raw : DEFAULT_IMPORT_MODE;
}

/**
 * Whether an import stops to ask which of the two it should be.
 *
 * On by default: the choice decides whether the document depends on a file
 * outside it, which is not a thing to have made silently on somebody's
 * behalf the first time. Turning it off is what the preference is for, and
 * then the stored default is used without a word.
 */
export function parseAskOnImport(raw: string | null): boolean {
  return raw !== "false";
}

/** Numbers group thousands by default; an explicit false is the only opt-out. */
export function parseUseThousandsSeparators(raw: string | null): boolean {
  return raw !== "false";
}

/**
 * A stored scale, or 100% for anything unreadable.
 *
 * Storage is the one input nobody typed: it can hold whatever an older
 * build wrote there, and a preference that fails to parse is not worth
 * refusing to draw the window over.
 */
export function parseInterfaceScale(raw: string | null): number {
  if (raw === null) return DEFAULT_INTERFACE_SCALE;
  return clampInterfaceScale(Number.parseFloat(raw));
}

export type McpClient = "codex" | "claude" | "generic";

/**
 * One setup surface for the three client shapes shown in Preferences.
 *
 * Codex and Claude Code both accept the server command after `--`; the
 * generic form is the de-facto stdio JSON shape used by clients that edit a
 * configuration file instead. Absolute placeholders are intentional: a
 * command that looks runnable but points at the wrong document is worse than
 * one that plainly asks to be completed.
 */
export function mcpSetupText(
  client: McpClient,
  executablePath: string | null,
  documentPath: string | null
): string {
  const executable = executablePath ?? "/absolute/path/to/framework-mcp";
  const document = documentPath ?? "/absolute/path/to/document.fw";
  if (client === "generic") {
    return JSON.stringify(
      {
        mcpServers: {
          framework: {
            command: executable,
            args: ["--document", document],
          },
        },
      },
      null,
      2
    );
  }
  const quote = (value: string) => `'${value.replaceAll("'", `'\\''`)}'`;
  const command = `${quote(executable)} --document ${quote(document)}`;
  return client === "codex"
    ? `codex mcp add framework -- ${command}`
    : `claude mcp add framework -- ${command}`;
}
