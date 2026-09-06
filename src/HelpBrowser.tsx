import { Check, Copy, Search, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import "./HelpBrowser.css";
import { writeClipboardText } from "./lib/clipboard";
import type { HelpScope } from "./lib/helpContent";
import {
  helpEntries,
  searchHelpEntries,
  type FormulaHelpEntry,
  type GuideHelpEntry,
  type HelpEntry,
} from "./lib/helpSearch";
import type { FormulaFunction } from "./lib/types";

type HelpBrowserProps = {
  scope: HelpScope;
  formulaFunctions: FormulaFunction[];
  canInsert: boolean;
  onScopeChange: (scope: HelpScope) => void;
  onInsert: (formula: string) => void;
  onClose: () => void;
};

const MAX_RESULTS = 80;

export function HelpBrowser({
  scope,
  formulaFunctions,
  canInsert,
  onScopeChange,
  onInsert,
  onClose,
}: HelpBrowserProps) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [copyState, setCopyState] = useState<{ key: string; copied: boolean } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const resetCopy = useRef<number | null>(null);
  const entries = useMemo(
    () => helpEntries(scope, formulaFunctions),
    [formulaFunctions, scope]
  );
  const results = useMemo(
    () => searchHelpEntries(entries, query).slice(0, MAX_RESULTS),
    [entries, query]
  );
  const selected =
    results.find((entry) => entry.id === selectedId) ?? results[0] ?? null;

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [scope]);

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected, selectedId]);

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  useEffect(
    () => () => {
      if (resetCopy.current !== null) window.clearTimeout(resetCopy.current);
    },
    []
  );

  const selectByOffset = (offset: number) => {
    if (!results.length) return;
    const current = Math.max(0, results.findIndex((entry) => entry.id === selected?.id));
    const next = Math.min(results.length - 1, Math.max(0, current + offset));
    setSelectedId(results[next].id);
    document.getElementById(`help-result-${results[next].id}`)?.scrollIntoView({ block: "nearest" });
  };

  const copy = async (key: string, text: string) => {
    const copied = await writeClipboardText(text);
    setCopyState({ key, copied });
    if (resetCopy.current !== null) window.clearTimeout(resetCopy.current);
    resetCopy.current = window.setTimeout(() => setCopyState(null), 1600);
  };

  const followRelated = (title: string) => {
    setQuery(title);
    inputRef.current?.focus();
  };

  return (
    <section
      className="help-browser"
      role="dialog"
      aria-modal="false"
      aria-label="FrameWork Reference"
    >
      <HelpSearchHeader
        scope={scope}
        query={query}
        inputRef={inputRef}
        onScopeChange={onScopeChange}
        onQueryChange={setQuery}
        onMove={selectByOffset}
        onClose={onClose}
      />

      <div className="help-browser-body">
        <div className="help-results" role="listbox" aria-label="Help results">
          {results.length ? (
            results.map((entry) => (
              <HelpResult
                key={entry.id}
                entry={entry}
                selected={entry.id === selected?.id}
                onSelect={() => setSelectedId(entry.id)}
              />
            ))
          ) : (
            <p className="help-empty">
              No answer yet. Try the job you want to do, an Excel name, or words such as
              “vector”, “live”, or “previous row”.
            </p>
          )}
        </div>
        <article className="help-detail" aria-live="polite">
          {selected?.kind === "Function" && (
            <FunctionDetail
              entry={selected}
              canInsert={canInsert}
              copyState={copyState}
              onCopy={copy}
              onInsert={onInsert}
            />
          )}
          {selected && selected.kind !== "Function" && (
            <GuideDetail
              entry={selected}
              canInsert={canInsert}
              copyState={copyState}
              onCopy={copy}
              onInsert={onInsert}
              onRelated={followRelated}
            />
          )}
        </article>
      </div>
    </section>
  );
}

function HelpSearchHeader({
  scope,
  query,
  inputRef,
  onScopeChange,
  onQueryChange,
  onMove,
  onClose,
}: {
  scope: HelpScope;
  query: string;
  inputRef: RefObject<HTMLInputElement | null>;
  onScopeChange: (scope: HelpScope) => void;
  onQueryChange: (query: string) => void;
  onMove: (offset: number) => void;
  onClose: () => void;
}) {
  return (
    <>
      <header className="help-browser-header">
        <nav aria-label="Help sections">
          <button
            className={scope === "formulas" ? "active" : ""}
            aria-pressed={scope === "formulas"}
            onClick={() => onScopeChange("formulas")}
          >
            Formulas
          </button>
          <button
            className={scope === "guide" ? "active" : ""}
            aria-pressed={scope === "guide"}
            onClick={() => onScopeChange("guide")}
          >
            How FrameWork works
          </button>
        </nav>
        <button className="icon-button" aria-label="Close help" onClick={onClose}>
          <X size={18} />
        </button>
      </header>
      <label className="help-search">
        <Search size={16} aria-hidden="true" />
        <input
          ref={inputRef}
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
            event.preventDefault();
            onMove(event.key === "ArrowDown" ? 1 : -1);
          }}
          placeholder={
            scope === "formulas"
              ? "Search formulas, jobs, or Excel names…"
              : "Ask how something works…"
          }
          aria-label={scope === "formulas" ? "Search formulas" : "Search help"}
          spellCheck={false}
        />
        {query && (
          <button aria-label="Clear help search" onClick={() => onQueryChange("")}>
            <X size={14} />
          </button>
        )}
      </label>
    </>
  );
}

function HelpResult({
  entry,
  selected,
  onSelect,
}: {
  entry: HelpEntry;
  selected: boolean;
  onSelect: () => void;
}) {
  const label =
    entry.kind === "Function"
      ? entry.function.category
      : entry.guide.category ?? entry.kind;
  return (
    <button
      id={`help-result-${entry.id}`}
      type="button"
      role="option"
      aria-selected={selected}
      className={selected ? "selected" : ""}
      onClick={onSelect}
    >
      <span>{label}</span>
      <strong>{entry.title}</strong>
      <small>{entry.summary}</small>
    </button>
  );
}

function FunctionDetail({
  entry,
  canInsert,
  copyState,
  onCopy,
  onInsert,
}: {
  entry: FormulaHelpEntry;
  canInsert: boolean;
  copyState: { key: string; copied: boolean } | null;
  onCopy: (key: string, text: string) => void;
  onInsert: (formula: string) => void;
}) {
  const fn = entry.function;
  const key = `function:${entry.id}`;
  return (
    <>
      <span className="eyebrow">{fn.category}</span>
      <h2><code>{fn.signature}</code></h2>
      <p className="help-summary">{fn.description}</p>
      <div className="help-function-example">
        <code>{entry.copyText}</code>
        <CopyButton
          state={copyState}
          copyKey={key}
          onClick={() => onCopy(key, entry.copyText)}
        />
      </div>
      <div className="help-detail-actions">
        {canInsert && (
          <button className="primary-action" onClick={() => onInsert(entry.copyText)}>
            Insert at formula cursor
          </button>
        )}
      </div>
      <dl className="help-facts">
        <div><dt>Returns</dt><dd>{fn.returnType}</dd></div>
        <div><dt>Nulls</dt><dd>{fn.nullBehavior}</dd></div>
        <div><dt>Use in</dt><dd>{entry.surfaces.join(" · ")}</dd></div>
        {fn.aliases.length > 0 && (
          <div><dt>Also find with</dt><dd>{fn.aliases.join(" · ")}</dd></div>
        )}
      </dl>
      {fn.arguments.length > 0 && (
        <section className="help-arguments">
          <h3>Arguments</h3>
          {fn.arguments.map((argument) => (
            <div key={argument.name}>
              <code>{argument.name}</code>
              <p>
                {argument.required ? "Required. " : "Optional. "}
                {argument.description}
                {argument.example && <> Try <code>{argument.example}</code>.</>}
              </p>
            </div>
          ))}
        </section>
      )}
    </>
  );
}

function GuideDetail({
  entry,
  canInsert,
  copyState,
  onCopy,
  onInsert,
  onRelated,
}: {
  entry: GuideHelpEntry;
  canInsert: boolean;
  copyState: { key: string; copied: boolean } | null;
  onCopy: (key: string, text: string) => void;
  onInsert: (formula: string) => void;
  onRelated: (title: string) => void;
}) {
  const { guide } = entry;
  return (
    <>
      <span className="eyebrow">
        {guide.category ? `${guide.category} · ${guide.kind}` : guide.kind}
      </span>
      <h2>{guide.title}</h2>
      <p className="help-summary">{guide.summary}</p>
      {guide.surfaces && (
        <p className="help-surfaces"><strong>Use in</strong> {guide.surfaces.join(" · ")}</p>
      )}
      {guide.body.map((paragraph) => <p key={paragraph}>{paragraph}</p>)}
      {guide.facts && (
        <dl className="help-facts">
          {guide.facts.map((fact) => (
            <div key={fact.term}>
              <dt>{fact.term}</dt>
              <dd>{fact.description}</dd>
            </div>
          ))}
        </dl>
      )}
      {guide.steps && (
        <section className="help-steps">
          <h3>Do this</h3>
          <ol>{guide.steps.map((step) => <li key={step}>{step}</li>)}</ol>
        </section>
      )}
      {guide.examples && (
        <section className="help-examples">
          <h3>Examples</h3>
          {guide.examples.map((example, index) => {
            const key = `${guide.id}:${index}`;
            return (
              <div key={example.code}>
                <span>{example.label}</span>
                <code>{example.code}</code>
                <CopyButton state={copyState} copyKey={key} onClick={() => onCopy(key, example.code)} />
                {canInsert && (
                  <button onClick={() => onInsert(example.code)}>Insert</button>
                )}
              </div>
            );
          })}
        </section>
      )}
      {guide.related && (
        <section className="help-related">
          <h3>Related</h3>
          {guide.related.map((title) => (
            <button key={title} onClick={() => onRelated(title)}>{title}</button>
          ))}
        </section>
      )}
    </>
  );
}

function CopyButton({
  state,
  copyKey,
  onClick,
}: {
  state: { key: string; copied: boolean } | null;
  copyKey: string;
  onClick: () => void;
}) {
  const current = state?.key === copyKey ? state : null;
  return (
    <button className="secondary-action help-copy" onClick={onClick}>
      {current?.copied ? <Check size={14} /> : <Copy size={14} />}
      {current ? (current.copied ? "Copied" : "Copy failed") : "Copy"}
    </button>
  );
}
