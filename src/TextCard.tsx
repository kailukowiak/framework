import { useRef, useState } from "react";
import {
  FormulaCompletionMenu,
  useFormulaCompletion,
} from "./FormulaCompletion";
import type { ComputedText } from "./lib/bindings/ComputedText";
import type { FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type { TextObject } from "./lib/types";
import { TextMarkdown } from "./Markdown";
import { DebugTracePanel } from "./DebugTracePanel";

type FormulaHole = {
  source: string;
  cursor: number;
  contentStart: number;
  contentEnd: number;
};

/**
 * The formula hole containing the caret, expressed in the coordinates the
 * shared formula completion engine expects.
 *
 * Markdown remains an ordinary textarea rather than becoming a second
 * formula language. Only the characters between the nearest unmatched `{{`
 * and its `}}` are handed to completion; insertion is translated back into
 * the full prose source afterwards. An unfinished hole is included because
 * completion is most useful while the closing braces have not been typed yet.
 */
export function formulaHoleAt(source: string, cursor: number): FormulaHole | null {
  const opening = source.lastIndexOf("{{", cursor);
  if (opening < 0) return null;
  const contentStart = opening + 2;
  const closedBeforeCaret = source.lastIndexOf("}}", cursor - 1);
  if (closedBeforeCaret >= contentStart) return null;
  const closing = source.indexOf("}}", contentStart);
  if (closing >= 0 && cursor > closing) return null;
  const contentEnd = closing >= 0 ? closing : source.length;
  return {
    source: source.slice(contentStart, contentEnd),
    cursor: Math.max(0, cursor - contentStart),
    contentStart,
    contentEnd,
  };
}

/**
 * A card of prose. Rendered markdown until clicked; one plain textarea
 * while being written; blur commits. The `{{…}}` holes print live answers
 * — the computed source the editor opens with is reconstructed from the
 * stored segments, so a rename elsewhere in the document has already been
 * written into what this card offers for editing.
 */
export function TextCard({
  text,
  computed,
  references,
  onOperation,
}: {
  text: TextObject;
  computed: ComputedText | undefined;
  references: FormulaReference[];
  onOperation: OperationHandler;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [cursor, setCursor] = useState(0);
  const editor = useRef<HTMLTextAreaElement>(null);
  const source = computed?.source ?? text.text;
  const hasError = computed?.segments.some(
    (segment) => segment.kind === "broken" || (segment.kind === "value" && segment.error)
  );
  const hole = editing ? formulaHoleAt(draft, cursor) : null;
  const completion = useFormulaCompletion({
    source: hole?.source ?? "",
    cursor: hole?.cursor ?? 0,
    enabled: hole !== null,
    context: { references },
    onInsert: (formula, formulaCursor) => {
      if (!hole) return;
      const updated = `${draft.slice(0, hole.contentStart)}${formula}${draft.slice(
        hole.contentEnd
      )}`;
      const nextCursor = hole.contentStart + formulaCursor;
      setDraft(updated);
      setCursor(nextCursor);
      requestAnimationFrame(() => {
        editor.current?.focus();
        editor.current?.setSelectionRange(nextCursor, nextCursor);
      });
    },
  });
  if (!editing)
    return (
      <>
        <button
          type="button"
          className="text-card-body"
          title="Edit text"
          onClick={() => {
            setDraft(source);
            setCursor(source.length);
            setEditing(true);
          }}
        >
          {computed && computed.segments.length ? (
            <TextMarkdown segments={computed.segments} />
          ) : (
            <span className="text-card-empty">
              {"Write here — markdown, with {{formula}} holes for live values"}
            </span>
          )}
        </button>
        {hasError && <DebugTracePanel objectId={text.id} />}
      </>
    );
  const { activeIndex, setActiveIndex, query, suggestionCount } = completion;
  return (
    <div className="text-card-editing">
      <textarea
        ref={editor}
        className="text-card-editor"
        aria-label={`${text.name} markdown`}
        autoFocus
        value={draft}
        onFocus={(event) => {
          const end = event.currentTarget.value.length;
          event.currentTarget.setSelectionRange(end, end);
          setCursor(end);
        }}
        onChange={(event) => {
          setDraft(event.target.value);
          setCursor(event.target.selectionStart);
        }}
        onSelect={(event) => setCursor(event.currentTarget.selectionStart)}
        onBlur={(event) => {
          // Choosing a completion holds the textarea focus through mouse down.
          // A real departure commits the whole markdown document as before.
          if (event.currentTarget.parentElement?.contains(event.relatedTarget)) return;
          setEditing(false);
          if (draft !== source)
            void onOperation({
              type: "setTextSource",
              objectId: text.id,
              source: draft,
            });
        }}
        onKeyDown={(event) => {
          if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
            event.currentTarget.blur();
            return;
          }
          if (!suggestionCount) return;
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setActiveIndex((activeIndex + 1) % suggestionCount);
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setActiveIndex((activeIndex - 1 + suggestionCount) % suggestionCount);
          } else if (event.key === "Tab" || (event.key === "Enter" && query.length)) {
            event.preventDefault();
            completion.insertActive();
          }
        }}
      />
      <FormulaCompletionMenu completion={completion} anchorRef={editor} />
    </div>
  );
}
