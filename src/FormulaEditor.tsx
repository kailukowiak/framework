import { Play } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { useFormulaEditorRegistration } from "./ActiveFormulaEditor";
import { HighlightedFormulaTextarea } from "./FormulaReferenceText";
import {
  FormulaCompletionMenu,
  useFormulaCompletion,
} from "./FormulaCompletion";
import {
  isFormulaExecuteShortcut,
  type FormulaReference,
} from "./lib/formulaReferences";
import { writeClipboardText } from "./lib/clipboard";
import type { FrameStepInput } from "./lib/types";

export function FormulaEditor({
  label,
  value,
  references,
  frameId,
  scope,
  editorId,
  focusToken,
  error,
  compact = false,
  executeLabel,
  placeholder,
  onChange,
  onCommit,
  onExecute,
}: {
  label: string;
  value: string;
  references: FormulaReference[];
  frameId?: string;
  /** Example text shown while empty — the cheapest way to teach a syntax. */
  placeholder?: string;
  /**
   * Where in a chain this formula sits, when it sits in one. A step reads
   * what the steps before it leave behind, so completion has to be asked
   * about that position rather than about the frame.
   */
  scope?: { steps: FrameStepInput[]; stepIndex: number };
  /** Stable semantic identity when the surrounding object already has one. */
  editorId?: string;
  /**
   * A counter that takes the cursor when it changes — how a keyboard command
   * elsewhere asks this editor for focus. Set rather than incremented would
   * only ever work once, and asking twice is the ordinary case.
   */
  focusToken?: number;
  error?: string | null;
  /** Suppress repeated keyboard prose in dense, repeated formula rows. */
  compact?: boolean;
  executeLabel?: string;
  onChange: (value: string) => void;
  /** Commit without adding another control; used by shared editing surfaces. */
  onCommit?: (value: string) => void | Promise<void>;
  onExecute?: (value: string) => void | Promise<void>;
}) {
  const textarea = useRef<HTMLTextAreaElement>(null);
  const generatedId = useId();
  const resolvedEditorId = editorId ?? `formula:${generatedId}`;
  const [focused, setFocused] = useState(false);
  const [cursor, setCursor] = useState(value.length);
  const commit = onCommit ?? onExecute;
  const registration = useFormulaEditorRegistration({
    id: resolvedEditorId,
    label,
    kind: "formula",
    draft: value,
    completion: { references, frameId, scope },
    onChange: (draft, selection) => {
      onChange(draft);
      setCursor(selection.end);
    },
    onSelection: (selection) => setCursor(selection.end),
    onCommit: commit,
    onFocus: (selection) => {
      requestAnimationFrame(() => {
        textarea.current?.focus();
        textarea.current?.setSelectionRange(selection.start, selection.end);
      });
    },
  });
  // The text is selected, not just focused: the line being asked for holds a
  // placeholder `0`, and typing over it is the whole gesture.
  useEffect(() => {
    if (focusToken === undefined) return;
    const node = textarea.current;
    if (!node) return;
    node.focus();
    node.select();
  }, [focusToken]);
  const completion = useFormulaCompletion({
    source: value,
    cursor,
    enabled: focused,
    context: { references, frameId, scope },
    onInsert: (updated, nextCursor) => {
      onChange(updated);
      setCursor(nextCursor);
      registration.update(updated, { start: nextCursor, end: nextCursor });
      requestAnimationFrame(() => {
        textarea.current?.focus();
        textarea.current?.setSelectionRange(nextCursor, nextCursor);
      });
    },
  });
  const {
    activeIndex,
    setActiveIndex,
    query,
    suggestionCount,
    parameterHelp,
    activeParameter,
    activeParameterHelp,
    offersSuggestions,
    dismissSuggestions,
    insertActive,
  } = completion;

  return (
    <label className="formula-editor">
      <span className="formula-editor-label">{label}</span>
      <div
        className={`formula-input ${focused ? "focused" : ""} ${
          error ? "invalid" : ""
        }`}
      >
        <span>=</span>
        <HighlightedFormulaTextarea
          ref={textarea}
          value={value}
          references={references}
          aria-invalid={Boolean(error)}
          placeholder={placeholder}
          spellCheck={false}
          onFocus={(event) => {
            setFocused(true);
            setCursor(event.currentTarget.selectionStart);
            registration.activate(event.currentTarget);
          }}
          onSelect={(event) => {
            setCursor(event.currentTarget.selectionStart);
            registration.select(event.currentTarget);
          }}
          onChange={(event) => {
            onChange(event.target.value);
            setCursor(event.target.selectionStart);
            registration.change(event.target.value, event.currentTarget);
          }}
          onBlur={() => {
            setFocused(false);
            registration.blur();
          }}
          onKeyDown={(event) => {
            if (isFormulaExecuteShortcut(event)) {
              event.preventDefault();
              void registration.commit();
              return;
            }
            if (event.key === "Escape") {
              if (offersSuggestions && suggestionCount) {
                event.preventDefault();
                event.stopPropagation();
                dismissSuggestions();
                return;
              }
              setFocused(false);
              event.currentTarget.blur();
              return;
            }
            if (!suggestionCount) return;
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActiveIndex((index) => (index + 1) % suggestionCount);
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActiveIndex(
                (index) => (index - 1 + suggestionCount) % suggestionCount
              );
            } else if (
              event.key === "Tab" ||
              (event.key === "Enter" && query.length > 0)
            ) {
              event.preventDefault();
              insertActive(activeIndex);
            }
          }}
        />
      </div>
      {(!compact || parameterHelp?.signature || executeLabel) && (
        <div className="formula-editor-help">
          {(!compact || parameterHelp?.signature) && (
            <span>
              {parameterHelp?.signature ? (
                <>
                  <code>{parameterHelp.signature}</code>
                  {activeParameter && <>&nbsp;· {activeParameter}</>}
                  {activeParameterHelp && (
                    <>
                      {" "}— {activeParameterHelp.description}
                      {activeParameterHelp.example && (
                        <>
                          {" "}Try <code>{activeParameterHelp.example}</code>.
                        </>
                      )}
                    </>
                  )}
                </>
              ) : (
                "Reference data or insert a function"
              )}
            </span>
          )}
          {!compact && (
            <>
              <kbd>↑↓</kbd>
              <kbd>Tab</kbd>
              {commit && <kbd>⌘↵ run</kbd>}
            </>
          )}
          {executeLabel && (
            <button
              type="button"
              className="formula-execute-button"
              onClick={() => void onExecute?.(textarea.current?.value ?? value)}
            >
              <Play size={10} />
              {executeLabel}
            </button>
          )}
        </div>
      )}
      {error && <FormulaErrorDetails error={error} formulas={[value]} />}
      {focused && <FormulaCompletionMenu completion={completion} anchorRef={textarea} />}
    </label>
  );
}

export function FormulaErrorDetails({
  error,
  formulas = [],
  title = "Formula could not run",
}: {
  error: string;
  formulas?: string[];
  title?: string;
}) {
  const [copied, setCopied] = useState(false);
  const relevantFormulas = formulas.map((formula) => formula.trim()).filter(Boolean);
  const details = [
    ...relevantFormulas.map(
      (formula, index) =>
        `${
          relevantFormulas.length > 1 ? `Formula ${index + 1}` : "Formula"
        }:\n=${formula}`
    ),
    `Error:\n${error}`,
  ].join("\n\n");
  const [copyFailed, setCopyFailed] = useState(false);
  const copyDetails = async () => {
    const written = await writeClipboardText(details);
    setCopied(written);
    setCopyFailed(!written);
    if (written) window.setTimeout(() => setCopied(false), 1800);
  };

  // The sentence, and a way to take it with you. What was here before — a
  // bordered panel with an icon, a heading restating that something failed,
  // the formula echoed back at somebody looking straight at it, and a
  // button — spent a card's worth of space to say one line. The title is
  // kept for the screen reader, which does need telling.
  return (
    <small className="formula-error-line" role="alert">
      <span className="visually-hidden">{title}: </span>
      {error}
      <button type="button" className="inline-action" onClick={() => void copyDetails()}>
        {copied ? "copied" : copyFailed ? "select it by hand" : "copy"}
      </button>
    </small>
  );
}
