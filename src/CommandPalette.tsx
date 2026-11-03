import { Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import "./QuickCommands.css";
import { useDocumentSearch } from "./hooks/useDocumentSearch";
import type { FindGroup, FindHit } from "./lib/documentSearch";
import type { HelpTarget } from "./lib/helpPanel";
import { helpEntries, searchHelpEntries, type HelpEntry } from "./lib/helpSearch";
import type { DocumentView } from "./lib/types";
import { matchingCommands, type QuickCommand } from "./QuickCommands";

/**
 * ⇧⌘P searches everything; ⌘F is the same palette looking only at the
 * document, which is what Find always was.
 */
export type PaletteMode = "commands" | "document";

/** One line of the palette, whichever kind of answer it is. */
export type PaletteRow = {
  id: string;
  section: "commands" | "document" | "help";
  /** The command's menu group, the object a match is in, or "Help". */
  group: string;
  label: string;
  detail?: string;
  shortcut?: string;
  /** A row that changes what the palette lists rather than doing something. */
  keepsOpen?: boolean;
  run: () => void;
};

/** In the everything search, a few of each kind, so commands are not buried. */
const DOCUMENT_PREVIEW = 5;
const HELP_PREVIEW = 4;
/**
 * One typed letter is nearly always the start of a command's name. Matching
 * it against every cell and every help entry would bury the command under
 * rows nobody asked for.
 */
const SEARCH_MIN_LENGTH = 2;

function findRow(hit: FindHit, group: string, onJump: (hit: FindHit) => void): PaletteRow {
  // A match on the object's own name already shows that name as its group,
  // so the row says what kind of thing it is instead of repeating it.
  const own = hit.kind === "object";
  return {
    id: `find:${hit.id}`,
    section: "document",
    group,
    label: own ? hit.snippet : hit.label,
    detail: own ? undefined : hit.snippet,
    run: () => onJump(hit),
  };
}

function helpRow(
  entry: HelpEntry,
  query: string,
  onOpenHelp: (target: HelpTarget) => void
): PaletteRow {
  return {
    id: `help:${entry.id}`,
    section: "help",
    group: "Help",
    label: entry.title,
    detail: entry.kind === "Function" ? entry.function.category : entry.guide.category ?? entry.kind,
    run: () => onOpenHelp({ query, entryId: entry.id }),
  };
}

/**
 * Every row for one query, in the order the palette lists them: commands,
 * then matches in the document, then help. Pure, so a test can state the
 * answers and read back the rows.
 *
 * The palette used to answer commands only and end with "Search help for…"
 * and "Search document for…" rows that handed the words to another box. Those
 * said where an answer might be without saying whether there was one; listing
 * the best few answers of each kind here says both.
 */
export function paletteRows({
  mode,
  query,
  commands,
  groups,
  help,
  onJump,
  onOpenHelp,
  onShowDocument,
}: {
  mode: PaletteMode;
  query: string;
  commands: QuickCommand[];
  groups: FindGroup[];
  help: HelpEntry[];
  onJump: (hit: FindHit) => void;
  onOpenHelp: (target: HelpTarget) => void;
  onShowDocument: () => void;
}): PaletteRow[] {
  const found = groups.flatMap((group) =>
    group.hits.map((hit) => findRow(hit, group.name, onJump))
  );
  if (mode === "document") return found;
  const rows: PaletteRow[] = matchingCommands(commands, query).map((command) => ({
    id: command.id,
    section: "commands",
    group: command.group,
    label: command.label,
    shortcut: command.shortcut,
    run: command.run,
  }));
  rows.push(...found.slice(0, DOCUMENT_PREVIEW));
  if (found.length > DOCUMENT_PREVIEW)
    rows.push({
      id: "find:all",
      section: "document",
      group: "Find",
      label: `All ${found.length} matches in the document`,
      shortcut: "⌘F",
      keepsOpen: true,
      run: onShowDocument,
    });
  rows.push(...help.slice(0, HELP_PREVIEW).map((entry) => helpRow(entry, query.trim(), onOpenHelp)));
  return rows;
}

/**
 * The one search box. Commands, things in the document, and help answer the
 * same query in one list, so nobody has to know which box an answer lives in
 * before they can find it — the discoverability failure four separate boxes
 * (⇧⌘P, ⌘F, ⌘K and the shortcuts list) kept producing. Each kind of answer
 * still has its own surface once chosen: a command runs, a match is jumped
 * to, a help entry opens beside the canvas.
 */
export function CommandPalette({
  initialMode,
  commands,
  document,
  onJump,
  onOpenHelp,
  onClose,
}: {
  initialMode: PaletteMode;
  commands: QuickCommand[];
  document: DocumentView;
  onJump: (hit: FindHit) => void;
  onOpenHelp: (target: HelpTarget) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState(initialMode);
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const finding = mode === "document";
  const searching = finding || query.trim().length >= SEARCH_MIN_LENGTH;
  const documentSearch = useDocumentSearch(document, searching ? query : "");
  const entries = useMemo(() => helpEntries(document.formulaFunctions), [document.formulaFunctions]);
  const help = useMemo(
    () =>
      searching && !finding ? searchHelpEntries(entries, query, { typoTolerant: false }) : [],
    [entries, finding, query, searching]
  );
  const rows = useMemo(
    () =>
      paletteRows({
        mode,
        query,
        commands,
        groups: documentSearch.groups,
        help,
        onJump,
        onOpenHelp,
        onShowDocument: () => {
          setMode("document");
          setSelectedId(null);
          inputRef.current?.focus();
        },
      }),
    [commands, documentSearch.groups, help, mode, onJump, onOpenHelp, query]
  );
  const selected = rows.find((row) => row.id === selectedId) ?? rows[0] ?? null;

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const move = (offset: number) => {
    if (!rows.length) return;
    const current = Math.max(0, rows.findIndex((row) => row.id === selected?.id));
    const next = Math.min(rows.length - 1, Math.max(0, current + offset));
    setSelectedId(rows[next].id);
    window.document
      .getElementById(`palette-row-${rows[next].id}`)
      ?.scrollIntoView?.({ block: "nearest" });
  };

  const run = (row: PaletteRow) => {
    if (!row.keepsOpen) onClose();
    row.run();
  };

  const empty =
    !rows.length && (!finding || query.trim()) && !documentSearch.searching;
  return (
    <>
      {/* A press anywhere else is a decision to do something else; the
          palette used to stay up over it and the click landed underneath. */}
      <div className="quick-commands-backdrop" onPointerDown={onClose} />
      <section
        className="quick-commands"
        role="dialog"
        aria-modal="false"
        aria-label={finding ? "Find" : "Quick Commands"}
      >
        <label className="quick-commands-search">
          <Search size={15} aria-hidden="true" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setSelectedId(null);
            }}
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
            placeholder={finding ? "Find in document" : "Type a command or search"}
            aria-label={finding ? "Find in document" : "Search commands"}
            spellCheck={false}
          />
          {documentSearch.searching ? (
            <span className="quick-commands-status">Searching…</span>
          ) : (
            <kbd>{finding ? "⌘F" : "⇧⌘P"}</kbd>
          )}
        </label>
        <div
          className="quick-commands-results"
          role="listbox"
          aria-label={finding ? "Find results" : "Commands"}
        >
          {documentSearch.error && <p>{documentSearch.error}</p>}
          {rows.map((row, index) => (
            <button
              id={`palette-row-${row.id}`}
              key={row.id}
              role="option"
              aria-selected={row.id === selected?.id}
              className={[
                row.id === selected?.id ? "selected" : "",
                index > 0 && rows[index - 1].section !== row.section ? "section-start" : "",
              ].join(" ").trim()}
              onClick={() => run(row)}
              onMouseEnter={() => setSelectedId(row.id)}
            >
              <small title={row.group}>{row.group}</small>
              <span>{row.label}</span>
              {row.detail && <em title={row.detail}>{row.detail}</em>}
              {row.shortcut && <kbd>{row.shortcut}</kbd>}
            </button>
          ))}
          {empty && <p>No matches</p>}
        </div>
      </section>
    </>
  );
}
