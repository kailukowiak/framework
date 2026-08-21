import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ActiveFormulaAnswer } from "./ActiveFormulaAnswer";
import { useActiveFormulaEditor } from "./ActiveFormulaEditor";
import { FormulaBarActions } from "./FormulaBarActions";
import { HighlightedFormulaTextarea } from "./FormulaReferenceText";
import {
  FormulaCompletionMenu,
  useFormulaCompletion,
} from "./FormulaCompletion";
import { continueFormula } from "./lib/blockLines";
import type { FormulaBarCell } from "./lib/formulaBarCell";
import { formatFormulaChains } from "./lib/formulaFormatting";
import type { FormulaReference } from "./lib/formulaReferences";
import { replaceScratchworkLine, scratchworkLineAt } from "./lib/scratchwork";
import { useMeasuredFormulaBar } from "./useMeasuredFormulaBar";

export type ScratchworkFormulaFeedback = {
  saved: boolean;
  name?: string;
  display?: string;
  error?: string;
};

function documentSelection(lineStart: number, start: number, end: number) {
  return { start: lineStart + start, end: lineStart + end };
}

function canFormatFormula(formulaMode: boolean, draft: string, busy: boolean) {
  return formulaMode && Boolean(draft.trim()) && !busy;
}

async function formatDraft({
  source,
  cursor,
  input,
  change,
  setFreshCursor,
  persist,
  setBusy,
}: {
  source: string;
  cursor: number;
  input: HTMLTextAreaElement | null;
  change: (value: string, start: number, end: number) => void;
  setFreshCursor: (cursor: number) => void;
  persist: (() => Promise<void>) | null;
  setBusy: (busy: boolean) => void;
}) {
  const formatted = formatFormulaChains(
    source,
    input?.selectionStart ?? cursor,
    input?.selectionEnd ?? cursor
  );
  if (formatted.source !== source) {
    change(formatted.source, formatted.selection.start, formatted.selection.end);
    setFreshCursor(formatted.selection.end);
    window.requestAnimationFrame(() => {
      input?.focus();
      input?.setSelectionRange(formatted.selection.start, formatted.selection.end);
    });
  }
  if (!persist) return;
  setBusy(true);
  try {
    await persist();
  } finally {
    setBusy(false);
  }
}

export function ScratchworkFormulaBar({
  onCommit,
  references,
  cell,
  onCommitCell,
  onEditCalculatedCell,
  onEditOverrideCell,
  onRequestReadOnlyCell,
  expanded,
  onToggle,
}: {
  onCommit: (formula: string) => Promise<ScratchworkFormulaFeedback>;
  references: FormulaReference[];
  cell: FormulaBarCell | null;
  onCommitCell: (cell: FormulaBarCell, raw: string) => Promise<string | null>;
  onEditCalculatedCell: (cell: FormulaBarCell) => void;
  onEditOverrideCell: (cell: FormulaBarCell) => void;
  onRequestReadOnlyCell: (cell: FormulaBarCell) => void;
  expanded: boolean;
  onToggle: () => void;
}) {
  const {
    active,
    setDraft: setActiveDraft,
    setSelection: setActiveSelection,
    engage: engageActiveEditor,
    disengage: disengageActiveEditor,
    clear: clearActiveEditor,
    commit: commitActiveEditor,
  } = useActiveFormulaEditor();
  const [freshDraft, setFreshDraft] = useState("");
  const [cellDraft, setCellDraft] = useState("");
  const [cellDirty, setCellDirty] = useState(false);
  const [cellError, setCellError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<ScratchworkFormulaFeedback | null>(
    null
  );
  const [busy, setBusy] = useState(false);
  const [focused, setFocused] = useState(false);
  const [freshCursor, setFreshCursor] = useState(0);
  const request = useRef(0);
  const bar = useMeasuredFormulaBar();
  const input = useRef<HTMLTextAreaElement>(null);

  const activeLine =
    active?.kind === "scratchwork"
      ? scratchworkLineAt(active.draft, active.selection.end)
      : null;
  const selectedCell = active ? null : cell;
  const draft =
    activeLine?.source ?? active?.draft ?? (selectedCell ? cellDraft : freshDraft);
  const cursor = active
    ? active.selection.end - (activeLine?.start ?? 0)
    : selectedCell
    ? cellDraft.length
    : freshCursor;
  const formulaMode = Boolean(active || !selectedCell);
  const formulaReferences = active?.completion.references ?? references;

  useEffect(() => setFeedback(null), [active?.id]);
  useEffect(() => {
    setCellDraft(cell?.value ?? "");
    setCellDirty(false);
    setCellError(null);
  }, [cell?.id, cell?.value]);

  const change = (
    value: string,
    selectionStart: number,
    selectionEnd: number
  ) => {
    setFeedback(null);
    if (!active && selectedCell) {
      if (selectedCell.kind !== "literal") return;
      setCellDraft(value);
      setCellDirty(true);
      setCellError(null);
      return;
    }
    if (!active) {
      setFreshDraft(value);
      return;
    }
    const selection = documentSelection(activeLine?.start ?? 0, selectionStart, selectionEnd);
    if (!activeLine) {
      setActiveDraft(value, selection);
      return;
    }
    const replaced = replaceScratchworkLine(
      active.draft,
      active.selection.end,
      value,
      { start: selectionStart, end: selectionEnd }
    );
    setActiveDraft(replaced.source, replaced.selection);
  };

  const completion = useFormulaCompletion({
    source: draft,
    cursor,
    enabled: focused && formulaMode,
    context: active?.completion ?? { references },
    onInsert: (updated, nextCursor) => {
      change(updated, nextCursor, nextCursor);
      setFreshCursor(nextCursor);
      window.requestAnimationFrame(() => {
        input.current?.focus();
        input.current?.setSelectionRange(nextCursor, nextCursor);
      });
    },
  });

  // A column picked from the canvas updates the logical cursor without
  // moving DOM focus away from this bar. Carry that cursor into the input
  // after the shared draft has rendered.
  useLayoutEffect(() => {
    const node = input.current;
    if (!node || !focused || !active || window.document.activeElement !== node) return;
    const offset = activeLine?.start ?? 0;
    node.setSelectionRange(
      active.selection.start - offset,
      active.selection.end - offset
    );
  }, [active, activeLine?.start, focused]);

  const commit = async () => {
    if (busy) return;
    if (active) {
      setBusy(true);
      try {
        await commitActiveEditor();
      } finally {
        setBusy(false);
      }
      return;
    }
    if (selectedCell) {
      if (selectedCell.kind === "override") {
        onEditOverrideCell(selectedCell);
        return;
      }
      if (selectedCell.kind === "calculated") {
        onEditCalculatedCell(selectedCell);
        return;
      }
      if (selectedCell.kind !== "literal" || !cellDirty) return;
      setBusy(true);
      try {
        const error = await onCommitCell(selectedCell, cellDraft);
        setCellError(error);
        if (!error) setCellDirty(false);
      } finally {
        setBusy(false);
      }
      return;
    }
    const formula = freshDraft.trim();
    if (!formula) return;
    const id = ++request.current;
    setBusy(true);
    let next: ScratchworkFormulaFeedback;
    try {
      next = await onCommit(formula);
    } catch (reason) {
      next = {
        saved: false,
        error: String(reason).replace(/^Error:\s*/, ""),
      };
    }
    if (request.current !== id) return;
    setBusy(false);
    setFeedback(next);
    if (next.saved) setFreshDraft("");
  };

  return (
    <form
      ref={bar}
      className="scratchwork-formula-bar"
      onSubmit={(event) => {
        event.preventDefault();
        void commit();
      }}
    >
      <span className="scratchwork-formula-prefix" aria-hidden>
        {selectedCell?.address ?? "="}
      </span>
      <HighlightedFormulaTextarea
        ref={input}
        rows={draft.split("\n").length}
        references={formulaMode ? formulaReferences : []}
        aria-label={
          active
            ? `Edit ${active.label}`
            : selectedCell
            ? `${selectedCell.kind === "literal" ? "Edit" : "View"} ${selectedCell.label}`
            : "Add formula to Scratchwork"
        }
        value={draft}
        placeholder={
          selectedCell?.kind === "literal"
            ? "Type a value"
            : selectedCell
            ? undefined
            : "4100 * 1.2  or  margin = revenue - cost"
        }
        spellCheck={false}
        readOnly={busy || Boolean(selectedCell && selectedCell.kind !== "literal")}
        onFocus={(event) => {
          setFocused(true);
          if (selectedCell?.kind === "calculated") {
            onEditCalculatedCell(selectedCell);
            return;
          }
          if (selectedCell?.kind === "override") {
            onEditOverrideCell(selectedCell);
            return;
          }
          if (selectedCell?.kind === "readOnly") {
            onRequestReadOnlyCell(selectedCell);
            return;
          }
          engageActiveEditor();
          setFreshCursor(event.currentTarget.selectionStart ?? draft.length);
        }}
        onBlur={() => {
          setFocused(false);
          if (active) disengageActiveEditor();
          else if (selectedCell?.kind === "literal" && cellDirty) void commit();
        }}
        onChange={(event) => {
          change(
            event.target.value,
            event.target.selectionStart ?? event.target.value.length,
            event.target.selectionEnd ?? event.target.value.length
          );
          setFreshCursor(
            event.target.selectionStart ?? event.target.value.length
          );
        }}
        onSelect={(event) => {
          const start = event.currentTarget.selectionStart ?? 0;
          const end = event.currentTarget.selectionEnd ?? 0;
          setFreshCursor(end);
          if (active)
            setActiveSelection(documentSelection(activeLine?.start ?? 0, start, end));
        }}
        onKeyDown={(event) => {
          // Alt+Return expands the formula inside an explicit boundary.
          if (formulaMode && event.key === "Enter" && event.altKey) {
            event.preventDefault();
            const node = event.currentTarget;
            const at = node.selectionStart ?? draft.length;
            const expanded = continueFormula(draft, at, node.selectionEnd ?? at);
            change(expanded.source, expanded.selection.start, expanded.selection.end);
            setFreshCursor(expanded.selection.end);
            window.requestAnimationFrame(() => {
              input.current?.focus();
              input.current?.setSelectionRange(expanded.selection.start, expanded.selection.end);
            });
            return;
          }
          if (event.key === "ArrowDown" && completion.suggestionCount) {
            event.preventDefault();
            completion.setActiveIndex(
              (completion.activeIndex + 1) % completion.suggestionCount
            );
            return;
          }
          if (event.key === "ArrowUp" && completion.suggestionCount) {
            event.preventDefault();
            completion.setActiveIndex(
              (completion.activeIndex - 1 + completion.suggestionCount) %
                completion.suggestionCount
            );
            return;
          }
          if (
            completion.suggestionCount &&
            (event.key === "Tab" ||
              (event.key === "Enter" && completion.query.length > 0))
          ) {
            event.preventDefault();
            completion.insertActive();
            return;
          }
          // A textarea would take Enter as a newline; here it stays the
          // commit it has always been. New rows are Alt+Return's job.
          if (event.key === "Enter") {
            event.preventDefault();
            void commit();
            return;
          }
          if (event.key !== "Escape") return;
          if (completion.offersSuggestions && completion.suggestionCount) {
            event.preventDefault();
            event.stopPropagation();
            completion.dismissSuggestions();
            return;
          }
          if (active) {
            clearActiveEditor();
            event.currentTarget.blur();
            return;
          }
          if (selectedCell) {
            setCellDraft(selectedCell.value);
            setCellDirty(false);
            setCellError(null);
            event.currentTarget.blur();
            return;
          }
          setFreshDraft("");
          setFeedback(null);
          event.currentTarget.blur();
        }}
      />
      {busy ? (
        <span className="scratchwork-formula-answer">…</span>
      ) : formulaMode && completion.parameterHelp?.signature ? (
        <span className="scratchwork-formula-parameter">
          <code>{completion.parameterHelp.signature}</code>
          {completion.activeParameter && ` · ${completion.activeParameter}`}
          {completion.activeParameterHelp && (
            <>
              {` — ${completion.activeParameterHelp.description}`}
              {completion.activeParameterHelp.example &&
                ` Try ${completion.activeParameterHelp.example}.`}
            </>
          )}
        </span>
      ) : active ? (
        <ActiveFormulaAnswer
          active={active}
          draft={draft}
          references={formulaReferences}
          onSelect={(start, end) => {
            const offset = activeLine?.start ?? 0;
            setActiveSelection({ start: offset + start, end: offset + end });
            window.requestAnimationFrame(() => {
              input.current?.focus();
              input.current?.setSelectionRange(start, end);
            });
          }}
        />
      ) : selectedCell ? (
        <span
          className={`scratchwork-formula-answer${
            cellError || selectedCell.kind === "readOnly" ? " invalid" : ""
          }`}
          title={cellError ?? selectedCell.reason}
        >
          <strong>{selectedCell.label}</strong>
          {cellError
            ? ` · ${cellError}`
            : selectedCell.kind === "calculated"
            ? " · calculated column · click to edit in Wrangle"
            : selectedCell.kind === "override"
            ? " · legacy cell formula · click to inspect"
            : selectedCell.kind === "readOnly"
            ? ` · ${selectedCell.reason}`
            : cellDirty
            ? " · Enter saves"
            : " · literal value"}
        </span>
      ) : feedback ? (
        <output
          className={`scratchwork-formula-answer${feedback.error ? " invalid" : ""}`}
          title={feedback.error}
        >
          {feedback.name && <strong>{feedback.name}</strong>}
          {feedback.name && (feedback.display || feedback.error) ? " = " : null}
          {feedback.error ?? feedback.display}
        </output>
      ) : (
        <span className="scratchwork-formula-key">Enter</span>
      )}
      {focused && formulaMode && <FormulaCompletionMenu completion={completion} anchorRef={input} />}
      {/* Pointer-down stays out of both actions: blur is a save and cannot
          race either a draft rewrite or the Scratchwork editor handoff. */}
      <FormulaBarActions
        canFormat={canFormatFormula(formulaMode, draft, busy)}
        expanded={expanded}
        onFormat={() => void formatDraft({ source: draft, cursor, input: input.current, change, setFreshCursor, persist: active ? commitActiveEditor : null, setBusy })}
        onToggle={onToggle}
      />
    </form>
  );
}
