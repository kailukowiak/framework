import { ChevronRight } from "lucide-react";
import {
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useFormulaEditorRegistration } from "./ActiveFormulaEditor";
import { BlockCompletionMenu } from "./BlockCompletionMenu";
import { NumberDisplayContext } from "./FrameGrid";
import {
  ScratchworkResultViewer,
  scratchworkResultIsLong,
} from "./ScratchworkResultViewer";
import { scalarFormulaReferences, takenWhen } from "./ScalarCards";
import { carryCaret } from "./lib/carryCaret";
import { formatComputedScalar } from "./lib/columnFormatting";
import { acceptCompletionOnce } from "./lib/completionAcceptance";
import { blockFormulaReferences } from "./lib/blockFormulaReferences";
import { formulaReferenceDecorations } from "./lib/formulaReferenceDecorations";
import { logicalLineIndexAt, scratchworkStripeRows } from "./lib/blockLines";
import {
  contextualFormulaReferenceCompletion,
  contextualFormulaReferenceToken,
  getFormulaReferenceQuery,
  insertFormulaReference,
  isFormulaExecuteShortcut,
  type FormulaReference,
} from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import {
  continueScratchworkLine,
  mergeStoredScratchwork,
} from "./lib/scratchwork";
import type {
  BlockObject,
  ComputedBlock,
  ComputedBlockLine,
  ComputedFrame,
  DataObject,
  FormulaFunction,
} from "./lib/types";

/**
 * The scratchpad: one text surface, and every line's answer beside it.
 *
 * This is a text editor rather than a stack of formula fields, and the
 * difference is the whole point of the object. A block exists to solve a
 * density problem — forty scratch calculations should not be forty cards —
 * and a card that spent a labelled field, a delete button and an Execute
 * button on every line would be forty cards again, stacked. So: type down
 * the page, one calculation per line, answers in the gutter.
 *
 * `x = 10` names a line as it defines it, siblings above resolve bare, and
 * a line that does not parse yet keeps its text and says why in its own
 * gutter — see `BlockLine` in the core for why that leniency is confined to
 * this one surface.
 */
export function BlockCardPreview({
  block,
  computed,
}: {
  block: BlockObject;
  computed: ComputedBlock | undefined;
}) {
  const useGrouping = useContext(NumberDisplayContext);
  const lines = computed?.lines ?? [];
  return (
    <div className="value-card block-card block-card-preview" aria-hidden>
      <span className="block-preview-name">{block.name}</span>
      <div className="block-sheet">
        <pre className="block-preview-source">{computed?.source}</pre>
        <div className="block-gutter">
          {lines.map((line) => {
            const span = line.text.split("\n").length;
            return (
              <div
                key={line.id}
                className={`block-gutter-row${line.error ? " failed" : ""}${
                  line.frozen ? (line.frozen.stale ? " stale" : " frozen") : ""
                }`}
                style={
                  span > 1
                    ? { height: `calc(var(--text-sm) * 1.7 * ${span})` }
                    : undefined
                }
              >
                {line.blank || line.comment
                  ? ""
                  : line.error
                    ? "!"
                    : formatComputedScalar(
                        line.typedValue,
                        line.dataType,
                        line.display,
                        useGrouping
                      )}
              </div>
            );
          })}
        </div>
      </div>
      <small>{lineSummary(lines)}</small>
    </div>
  );
}

export function BlockCard({
  block,
  computed,
  focusToken,
  objects,
  computedFrames,
  formulaFunctions,
  onOperation,
  onFreeze,
}: {
  block: BlockObject;
  computed: ComputedBlock | undefined;
  /** Bumped when ⌘J asks this block for the cursor. */
  focusToken?: number;
  objects: DataObject[];
  computedFrames: Record<string, ComputedFrame>;
  formulaFunctions: FormulaFunction[];
  onOperation: OperationHandler;
  onFreeze: (objectId: string) => Promise<void>;
}) {
  const stored = computed?.source ?? "";
  const [draft, setDraft] = useState(stored);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState(0);
  const [picking, setPicking] = useState(0);
  const [focused, setFocused] = useState(false);
  const [openResultId, setOpenResultId] = useState<string | null>(null);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const gutter = useRef<HTMLDivElement>(null);
  const highlight = useRef<HTMLPreElement>(null);
  const stripes = useRef<HTMLDivElement>(null);
  // The text last handed to the document. What separates "the author has
  // typed since" from "this is our own edit coming back".
  const sent = useRef(stored);
  // The line said to be under the cursor when that text was sent, so we know
  // whether there is a name still being held open down there.
  const sentEditing = useRef<number | null>(null);
  // Sends that have not been answered yet. `stored` is one edit behind for
  // as long as one is outstanding, and text one edit behind is not a
  // correction to accept -- it is our own previous send, which would undo
  // whatever has been typed since.
  const inflight = useRef(0);
  const pending = useRef<number | undefined>(undefined);
  const latestCommit = useRef<{
    source: string;
    editing: number | null;
    promise: Promise<void>;
  } | null>(null);

  const commit = useCallback(
    (source: string, editing: number | null): Promise<void> => {
      window.clearTimeout(pending.current);
      if (source === sent.current && editing === sentEditing.current)
        return latestCommit.current?.promise ?? Promise.resolve();
      sent.current = source;
      sentEditing.current = editing;
      inflight.current += 1;
      const promise = (async () => {
        try {
          setError(
            await onOperation(
              { type: "setBlockSource", blockId: block.id, source, editing },
              { inlineError: true }
            )
          );
        } finally {
          inflight.current -= 1;
        }
      })();
      latestCommit.current = { source, editing, promise };
      return promise;
    },
    [block.id, onOperation]
  );

  // Live, on a pause rather than a keystroke: every send is an undo step, and
  // an undo step per character would bury the history. A pause is also what
  // the eye reads as "I have finished saying that".
  //
  // The line being typed on goes with it. `revenue` on its way to `revenue10`
  // is not a rename, and the document holds the old name until told the
  // cursor has moved off.
  const type = (source: string, at: number) => {
    setDraft(source);
    const line = logicalLineIndexAt(source, at);
    window.clearTimeout(pending.current);
    pending.current = window.setTimeout(() => void commit(source, line), 350);
  };
  useEffect(() => () => window.clearTimeout(pending.current), []);

  // The document may hand back text that is not what was sent: renaming a
  // line rewrites the lines that read it. Taken while the author is typing,
  // because the alternative is their next edit sending the old name back and
  // breaking the reference that was just repaired -- so the caret is carried
  // across the change rather than left at the end or counted again.
  const caret = useRef<{ at: number; from: string } | null>(null);
  useEffect(() => {
    if (inflight.current > 0) return;
    if (stored === sent.current) return;
    if (draft !== sent.current) return;
    // The line the cursor is in is the author's. The document is holding
    // that line's old name on purpose and would hand it straight back,
    // undoing the name they are halfway through typing. Every other line is
    // the document's to correct.
    const next = mergeStoredScratchwork(stored, draft, sentEditing.current);
    if (next === draft) return;
    const node = textarea.current;
    caret.current = node ? { at: node.selectionStart, from: draft } : null;
    sent.current = next;
    setDraft(next);
  }, [stored, draft]);
  useLayoutEffect(() => {
    const held = caret.current;
    if (!held) return;
    caret.current = null;
    const node = textarea.current;
    if (!node || window.document.activeElement !== node) return;
    const at = carryCaret(held.from, draft, held.at);
    node.setSelectionRange(at, at);
    setCursor(at);
  }, [draft]);

  useEffect(() => {
    if (focusToken === undefined) return;
    const node = textarea.current;
    if (!node) return;
    node.focus();
    node.setSelectionRange(node.value.length, node.value.length);
  }, [focusToken]);

  // Leaving the line is what finishes naming it. Said once, when the cursor
  // goes somewhere else, so the lines that read this one are rewritten to the
  // name that was meant rather than to each prefix of it on the way past.
  useEffect(() => {
    if (!focused || sentEditing.current === null) return;
    if (sentEditing.current === logicalLineIndexAt(draft, cursor)) return;
    void commit(draft, null);
  }, [draft, cursor, focused, commit]);

  // The mirror put back under the text, from wherever the text now is.
  //
  // Scrolling is not the only thing that moves a scroll position: shortening
  // a line moves it too, because the browser clamps whatever is scrolled to
  // whatever there is left to scroll. The two elements clamp separately --
  // the text area has a scrollbar and the mirror does not, so they do not
  // even agree on where the end is -- and no scroll event is sent for a
  // clamp. Typing past the right edge and then deleting back was enough to
  // leave the highlighted text a few characters away from the caret drawing
  // it, which is the offset that shows up while editing and never while
  // reading.
  const align = useCallback(() => {
    const node = textarea.current;
    if (!node) return;
    if (highlight.current) {
      highlight.current.scrollTop = node.scrollTop;
      highlight.current.scrollLeft = node.scrollLeft;
    }
    if (stripes.current) stripes.current.scrollTop = node.scrollTop;
    if (gutter.current) gutter.current.scrollTop = node.scrollTop;
  }, []);
  // After the text is laid out rather than after it is set, so the mirror is
  // never painted at the old position for a frame.
  useLayoutEffect(align, [align, draft]);

  const lines = useMemo(() => computed?.lines ?? [], [computed]);
  const stripeRows = useMemo(
    () => scratchworkStripeRows(draft, lines.map((line) => line.id)),
    [draft, lines]
  );
  const openResult = lines.find((line) => line.id === openResultId && !line.error);
  useEffect(() => {
    if (openResultId !== null && !openResult) setOpenResultId(null);
  }, [openResult, openResultId]);
  const failing = lines.find((line) => line.error);
  // A line whose answer was written down and whose sources have moved since.
  // Reported, never repaired: a recorded number changing on its own is the
  // failure freezing exists to avoid.
  const stale = lines.find((line) => line.frozen?.stale);

  // What this line may name: the lines above it, bare, and then everything
  // on the canvas. Same catalog as any other formula — the scratchpad gets
  // the whole language, not a calculator subset of it.
  const outside = useMemo(
    () => scalarFormulaReferences(objects, formulaFunctions, computedFrames, block.id),
    [objects, formulaFunctions, computedFrames, block.id]
  );
  const currentLine = logicalLineIndexAt(draft, cursor);
  const query = getFormulaReferenceQuery(draft, cursor);
  const available = useMemo(
    () => blockFormulaReferences(lines, currentLine, outside, block.id),
    [block.id, currentLine, lines, outside]
  );
  // Inside an unclosed backtick, only names. Backticks are what this
  // language uses to say "this is something on the canvas", so offering
  // `.list.arg_max` to somebody who has typed `` `List `` answers a
  // question they did not ask — and buries the list they are reaching for.
  const naming = query.startsWith("`");
  const offered = naming
    ? available.filter((reference) => reference.kind !== "function")
    : available;
  const contextualCompletion = contextualFormulaReferenceCompletion(
    offered, draft, cursor, query
  );
  const registration = useFormulaEditorRegistration({
    id: `scratchwork:${block.id}`,
    label: block.name,
    kind: "scratchwork",
    draft,
    completion: { references: offered },
    onChange: (source, selection) => {
      type(source, selection.end);
      setCursor(selection.end);
      setPicking(0);
    },
    onSelection: (selection) => {
      setCursor(selection.end);
      setPicking(0);
    },
    onCommit: (source) => commit(source, null),
    onFocus: (selection) => {
      window.requestAnimationFrame(() => {
        textarea.current?.focus();
        textarea.current?.setSelectionRange(selection.start, selection.end);
      });
    },
  });
  const suggestions = !focused
    ? []
    : contextualCompletion.suggestions.slice(0, 6);
  const offering =
    focused &&
    (contextualCompletion.qualifier !== undefined || query.trim().length > 0) &&
    suggestions.length > 0;

  // What a name has to match to be drawn as one. Normalized the way the
  // parser matches names, so what looks real is what resolves.
  const knownNames = useMemo(() => {
    const names = new Set<string>();
    for (const reference of available) {
      for (const part of reference.label.split(".")) names.add(nameKey(part));
    }
    return names;
  }, [available]);

  const acceptedAt = useRef<string | null>(null);
  const take = (reference: FormulaReference) => {
    const node = textarea.current;
    const at = node?.selectionStart ?? cursor;
    acceptCompletionOnce(acceptedAt, `${draft}\u0000${at}`, () => {
      const token = contextualFormulaReferenceToken(
        reference,
        contextualCompletion.qualifier
      );
      const result = insertFormulaReference(draft, at, token);
      type(result.source, result.cursor);
      setCursor(result.cursor);
      registration.update(result.source, {
        start: result.cursor,
        end: result.cursor,
      });
      window.requestAnimationFrame(() => {
        node?.focus();
        node?.setSelectionRange(result.cursor, result.cursor);
      });
    });
  };

  return (
    <div className="value-card block-card">
      <input
        className="object-name-input"
        defaultValue={block.name}
        key={block.name}
        onBlur={(event) => {
          if (event.target.value !== block.name)
            onOperation({ type: "renameObject", objectId: block.id, name: event.target.value });
        }}
      />
      <div className="block-sheet">
        <div className="block-editor">
          {/* A mirror of the text, behind a textarea painted transparent.
              A textarea cannot style its own contents, and the alternative
              — a contenteditable — brings its own selection and undo
              behaviour, which is a much larger thing to own than one
              element that has to keep the same metrics as another. */}
          {/* Bands under the text, one per logical line: what makes a
              three-row formula read as one thing and its neighbour as the
              next. Behind the highlight mirror, which has no background. */}
          <div className="block-stripes" aria-hidden ref={stripes}>
            {stripeRows.map((row) => (
              <div
                key={row.id}
                className={`block-stripe${row.on ? " on" : ""}`}
                style={
                  row.span > 1
                    ? { height: `calc(var(--text-sm) * 1.7 * ${row.span})` }
                    : undefined
                }
              />
            ))}
          </div>
          <pre className="block-highlight" aria-hidden ref={highlight}>
            {highlightSource(draft, knownNames, available)}
          </pre>
        <textarea
          ref={textarea}
          className="block-source"
          aria-label={`${block.name} lines`}
          spellCheck={false}
          value={draft}
          placeholder={"x = 10\ny = 30\nx + y"}
          onChange={(event) => {
            type(event.target.value, event.target.selectionStart);
            setCursor(event.target.selectionStart);
            setPicking(0);
            registration.change(event.target.value, event.currentTarget);
          }}
          onSelect={(event) => {
            setCursor(event.currentTarget.selectionStart);
            registration.select(event.currentTarget);
          }}
          onFocus={(event) => {
            setFocused(true);
            registration.activate(event.currentTarget);
          }}
          onBlur={() => {
            setFocused(false);
            registration.blur();
            void commit(draft, null);
          }}
          // Tab takes the suggestion, and Enter is left alone: in a surface
          // made of lines, the key that makes a new one cannot be borrowed.
          // Alt+Return expands one calculation within visible parentheses.
          // That makes indentation layout rather than an invisible boundary.
          onKeyDown={(event) => {
            if (event.key === "Enter" && event.altKey) {
              event.preventDefault();
              const node = event.currentTarget;
              const expanded = continueScratchworkLine(draft, node.selectionStart, node.selectionEnd);
              type(expanded.source, expanded.selection.start);
              setCursor(expanded.selection.start);
              registration.update(expanded.source, expanded.selection);
              window.requestAnimationFrame(() => {
                node.focus();
                node.setSelectionRange(expanded.selection.start, expanded.selection.end);
              });
              return;
            }
            if (isFormulaExecuteShortcut(event)) {
              event.preventDefault();
              void registration.commit();
              return;
            }
            if (!offering) return;
            if (event.key === "Escape") {
              setFocused(false);
              return;
            }
            if (event.key === "ArrowDown" || event.key === "ArrowUp") {
              event.preventDefault();
              setPicking(
                (current) =>
                  (current + (event.key === "ArrowDown" ? 1 : suggestions.length - 1)) %
                  suggestions.length
              );
              return;
            }
            if (event.key === "Tab") {
              event.preventDefault();
              const suggestion = suggestions[picking] ?? suggestions[0];
              if (suggestion) take(suggestion);
            }
          }}
          onScroll={align}
        />
        </div>
        {/* One row per line, in the same order and at the same line height,
            which is what keeps an answer beside the thing it answers. The
            text does not wrap for the same reason. */}
        <div className="block-gutter" ref={gutter} aria-label="Scratchwork answers">
          {lines.map((line, index) => (
            <GutterRow
              key={line.id}
              line={line}
              span={stripeRows[index]?.span ?? 1}
              banded={stripeRows[index]?.on ?? false}
              open={openResultId === line.id}
              onToggle={() =>
                setOpenResultId((current) => (current === line.id ? null : line.id))
              }
            />
          ))}
        </div>
      </div>
      {openResult && (
        <ScratchworkResultViewer
          blockId={block.id}
          line={openResult}
          onClose={() => setOpenResultId(null)}
        />
      )}
      {offering && (
        <BlockCompletionMenu
          anchorRef={textarea}
          suggestions={suggestions}
          activeIndex={picking}
          onTake={take}
        />
      )}
      {/* The reason, not a count of reasons. A card that says "1 line cannot
          be worked out" and keeps the explanation in a tooltip has told you
          the one thing you already knew. */}
      {error ? (
        <small className="result-error">{error}</small>
      ) : failing ? (
        <small className="result-error">
          {failing.name ? `${failing.name} · ` : ""}
          {failing.error}
          {/* Offered where the demand was made. The other way out —
              materializing the frame — is on the frame's own card, and the
              message above names it. */}
          {failing.error?.includes("Freeze") && (
            <button className="inline-action" onClick={() => void onFreeze(failing.id)}>
              Freeze this answer
            </button>
          )}
        </small>
      ) : stale ? (
        <small className="result-stale">
          {stale.name} was frozen {takenWhen(stale.frozen!.takenAt)}, and what it reads
          has changed since.
          <button className="inline-action" onClick={() => void onFreeze(stale.id)}>
            Refresh
          </button>
        </small>
      ) : (
        <small>{lineSummary(lines)}</small>
      )}
    </div>
  );
}

function nameKey(value: string): string {
  return value.replace(/[^\p{L}\p{N}]/gu, "").toLocaleLowerCase();
}

/**
 * The document's text, with the one line the cursor is in left as the author
 * has it.
 *
 * They are two different authorities on the same text and they only disagree
 * about one line: the document is deliberately holding that line's old name
 * until the cursor leaves, and handing it back would retype the name out from
 * under whoever is typing it. Everywhere else the document is right, which is
 * how a rename made elsewhere still lands while somebody is typing here.
 */
function gutterRowTitle(
  line: ComputedBlockLine,
  answered: boolean
): string | undefined {
  if (line.error) return line.error;
  if (line.frozen) return `Frozen ${takenWhen(line.frozen.takenAt)}`;
  return answered ? "Open and copy this answer" : undefined;
}

/** One answer beside its line, spanning as many rows as the line does. */
function GutterRow({
  line,
  span,
  banded,
  open,
  onToggle,
}: {
  line: ComputedBlockLine;
  span: number;
  banded: boolean;
  open: boolean;
  onToggle: () => void;
}) {
  const useGrouping = useContext(NumberDisplayContext);
  const answered = !line.blank && !line.comment && !line.error;
  const className = `block-gutter-row${line.error ? " failed" : ""}${
    line.frozen ? (line.frozen.stale ? " stale" : " frozen") : ""
  }${answered && scratchworkResultIsLong(line.display) ? " long" : ""}${
    banded ? " banded" : ""
  }${span > 1 ? " spanning" : ""}`;
  const spanStyle =
    span > 1 ? { height: `calc(var(--text-sm) * 1.7 * ${span})` } : undefined;
  const title = gutterRowTitle(line, answered);
  return answered ? (
    <button
      type="button"
      className={`${className}${open ? " open" : ""}`}
      style={spanStyle}
      title={title}
      aria-label={`Open ${line.name || "scratchwork"} result`}
      aria-expanded={open}
      onClick={onToggle}
    >
      <span>
        {formatComputedScalar(
          line.typedValue,
          line.dataType,
          line.display,
          useGrouping
        )}
        {line.frozen && (
          <i className={line.frozen.stale ? "stale" : "frozen"}>
            {line.frozen.stale ? "*" : "·"}
          </i>
        )}
      </span>
      <ChevronRight size={10} aria-hidden />
    </button>
  ) : (
    <div className={className} style={spanStyle} title={title}>
      {line.error ? "!" : ""}
    </div>
  );
}

/**
 * The block's text with its names marked up — the same courtesy a column
 * header gets in the grid: you can see at a glance that a thing is a real
 * thing and not a word you typed hopefully.
 *
 * A name that resolves is drawn as a reference; one that does not is drawn
 * as unknown rather than left plain, because "I have not finished typing it"
 * and "this does not exist" look identical otherwise.
 */
function highlightSource(
  source: string,
  known: Set<string>,
  references: FormulaReference[]
): ReactNode[] {
  const decorations = formulaReferenceDecorations(source, references);
  const parts: ReactNode[] = [];
  let index = 0;
  for (const decoration of decorations) {
    if (decoration.start > index)
      parts.push(...highlightNames(source.slice(index, decoration.start), known, index));
    parts.push(
      <span
        key={`${decoration.start}-${decoration.reference.id}`}
        className={`formula-reference-text formula-ref-color-${decoration.colorIndex}`}
      >
        {source.slice(decoration.start, decoration.end)}
      </span>
    );
    index = decoration.end;
  }
  parts.push(...highlightNames(source.slice(index), known, index));
  return parts;
}

function highlightNames(
  source: string,
  known: Set<string>,
  offset: number
): ReactNode[] {
  const parts: ReactNode[] = [];
  // Backticked names, bare words, and everything between them.
  const pattern = /`(?:[^`]|``)*`?|[\p{L}_][\p{L}\p{N}_]*/gu;
  let index = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    if (match.index > index) parts.push(source.slice(index, match.index));
    const text = match[0];
    const quoted = text.startsWith("`");
    const inner = quoted ? text.replace(/`/g, "") : text;
    const resolves = known.has(nameKey(inner));
    parts.push(
      quoted || resolves ? (
        <span
          key={`${offset + match.index}-${text}`}
          className={resolves ? "formula-name" : "formula-name unknown"}
        >
          {text}
        </span>
      ) : (
        text
      )
    );
    index = match.index + text.length;
  }
  parts.push(source.slice(index));
  return parts;
}

/** What the card says about itself under the text: quiet, and only useful. */
function lineSummary(lines: ComputedBlockLine[]): string {
  const failed = lines.filter((line) => line.error).length;
  if (failed > 0) return `${failed} line${failed === 1 ? "" : "s"} cannot be worked out yet`;
  const answered = lines.filter((line) => !line.blank && !line.comment);
  if (answered.length === 0) return "type a calculation — name one with ‘rate = 0.08’";
  // Counted rather than assumed: a card that says "live" while one of its
  // answers was written down last Tuesday is lying about the thing it
  // exists to be honest about.
  const frozen = answered.filter((line) => line.frozen).length;
  if (frozen === 0) return `${answered.length} line${answered.length === 1 ? "" : "s"} · live`;
  if (frozen === answered.length) return `${frozen} frozen`;
  return `${answered.length - frozen} live · ${frozen} frozen`;
}
