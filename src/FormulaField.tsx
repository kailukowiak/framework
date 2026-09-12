import { useEffect, useRef, useState } from "react";
import { useActiveFormulaEditorCommands } from "./ActiveFormulaEditor";
import { FormulaEditor } from "./FormulaEditor";
import type { FormulaReference } from "./lib/formulaReferences";

export function FormulaField({
  editorId,
  label,
  initial,
  help,
  references,
  frameId,
  focusToken,
  compact = false,
  commitOnBlur,
  onCommit,
}: {
  editorId: string;
  label: string;
  initial: string;
  help?: string;
  references: FormulaReference[];
  frameId?: string;
  focusToken?: number;
  compact?: boolean;
  /** Self-saving surfaces persist on blur; see FormulaEditor's prop. */
  commitOnBlur?: boolean;
  onCommit: (value: string) => Promise<string | null>;
}) {
  const [value, setValue] = useState(initial);
  const [formulaError, setFormulaError] = useState<string | null>(null);
  const previousInitial = useRef(initial);
  const { getActive, reconcile } = useActiveFormulaEditorCommands();
  useEffect(() => {
    // Mounting must retain an editor session's unsaved draft. A changed
    // stored formula, however, must reach both the field and the formula
    // bar, or rebinding would restore the pre-control value over this one.
    if (previousInitial.current === initial) return;
    previousInitial.current = initial;
    const active = getActive();
    const selection = active?.id === editorId ? active.selection : { start: 0, end: 0 };
    reconcile(editorId, initial, selection);
    setValue(initial);
  }, [initial, editorId, getActive, reconcile]);
  const execute = async (draft = value) => {
    if (draft.trim() === initial.trim()) {
      setFormulaError(null);
      return;
    }
    setFormulaError(await onCommit(draft));
  };
  return (
    <div className="inspector-field formula-field">
      <FormulaEditor
        editorId={editorId}
        label={label}
        value={value}
        references={references}
        frameId={frameId}
        focusToken={focusToken}
        error={formulaError}
        compact={compact}
        commitOnBlur={commitOnBlur}
        onChange={(next) => {
          setValue(next);
          setFormulaError(null);
        }}
        {...(compact
          ? { onCommit: execute }
          : { onExecute: execute, executeLabel: "Execute" })}
      />
      {help && <small>{help}</small>}
    </div>
  );
}
