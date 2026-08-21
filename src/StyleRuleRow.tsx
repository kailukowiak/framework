import { useEffect, useState } from "react";
import { Minus, Plus, X } from "lucide-react";
import { FormulaEditor } from "./FormulaEditor";
import { frameCellStyleProperties } from "./FrameGrid";
import {
  ruleStops,
  ruleWithScaleMid,
  ruleWithoutStop,
  sameStop,
  stopLabel,
  stopStyle,
  type RuleStop,
} from "./lib/conditionalFormatting";
import type { FormulaReference } from "./lib/formulaReferences";
import type { Column, FrameStyleRule } from "./lib/types";

/** Which stop of which rule the format controls above are pointed at. */
export type RuleTarget = { ruleId: string; stop: RuleStop };

/**
 * One rule: what it reads, where it may paint, and its stops.
 *
 * The rule holds three things nothing else in the panel can say — the
 * formula behind it, the field it is allowed to paint, and which of its
 * stops is being dressed — so those are the only three things it draws. The
 * dressing itself is the Format controls above, aimed at the selected stop,
 * because bold is bold whether it lands on a cell or on a rule.
 */
export function StyleRuleRow({
  rule,
  columns,
  formula,
  frameId,
  references,
  error,
  target,
  onTarget,
  onFormula,
  onRule,
  onDelete,
  onDraftCleared,
}: {
  rule: FrameStyleRule;
  columns: Column[];
  formula: string;
  frameId: string;
  references: FormulaReference[];
  error?: string;
  target: RuleTarget | null;
  onTarget: (target: RuleTarget | null) => void;
  onFormula: (value: string) => void | Promise<void>;
  onRule: (rule: FrameStyleRule) => Promise<string | null>;
  onDelete: () => void;
  onDraftCleared: () => void;
}) {
  // The formula editor is controlled, so the row holds the draft: without
  // one, every keystroke is handed back the committed text and typing does
  // nothing. Re-seeded when the core answers with a different formula --
  // which is what a commit, an undo, or another writer looks like from here.
  const [draft, setDraft] = useState(formula);
  useEffect(() => setDraft(formula), [formula]);
  return (
    <div className={`style-rule ${target ? "selected" : ""}`}>
      {/* Leaving the formula commits it. ⌘↵ still works, and still says so
          in the formula bar, but a list of rules is read and retyped in
          passing: a rule that silently reverts because somebody clicked
          away is the field asking for a Save button nobody should need. */}
      <div
        className="style-rule-head"
        onBlur={(event) => {
          if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
          if (draft.trim() !== formula.trim()) void onFormula(draft);
        }}
      >
        <FormulaEditor
          editorId={`style-rule:${rule.id}`}
          label="Rule formula"
          compact
          value={draft}
          references={references}
          frameId={frameId}
          placeholder="`Amount` < 0"
          onChange={(next) => {
            setDraft(next);
            onDraftCleared();
          }}
          onCommit={onFormula}
        />
        <select
          aria-label="Rule applies to"
          value={rule.columnId ?? ""}
          onChange={(event) =>
            void onRule({ ...rule, columnId: event.target.value || null })
          }
        >
          <option value="">Whole row</option>
          {columns.map((column) => (
            <option key={column.id} value={column.id}>
              {column.name}
            </option>
          ))}
        </select>
        {/* Only on the rule being edited: a delete control on every row is a
            column of buttons for a list that is read far more often than it
            is pruned. */}
        {target && (
          <button aria-label="Delete rule" title="Delete rule" onClick={onDelete}>
            <X size={12} />
          </button>
        )}
      </div>
      {error && <p className="style-rule-error">{error}</p>}
      <RuleStops rule={rule} target={target} onTarget={onTarget} onRule={onRule} />
    </div>
  );
}

/**
 * The rule's stops: every place it holds a style, as a swatch painted by the
 * style it holds.
 *
 * Which stops there are is the rule's own business — a condition has one, a
 * ramp has two or three, a category has one per value it found plus the
 * catch-all — so this draws the list it is given and offers the two things
 * that change its length: a value typed in by hand, and the ramp's middle.
 */
function RuleStops({
  rule,
  target,
  onTarget,
  onRule,
}: {
  rule: FrameStyleRule;
  target: RuleTarget | null;
  onTarget: (target: RuleTarget | null) => void;
  onRule: (rule: FrameStyleRule) => Promise<string | null>;
}) {
  const [caseDraft, setCaseDraft] = useState("");
  const addCase = (value: string) => {
    if (rule.output.kind !== "category") return;
    setCaseDraft("");
    // Seeded from "anything else", so a value split out of the catch-all
    // starts looking the way it already looked.
    const style = { ...stopStyle(rule, { kind: "other" }) };
    void onRule({
      ...rule,
      output: { ...rule.output, cases: [...rule.output.cases, { value, style }] },
    }).then((failure) => {
      if (!failure) onTarget({ ruleId: rule.id, stop: { kind: "case", value } });
    });
  };
  const removeStop = (stop: RuleStop) => {
    const next = ruleWithoutStop(rule, stop);
    if (!next) return;
    void onRule(next).then((failure) => {
      if (!failure) onTarget(null);
    });
  };
  return (
    <div className="style-rule-stops">
      {ruleStops(rule).map((stop) => {
        const active = Boolean(target && sameStop(target.stop, stop));
        const label = stopLabel(rule, stop);
        return (
          <span className="style-rule-stop-group" key={`${stop.kind}:${label}`}>
            <button
              className={`style-rule-stop ${active ? "active" : ""}`}
              aria-pressed={active}
              onClick={() => onTarget({ ruleId: rule.id, stop })}
            >
              {/* The stop's own style, on the stop: a swatch painted by the
                  thing it edits needs no legend. */}
              <span
                className="style-rule-swatch"
                style={frameCellStyleProperties(stopStyle(rule, stop))}
              >
                123
              </span>
              {label}
            </button>
            {/* Only two stops can go, and only while selected: a value the
                fill found that nobody cares about, and the middle somebody
                added. The rest are what the rule is. */}
            {active && ruleWithoutStop(rule, stop) && (
              <button
                className="style-rule-drop"
                aria-label={`Remove ${label}`}
                title={`Remove ${label}`}
                onClick={() => removeStop(stop)}
              >
                <Minus size={11} />
              </button>
            )}
          </span>
        );
      })}
      {rule.output.kind === "category" && (
        <input
          className="style-rule-case"
          aria-label="Style another value"
          placeholder="value…"
          value={caseDraft}
          onChange={(event) => setCaseDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || !caseDraft.trim()) return;
            addCase(caseDraft.trim());
          }}
        />
      )}
      {/* Each property owns its own ramp. The small, explicit additions are
          what allow red-yellow-green fill to coexist with two-colour ink;
          one shared "middle" switch would quietly couple them again. */}
      {rule.output.kind === "scale" &&
        (["text", "fill"] as const).map((property) => {
          const colors = rule.output.kind === "scale" ? rule.output.scale[property] : null;
          if (!colors || colors.mid) return null;
          return (
            <button
              key={property}
              className="style-rule-add-stop"
              onClick={() => {
                const next = ruleWithScaleMid(rule, property);
                if (next) void onRule(next);
              }}
            >
              <Plus size={11} /> {property} middle
            </button>
          );
        })}
    </div>
  );
}
