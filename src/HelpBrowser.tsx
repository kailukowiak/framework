import { Check, Copy, Search, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import "./HelpBrowser.css";
import { writeClipboardText } from "./lib/clipboard";
import type { HelpRequest } from "./lib/helpPanel";
import {
  helpEntries,
  searchHelpEntries,
  type FormulaHelpEntry,
  type GuideHelpEntry,
  type HelpEntry,
} from "./lib/helpSearch";
import type { FormulaFunction } from "./lib/types";

type HelpBrowserProps = {
  request: HelpRequest;
  formulaFunctions: FormulaFunction[];
  canInsert: boolean;
  /**
   * How much of the right edge already belongs to the inspector. Help docks
   * beside it rather than over it, because the Wrangle editor someone is
   * asking about lives there.
   */
  rightInset?: string;
  onInsert: (formula: string) => void;
  onClose: () => void;
};

const MAX_RESULTS = 80;

type CopyState = { key: string; copied: boolean } | null;

/**
 * Help docked at the right edge, not floated over the canvas. Most help is
 * asked for halfway through a formula, and a formula is written by clicking
 * the frames it reads: a panel over the canvas hid the very columns the
 * person was about to point at. Docked, the canvas stays where it was and
 * clickable, Insert can leave the panel open, and help stays beside the work
 * until it is dismissed.
 */
export function HelpBrowser({
  request,
  formulaFunctions,
  canInsert,
  rightInset = "0px",
  onInsert,
  onClose,
}: HelpBrowserProps) {
  const [query, setQuery] = useState(request.query);
  const [selectedId, setSelectedId] = useState<string | null>(request.entryId ?? null);
  const [copyState, setCopyState] = useState<CopyState>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const resetCopy = useRef<number | null>(null);
  const entries = useMemo(() => helpEntries(formulaFunctions), [formulaFunctions]);
  const results = useMemo(
    () => searchHelpEntries(entries, query).slice(0, MAX_RESULTS),
    [entries, query]
  );
  // An entry opened by id — the function under the cursor, a Related link —
  // stays open even when the search no longer lists it, so following a link
  // never throws away the list the person was reading.
  const selected =
    entries.find((entry) => entry.id === selectedId) ?? results[0] ?? null;

  // Every request retargets the panel, including one that arrives while it
  // is already open: ⌘K inside a different function opens that function.
  useEffect(() => {
    setQuery(request.query);
    setSelectedId(request.entryId ?? null);
    if (request.keepFocus) return;
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [request.serial, request.query, request.entryId, request.keepFocus]);

  useEffect(
    () => () => {
      if (resetCopy.current !== null) window.clearTimeout(resetCopy.current);
    },
    []
  );

  const selectByOffset = (offset: number) => {
    if (!results.length) return;
    const current = results.findIndex((entry) => entry.id === selected?.id);
    const next =
      current < 0 ? 0 : Math.min(results.length - 1, Math.max(0, current + offset));
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
    const entry = entries.find((candidate) => candidate.title === title);
    if (entry) setSelectedId(entry.id);
    else setQuery(title);
  };

  // Escape is handled on the panel itself. A keydown raised inside it did not
  // reach a window listener in the packaged app, and a docked panel should
  // not answer an Escape pressed somewhere else anyway: on the canvas that key
  // already means "stop editing this cell".
  return (
    <aside
      className="help-browser"
      style={{ "--help-right": rightInset } as CSSProperties}
      aria-label="FrameWork Reference"
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.preventDefault();
        onClose();
      }}
    >
      <label className="help-search">
        <Search size={13} aria-hidden="true" />
        <input
          ref={inputRef}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setSelectedId(null);
          }}
          onKeyDown={(event) => {
            if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
            event.preventDefault();
            selectByOffset(event.key === "ArrowDown" ? 1 : -1);
          }}
          placeholder="sum, vlookup, previous row, pivot…"
          aria-label="Search help"
          spellCheck={false}
        />
        <button type="button" aria-label="Close help" onClick={onClose}>
          <X size={13} />
        </button>
      </label>
      <div className="help-results" role="listbox" aria-label="Help results">
        {results.map((entry) => (
          <HelpResult
            key={entry.id}
            entry={entry}
            selected={entry.id === selected?.id}
            onSelect={() => setSelectedId(entry.id)}
          />
        ))}
        {!results.length && (
          <p className="help-empty">
            No answer. Try the job, an Excel name, or words like “previous row”.
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
    </aside>
  );
}

function entryKind(entry: HelpEntry): string {
  return entry.kind === "Function"
    ? entry.function.category
    : entry.guide.category ?? entry.kind;
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
  return (
    <button
      id={`help-result-${entry.id}`}
      type="button"
      role="option"
      aria-selected={selected}
      className={selected ? "selected" : ""}
      title={entry.summary}
      onClick={onSelect}
    >
      <strong className={entry.kind === "Function" ? "help-code" : ""}>{entry.title}</strong>
      <small>{entryKind(entry)}</small>
    </button>
  );
}

/** A copyable formula, with Insert when a formula cursor can take it. */
function HelpExample({
  label,
  code,
  copyKey,
  canInsert,
  copyState,
  onCopy,
  onInsert,
}: {
  label?: string;
  code: string;
  copyKey: string;
  canInsert: boolean;
  copyState: CopyState;
  onCopy: (key: string, text: string) => void;
  onInsert: (formula: string) => void;
}) {
  const copied = copyState?.key === copyKey ? copyState : null;
  return (
    <div className="help-example">
      {label && <span>{label}</span>}
      <code>{code}</code>
      <button type="button" onClick={() => onCopy(copyKey, code)}>
        {copied?.copied ? <Check size={12} /> : <Copy size={12} />}
        {copied ? (copied.copied ? "Copied" : "Copy failed") : "Copy"}
      </button>
      {canInsert && (
        <button type="button" onClick={() => onInsert(code)}>
          Insert
        </button>
      )}
    </div>
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
  copyState: CopyState;
  onCopy: (key: string, text: string) => void;
  onInsert: (formula: string) => void;
}) {
  const fn = entry.function;
  return (
    <>
      <span className="eyebrow">{fn.category}</span>
      <h2><code>{fn.signature}</code></h2>
      <p className="help-summary">{fn.description}</p>
      <HelpExample
        code={entry.copyText}
        copyKey={`function:${entry.id}`}
        canInsert={canInsert}
        copyState={copyState}
        onCopy={onCopy}
        onInsert={onInsert}
      />
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
  copyState: CopyState;
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
          {guide.examples.map((example, index) => (
            <HelpExample
              key={example.code}
              label={example.label}
              code={example.code}
              copyKey={`${guide.id}:${index}`}
              canInsert={canInsert}
              copyState={copyState}
              onCopy={onCopy}
              onInsert={onInsert}
            />
          ))}
        </section>
      )}
      {guide.related && (
        <section className="help-related">
          <h3>Related</h3>
          {guide.related.map((title) => (
            <button type="button" key={title} onClick={() => onRelated(title)}>{title}</button>
          ))}
        </section>
      )}
    </>
  );
}
