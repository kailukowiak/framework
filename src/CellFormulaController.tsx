import { useEffect, useRef, useState } from "react";
import {
  useActiveFormulaEditorCommands,
  useFormulaEditorRegistration,
} from "./ActiveFormulaEditor";
import type { FormulaBarCell } from "./lib/formulaBarCell";
import type { FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";

export type CellFormulaRequest = {
  key: string;
  cellId: string;
  /** null edits the saved formula; a string replaces it as typing does. */
  seed: string | null;
};

/**
 * Compatibility editor for a cell formula stored by an older document. Its
 * address is the selected cell and its only editor is the shared formula bar,
 * so inspecting or correcting it cannot create two disagreeing drafts. New
 * cell-level calculations are written to Scratchwork instead.
 */
export function CellFormulaController({
  cell,
  references,
  request,
  onOperation,
  onSaved,
  orderingDeclared,
}: {
  cell: FormulaBarCell;
  references: FormulaReference[];
  request: CellFormulaRequest | null;
  onOperation: OperationHandler;
  onSaved: () => void;
  orderingDeclared: boolean;
}) {
  const editorId = `cell:${cell.frameId}:${cell.rowId}:${cell.columnId}`;
  const saved = cell.kind === "override" ? cell.value : "";
  const [draft, setDraft] = useState(saved);
  const [pending, setPending] = useState<{
    key: string;
    draft: string;
  } | null>(null);
  const commands = useActiveFormulaEditorCommands();

  useEffect(() => setDraft(saved), [cell.id, saved]);
  useEffect(
    () => () => commands.clear(editorId),
    [commands, editorId]
  );

  const registration = useFormulaEditorRegistration({
    id: editorId,
    label: cell.label,
    kind: "formula",
    draft,
    completion: {
      references,
      frameId: cell.frameId,
      targetColumnId: cell.columnId,
      anchorRowIndex: cell.rowIndex,
      anchorFrameId: cell.frameId,
      orderingDeclared,
      appliesToAllRows: false,
    },
    onChange: setDraft,
    onCommit: async (source) => {
      const failure = await onOperation({
        type: "setCellOverride",
        frameId: cell.frameId,
        rowId: cell.rowId,
        columnId: cell.columnId,
        formula: source.trim() || null,
      });
      if (failure) return;
      commands.clear(editorId);
      onSaved();
    },
    onFocus: (selection) => {
      requestAnimationFrame(() => {
        const input = window.document.querySelector<HTMLTextAreaElement>(
          ".scratchwork-formula-bar textarea"
        );
        input?.focus();
        input?.setSelectionRange(selection.start, selection.end);
      });
    },
  });
  const activate = useRef(registration.activateAt);
  activate.current = registration.activateAt;

  useEffect(() => {
    if (!request || request.cellId !== cell.id) return;
    const next = request.seed ?? saved;
    setDraft(next);
    setPending({ key: request.key, draft: next });
  }, [cell.id, request, saved]);

  useEffect(() => {
    if (!pending || pending.draft !== draft) return;
    activate.current({ start: draft.length, end: draft.length });
    setPending(null);
  }, [draft, pending]);

  return null;
}
