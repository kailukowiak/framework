/**
 * ⌘F: one field, every match in the document, Enter to go there.
 *
 * Modelled on `HelpBrowser` — a query, a memoised list, arrow keys, Enter,
 * Escape — but without its detail pane. A match already says everything there
 * is to say about itself in two short strings, so a second column showing one
 * of them larger would be a card holding one number.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { useDocumentSearch } from "./hooks/useDocumentSearch";
import type { FindHit } from "./lib/documentSearch";
import type { DocumentView, Selection } from "./lib/types";
import type { GridFocus } from "./FrameGrid";

export type FindPaletteProps = {
  document: DocumentView | null;
  /**
   * What to search for on open, when Find was reached from somewhere that
   * already had a query — ⌘⇧P's last row, which hands over a phrase that
   * named no command. Selected, so typing replaces it.
   */
  initialQuery?: string;
  onJumpToObject: (objectId: string) => void;
  onFocusCell: (hit: FindHit) => void;
  onFocusColumn: (hit: FindHit) => void;
  onClose: () => void;
};

export function FindPalette({
  document,
  initialQuery,
  onJumpToObject,
  onFocusCell,
  onFocusColumn,
  onClose,
}: FindPaletteProps) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const { hits, groups, searching, error } = useDocumentSearch(document, query);
  const selected = useMemo(
    () => hits.find((hit) => hit.id === selectedId) ?? hits[0] ?? null,
    [hits, selectedId]
  );

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected, selectedId]);

  const selectByOffset = (offset: number) => {
    if (!hits.length) return;
    const current = Math.max(0, hits.findIndex((hit) => hit.id === selected?.id));
    const next = Math.min(hits.length - 1, Math.max(0, current + offset));
    setSelectedId(hits[next].id);
    window.document
      .getElementById(`find-result-${hits[next].id}`)
      ?.scrollIntoView({ block: "nearest" });
  };

  // Where a hit takes you depends on how specific it is: a cell knows its row
  // and column, a column knows its column, and everything else is a thing on
  // the canvas. Each callback does the jumping; the palette only decides which
  // question was answered.
  const jump = (hit: FindHit) => {
    if (hit.kind === "cell") onFocusCell(hit);
    else if (hit.kind === "column") onFocusColumn(hit);
    else onJumpToObject(hit.objectId);
    onClose();
  };

  return (
    <section className="find-palette" role="dialog" aria-modal="false" aria-label="Find">
      <label className="find-palette-search">
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
              selectByOffset(event.key === "ArrowDown" ? 1 : -1);
            } else if (event.key === "Enter" && selected) {
              event.preventDefault();
              jump(selected);
            }
          }}
          placeholder="Find in document"
          aria-label="Find in document"
          spellCheck={false}
        />
        {searching && <span className="find-palette-status">Searching…</span>}
      </label>
      <div className="find-palette-results" role="listbox" aria-label="Find results">
        {error && <p className="find-palette-empty">{error}</p>}
        {!error && query.trim() && !hits.length && !searching && (
          <p className="find-palette-empty">No matches</p>
        )}
        {groups.map((group) => (
          <div className="find-palette-group" key={group.objectId}>
            <strong>{group.name}</strong>
            {group.hits.map((hit) => (
              <button
                key={hit.id}
                id={`find-result-${hit.id}`}
                role="option"
                aria-selected={hit.id === selected?.id}
                className={hit.id === selected?.id ? "selected" : ""}
                onClick={() => jump(hit)}
                onMouseEnter={() => setSelectedId(hit.id)}
              >
                <span>{hit.label}</span>
                <small>{hit.snippet}</small>
              </button>
            ))}
          </div>
        ))}
      </div>
    </section>
  );
}

/**
 * The palette plus the one thing it cannot know: how this application moves
 * to a cell.
 *
 * It exists so `App.tsx` spends a line on Find rather than a paragraph. A
 * jump is two facts — bring the card forward, then put the cursor in it — and
 * both are already implemented; this only says which hit needs which.
 */
export function FindPaletteHost({
  document,
  initialQuery,
  jumpToObject,
  setSelection,
  setGridFocus,
  onClose,
}: {
  document: DocumentView | null;
  initialQuery?: string;
  jumpToObject: (objectId: string) => void;
  setSelection: (selection: Selection | null) => void;
  setGridFocus: (focus: GridFocus | null) => void;
  onClose: () => void;
}) {
  const focus = (hit: FindHit) => {
    jumpToObject(hit.objectId);
    if (!hit.columnId) return;
    // A cell hit knows a row, so the grid gets a cursor; a column hit knows
    // only the column, and putting a cursor on an arbitrary row of it would
    // be inventing an answer the search never gave.
    if (hit.viewId && hit.rowId) {
      setGridFocus({
        viewId: hit.viewId,
        objectId: hit.objectId,
        rowId: hit.rowId,
        columnId: hit.columnId,
        mode: "navigate",
        editSeed: null,
        anchor: null,
        span: null,
        rowIndex: hit.rowIndex,
      });
    }
    setSelection({
      objectId: hit.objectId,
      viewId: hit.viewId,
      rowId: hit.rowId,
      columnId: hit.columnId,
    });
  };

  return (
    <FindPalette
      document={document}
      initialQuery={initialQuery}
      onJumpToObject={jumpToObject}
      onFocusCell={focus}
      onFocusColumn={focus}
      onClose={onClose}
    />
  );
}
