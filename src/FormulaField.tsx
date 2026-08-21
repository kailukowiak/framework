import { useState } from "react";
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
  onCommit,
}: {
  editorId: string;
  label: string;
  initial: string;
  help?: string;
  references: FormulaReference[];
  frameId?: string;
  focusToken?: number;
  onCommit: (value: string) => Promise<string | null>;
}) {
  const [value, setValue] = useState(initial);
  const [formulaError, setFormulaError] = useState<string | null>(null);
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
        onChange={(next) => {
          setValue(next);
          setFormulaError(null);
        }}
        onExecute={execute}
        executeLabel="Execute"
      />
      {help && <small>{help}</small>}
    </div>
  );
}


