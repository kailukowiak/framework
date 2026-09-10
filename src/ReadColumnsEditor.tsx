import { useState } from "react";
import type { OperationHandler } from "./lib/handlers";
import type { FrameObject } from "./lib/types";

/** A pasted header row is one edit, including swaps of two existing names.
 * Keep one surface so twenty columns cost twenty lines, not twenty controls. */
export function ReadColumnsEditor({ frame, onOperation }: {
  frame: FrameObject;
  onOperation: OperationHandler;
}) {
  const initial = frame.columns.map((column) => column.name).join("\n");
  const [draft, setDraft] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const commit = async () => {
    if (draft === initial) return;
    const names = draft.replace(/\r\n/g, "\n").replace(/\n$/, "").split(/[\t\n]/);
    if (names.length !== frame.columns.length) {
      setError(`Expected ${frame.columns.length} names; received ${names.length}.`);
      return;
    }
    if (names.some((name) => !name.trim()) || new Set(names).size !== names.length) {
      setError("Column names must be nonempty and unique.");
      return;
    }
    setError(await onOperation({
      type: "renameColumns", frameId: frame.id,
      names: frame.columns.map((column, index) => [column.id, names[index]]),
    }, { inlineError: true }));
  };
  return <details>
    <summary>Column names</summary>
    <textarea
      aria-label="Column names"
      value={draft}
      rows={Math.min(20, Math.max(2, frame.columns.length))}
      style={{ width: "100%", resize: "vertical", font: "inherit" }}
      onChange={(event) => { setDraft(event.target.value); setError(null); }}
      onBlur={() => void commit()}
    />
    {error && <p role="alert">{error}</p>}
  </details>;
}
