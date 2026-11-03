import type { FormulaCompletionState } from "../FormulaCompletion";
import type { ActiveFormulaEditor } from "./activeFormulaEditor";
import type { FormulaBarCell } from "./formulaBarCell";
import { useEffect } from "react";
import { useHelpPanelContext, type HelpTarget } from "./helpPanel";
import type { FormulaReference } from "./formulaReferences";
import { scratchworkLineAt } from "./scratchwork";

export function formulaRows(draft: string, expanded: boolean) {
  return expanded ? Math.min(5, Math.max(2, draft.split("\n").length)) : 1;
}

export function canFormatFormula(formulaMode: boolean, draft: string, busy: boolean) {
  return formulaMode && Boolean(draft.trim()) && !busy;
}

/** A session is a visible mode because canvas clicks insert into its formula. */
export function barModeClass(active: ActiveFormulaEditor | null, expanded: boolean): string {
  return `scratchwork-formula-bar${active ? " session" : ""}${expanded ? " formula-expanded" : ""}`;
}

export function formulaBarEditingState(
  active: ActiveFormulaEditor | null,
  cell: FormulaBarCell | null,
  cellDraft: string,
  freshDraft: string,
  freshCursor: number,
  references: FormulaReference[]
) {
  const activeLine = active?.kind === "scratchwork"
    ? scratchworkLineAt(active.draft, active.selection.end)
    : null;
  const selectedCell = active ? null : cell;
  return {
    activeLine,
    selectedCell,
    draft: activeLine?.source ?? active?.draft ?? (selectedCell ? cellDraft : freshDraft),
    cursor: active
      ? active.selection.end - (activeLine?.start ?? 0)
      : selectedCell ? cellDraft.length : freshCursor,
    formulaMode: Boolean(active || !selectedCell),
    formulaReferences: active?.completion.references ?? references,
  };
}

/**
 * The function whose call the cursor is inside, by reading the text: the
 * innermost `(` still open at the cursor, outside quotes and backtick names,
 * and the name written right before it.
 *
 * The engine answers the same question exactly, but only for the editor that
 * is asking it — the bar while the bar has focus. A Scratchwork line is typed
 * in its own field, so for ⌘K to open the function there the bar reads the
 * text itself rather than send a second completion request per keystroke.
 */
export function functionAtCursor(
  source: string,
  cursor: number,
  references: FormulaReference[]
): FormulaReference | undefined {
  const open: number[] = [];
  let quote: string | null = null;
  for (let index = 0; index < cursor && index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === "\\" && quote !== "`") index += 1;
      else if (character === quote) quote = null;
    } else if (character === '"' || character === "'" || character === "`") quote = character;
    else if (character === "(") open.push(index);
    else if (character === ")") open.pop();
  }
  const call = open.at(-1);
  if (call === undefined) return undefined;
  const name = /(\.?[\p{L}_][\p{L}\p{N}_]*)\s*$/u.exec(source.slice(0, call))?.[1];
  return name
    ? references.find((reference) => reference.kind === "function" && reference.label === name)
    : undefined;
}

/**
 * The help entry for wherever the formula cursor is: the call it is inside,
 * else the suggestion it is choosing, else the transformation being edited.
 * A function reference carries its catalog id, so help can open on that exact
 * entry; the label is searched as well, which keeps neighbouring answers
 * listed beside it.
 */
export function formulaHelpTarget(
  formulaMode: boolean,
  activeLabel: string | undefined,
  completion: FormulaCompletionState,
  text: { draft: string; cursor: number; references: FormulaReference[] }
): HelpTarget | undefined {
  if (!formulaMode) return undefined;
  const reference =
    completion.parameterHelp ??
    completion.suggestionHelp ??
    functionAtCursor(text.draft, text.cursor, text.references);
  if (reference) return { query: reference.label, entryId: reference.id };
  if (activeLabel === "Columns") return { query: "choose several columns" };
  if (activeLabel) return { query: activeLabel };
  return { query: completion.query.length >= 3 ? completion.query : "formula" };
}

/**
 * The bar's half of contextual help. It keeps the panel told what the cursor
 * is on — ⌘K arrives through the native menu and reads it from there — and
 * gives the bar's Help button a toggle that opens the same entry without
 * taking the formula's focus.
 */
export function useFormulaBarHelp(
  target: HelpTarget | undefined
): { open: boolean; onToggle: () => void } | undefined {
  const help = useHelpPanelContext();
  useEffect(() => {
    if (help) help.formulaTarget.current = target;
  });
  if (!help || !target) return undefined;
  return { open: Boolean(help.request), onToggle: () => help.toggle({ ...target, keepFocus: true }) };
}
