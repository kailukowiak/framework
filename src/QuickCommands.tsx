import { Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import "./QuickCommands.css";

export type QuickCommand = {
  id: string;
  label: string;
  group: "Document" | "Canvas" | "View" | "Help";
  shortcut?: string;
  keywords?: string;
  disabled?: boolean;
  run: () => void;
};

type CommandState = {
  canUndo: boolean;
  canRedo: boolean;
  hasSelectedView: boolean;
};

const COMMANDS: Array<Omit<QuickCommand, "run" | "disabled"> & {
  action: string;
  available?: keyof CommandState;
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
  { id: "add-block", action: "add-block", label: "Add Formula Block", group: "Canvas", shortcut: "⌥⌘B" },
  { id: "add-text", action: "add-text", label: "Add Text", group: "Canvas", shortcut: "⌥⌘T" },
  { id: "add-frame", action: "add-frame", label: "Add Frame", group: "Canvas", shortcut: "⌥⌘F" },
  { id: "add-container", action: "add-container", label: "Add Container", group: "Canvas", shortcut: "⌥⌘G" },
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

/** The palette is an index over the same actions the native menu dispatches. */
export function quickCommandItems(
  actions: Record<string, () => void>,
  state: CommandState
): QuickCommand[] {
  return COMMANDS.flatMap(({ action, available, ...command }) => {
    const run = actions[action];
    if (!run) return [];
    return [{ ...command, disabled: available ? !state[available] : false, run }];
  });
}

const normalize = (value: string) => value.trim().toLocaleLowerCase();

/**
 * A keyboard-sized index over actions the application already owns. It does
 * not explain commands or duplicate their controls: a row names one action,
 * its existing shortcut when it has one, and runs it.
 */
export function QuickCommands({
  open = true,
  commands,
  onClose,
}: {
  open?: boolean;
  commands: QuickCommand[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const results = useMemo(() => {
    const needle = normalize(query);
    return commands.filter((command) => {
      if (command.disabled) return false;
      if (!needle) return true;
      return normalize(`${command.label} ${command.group} ${command.keywords ?? ""}`).includes(
        needle
      );
    });
  }, [commands, query]);
  const selected =
    results.find((command) => command.id === selectedId) ?? results[0] ?? null;

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);
  useEffect(() => {
    if (selected?.id !== selectedId) setSelectedId(selected?.id ?? null);
  }, [selected, selectedId]);

  const move = (offset: number) => {
    if (!results.length) return;
    const current = Math.max(
      0,
      results.findIndex((command) => command.id === selected?.id)
    );
    const next = Math.min(results.length - 1, Math.max(0, current + offset));
    setSelectedId(results[next].id);
    document
      .getElementById(`quick-command-${results[next].id}`)
      ?.scrollIntoView?.({ block: "nearest" });
  };

  const run = (command: QuickCommand) => {
    onClose();
    command.run();
  };

  if (!open) return null;

  return (
    <section
      className="quick-commands"
      role="dialog"
      aria-modal="false"
      aria-label="Quick Commands"
    >
      <label className="quick-commands-search">
        <Search size={15} aria-hidden="true" />
        <input
          ref={inputRef}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              onClose();
            } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
              event.preventDefault();
              move(event.key === "ArrowDown" ? 1 : -1);
            } else if (event.key === "Enter" && selected) {
              event.preventDefault();
              run(selected);
            }
          }}
          placeholder="Type a command"
          aria-label="Search commands"
          spellCheck={false}
        />
        <kbd>⇧⌘P</kbd>
      </label>
      <div className="quick-commands-results" role="listbox" aria-label="Commands">
        {results.map((command) => (
          <button
            id={`quick-command-${command.id}`}
            key={command.id}
            role="option"
            aria-selected={command.id === selected?.id}
            className={command.id === selected?.id ? "selected" : ""}
            onClick={() => run(command)}
            onMouseEnter={() => setSelectedId(command.id)}
          >
            <small>{command.group}</small>
            <span>{command.label}</span>
            {command.shortcut && <kbd>{command.shortcut}</kbd>}
          </button>
        ))}
        {!results.length && <p>No commands found</p>}
      </div>
    </section>
  );
}
