import type { FormulaCompletionState } from "../FormulaCompletion";
import type { ActiveFormulaEditor } from "./activeFormulaEditor";
import type { FormulaBarCell } from "./formulaBarCell";
import type { FormulaReference } from "./formulaReferences";
import { scratchworkLineAt } from "./scratchwork";

export type FormulaHelpControl = {
  open: boolean;
  query: string;
  onToggle: (query: string) => void;
};

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

export function formulaHelpControl(
  formulaMode: boolean,
  activeLabel: string | undefined,
  completion: FormulaCompletionState,
  open: boolean,
  onToggle: ((query: string) => void) | undefined
): FormulaHelpControl | undefined {
  if (!formulaMode || !onToggle) return undefined;
  const query =
    completion.parameterHelp?.label ??
    completion.suggestionHelp?.label ??
    (activeLabel === "Columns" ? "choose several columns" : activeLabel) ??
    (completion.query.length >= 3 ? completion.query : "formula");
  return { open, query, onToggle };
}
