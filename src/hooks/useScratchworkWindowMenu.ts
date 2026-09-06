import { useCallback, useEffect } from "react";
import type { useActiveFormulaEditorCommands } from "../ActiveFormulaEditor";
import { useApplicationMenu } from "../useApplicationMenu";
import {
  forwardScratchworkCommand,
  getDocumentPath,
} from "../lib/api";
import {
  applicationShortcut,
  hasNativeMenu,
  shortcutCommandId,
} from "../lib/applicationShortcuts";

const FORWARDED_COMMANDS = [
  "new-window", "new-document", "open-document", "save-document-as",
  "package-document", "compact-data", "preferences", "keyboard-shortcuts",
  "find", "quick-commands", "reference", "check-for-updates", "data-library",
  "toggle-sources", "tidy-layout", "fit-view", "collapse-view",
  "inspector-toggle", "inspector-selection", "inspector-format",
  "inspector-wrangle", "add-block", "add-text", "add-frame", "add-container",
  "zoom-in", "zoom-out", "zoom-reset",
] as const;

/** Keeps the pop-out a full participant in the shared application menu. */
export function useScratchworkWindowMenu({
  activeCommands,
  navigateHistory,
  requestFocus,
  setError,
}: {
  activeCommands: ReturnType<typeof useActiveFormulaEditorCommands>;
  navigateHistory: (direction: "undo" | "redo") => Promise<void>;
  requestFocus: () => void;
  setError: (message: string | null) => void;
}) {
  const forward = useCallback(
    (command: string) => {
      void forwardScratchworkCommand(command).catch((reason) =>
        setError(String(reason).replace(/^Error:\s*/, ""))
      );
    },
    [setError]
  );
  const focusEditor = useCallback(() => {
    const current = activeCommands.getActive();
    if (current) activeCommands.focus();
    else requestFocus();
  }, [activeCommands, requestFocus]);
  const forwardedHandlers = Object.fromEntries(
    FORWARDED_COMMANDS.map((command) => [command, () => forward(command)])
  );
  useApplicationMenu(
    hasNativeMenu(),
    {
      ...forwardedHandlers,
      undo: () => void navigateHistory("undo"),
      redo: () => void navigateHistory("redo"),
      scratchpad: focusEditor,
      "open-scratchwork-window": focusEditor,
    },
    setError
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const shortcut = applicationShortcut(event);
      if (!shortcut) return;
      // Save is intentionally absent from the native menu because saved
      // workbooks autosave. An untitled workbook still uses ⌘S as Save As,
      // so this one shortcut belongs to the webview in every shell.
      if (shortcut === "save") {
        event.preventDefault();
        void getDocumentPath().then((path) => {
          if (!path) forward("save-document-as");
        });
        return;
      }
      if (hasNativeMenu()) return;
      event.preventDefault();
      if (shortcut === "undo" || shortcut === "redo") {
        void navigateHistory(shortcut);
      } else if (shortcut === "scratchpad") {
        focusEditor();
      } else {
        const command = shortcutCommandId(shortcut);
        if (command) forward(command);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [focusEditor, forward, navigateHistory]);
}
