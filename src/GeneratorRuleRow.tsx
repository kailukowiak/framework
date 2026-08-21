import { useEffect, useState } from "react";
import type { OperationHandler } from "./lib/handlers";

/**
 * The one editable line a generated frame has: its rule. Lives directly
 * under the title row, the way a chain's steps live in Wrangle — the rows
 * below are this line's output, so the line is the frame's real editing
 * surface. Commits on blur or Enter, like every field here; a rule the
 * core refuses stays in the box with the refusal under it.
 */
export function GeneratorRuleRow({
  frameId,
  rule,
  onOperation,
}: {
  frameId: string;
  rule: string;
  onOperation: OperationHandler;
}) {
  const [draft, setDraft] = useState(rule);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setDraft(rule);
    setError(null);
  }, [rule]);

  const commit = async () => {
    if (draft.trim() === rule.trim()) return;
    try {
      await onOperation({ type: "setFrameGenerator", frameId, formula: draft });
      setError(null);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    }
  };

  return (
    <div className="generator-rule-row">
      <input
        aria-label="Generator rule"
        className="generator-rule-input"
        value={draft}
        spellCheck={false}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => void commit()}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            event.currentTarget.blur();
          } else if (event.key === "Escape") {
            setDraft(rule);
            setError(null);
            event.currentTarget.blur();
          }
        }}
      />
      {error && <div className="generator-rule-error">{error}</div>}
    </div>
  );
}
