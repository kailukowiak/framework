import { useEffect, useState } from "react";
import { Palette, Pencil, Plus } from "lucide-react";
import { ContextMenuSurface } from "./ContextMenuSurface";
import { frameStyleRules } from "./FrameGrid";
import { StyleRuleRow, type RuleTarget } from "./StyleRuleRow";
import { frameFormulaValues } from "./lib/api";
import {
  CATEGORY_FILLS,
  candidateOutputs,
  categoryOutput,
  defaultOutputFor,
  quoteColumn,
  stylePresets,
  ruleInput,
  type StylePreset,
} from "./lib/conditionalFormatting";
import type { FormulaReference } from "./lib/formulaReferences";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  ComputedFrame,
  FrameObject,
  FrameStyleOutput,
  FrameStyleRule,
  FrameStyleRuleInput,
  Selection,
} from "./lib/types";

export type { RuleTarget };

/**
 * The Rules list: one line per rule, and its stops underneath while it is
 * selected.
 *
 * There is no rule-kind picker and no second set of formatting controls:
 * what the formula returns is what the rule is, and the dressing is the
 * Format controls above aimed at whichever stop is selected.
 */
export function ConditionalFormattingRules({
  frame,
  computed,
  selection,
  references,
  target,
  onTarget,
  onOperation,
}: {
  frame: FrameObject;
  computed: ComputedFrame;
  selection: Selection;
  references: FormulaReference[];
  target: RuleTarget | null;
  onTarget: (target: RuleTarget | null) => void;
  onOperation: OperationHandler;
}) {
  const rules = frameStyleRules(frame);
  const formulas = computed.styleRuleFormulas ?? {};
  const errors = computed.styleRuleErrors ?? {};
  const [error, setError] = useState<string | null>(null);
  const commit = async (next: FrameStyleRuleInput[]) => {
    const failure = await onOperation({
      type: "setFrameStyleRules",
      frameId: frame.id,
      rules: next,
    });
    setError(failure ?? null);
    return failure;
  };
  const inputs = () => rules.map((rule) => ruleInput(rule, formulas));
  const replace = (ruleId: string, next: FrameStyleRuleInput) =>
    commit(inputs().map((input) => (input.id === ruleId ? next : input)));
  const replaceRule = (rule: FrameStyleRule) =>
    replace(rule.id, ruleInput(rule, formulas));

  /**
   * A category rule with the values the column actually holds already in it,
   * each wearing a color.
   *
   * The reason a rule over text used to arrive empty is that only the engine
   * knows what the formula answers, so the panel asked nobody and offered a
   * catch-all. Asking is one call, and it turns "color by value" from a
   * starting point into the finished thing with every square still editable.
   *
   * One more value is requested than there are colors, which is how the fill
   * finds out it did not cover everything and keeps the catch-all for the
   * tail. A formula the engine will not run answers nothing, and the rule
   * commits unfilled: the refusal that matters is the one the commit itself
   * reports, said once, against the rule.
   */
  const filled = async (
    formula: string,
    output: FrameStyleOutput
  ): Promise<FrameStyleOutput> => {
    if (output.kind !== "category") return output;
    try {
      const values = await frameFormulaValues(
        frame.id,
        formula,
        CATEGORY_FILLS.length + 1
      );
      return categoryOutput(values, output);
    } catch {
      return output;
    }
  };

  /**
   * A rewritten formula, offered to the reading the rule already had and
   * then to the others. Only the core knows what a formula returns, and it
   * says so by taking the rule or refusing it.
   */
  const commitFormula = async (rule: FrameStyleRule, formula: string) => {
    if (formula.trim() === (formulas[rule.id] ?? "").trim()) return;
    let failure: string | null = null;
    for (const output of candidateOutputs(rule.output)) {
      failure = await replace(rule.id, {
        id: rule.id,
        formula,
        columnId: rule.columnId ?? null,
        // Refilled rather than kept: the values a rewritten formula answers
        // are not the ones the old one did, and a case list left over from
        // the previous formula is a mapping to values that no longer occur.
        // Styles for values that survive the rewrite survive with them.
        output: await filled(formula, output),
      });
      if (!failure) return;
    }
    setError(failure);
  };

  // The column a new rule is about: whatever is selected, because a rule
  // about the column someone is looking at is the rule they are about to
  // write. An empty selection starts from the first column, so the seeded
  // formula is a real one rather than a blank.
  const subject =
    frame.columns.find((candidate) => candidate.id === selection.columnId) ??
    frame.columns[0];

  const addRule = async (preset?: StylePreset) => {
    if (!subject) return;
    const formula = preset?.formula ?? quoteColumn(subject.name);
    const failure = await commit([
      ...inputs(),
      {
        formula,
        columnId: subject.id,
        output: await filled(
          formula,
          preset?.output ?? defaultOutputFor(subject.dataType)
        ),
      },
    ]);
    if (!failure) onTarget(null);
  };

  const removeRule = async (rule: FrameStyleRule) => {
    const failure = await commit(inputs().filter((input) => input.id !== rule.id));
    if (!failure && target?.ruleId === rule.id) onTarget(null);
  };

  return (
    <div className="style-rules">
      <NewRuleMenu frameName={frame.name} subject={subject} onAdd={addRule} />
      {rules.length === 0 && (
        <p className="style-rules-empty">
          A rule is a formula over each row, and the formula is where the work
          happens. <code>`Amount` &lt; 0</code> paints the rows it answers true
          for. Text sorts rows into named values, filled in from the data —{" "}
          <code>when(`Stat`).then(&quot;Holiday&quot;).otherwise(&quot;Work&quot;)</code>{" "}
          names its own. A number is a position from 0 to 1, so{" "}
          <code>`Amount`.normalize()</code> is a heatmap and{" "}
          <code>.normalize(center=0)</code> turns at zero. A rule reads the whole
          row, so <code>`Weekend`</code> can paint <em>Day</em> — that is what{" "}
          <em>applies to</em> chooses. <em>Rule</em> offers the usual ones for the
          selected column.
        </p>
      )}
      {rules.map((rule) => (
        <StyleRuleRow
          key={rule.id}
          rule={rule}
          columns={frame.columns}
          formula={formulas[rule.id] ?? ""}
          frameId={frame.id}
          references={references}
          error={errors[rule.id]}
          target={target?.ruleId === rule.id ? target : null}
          onTarget={onTarget}
          onFormula={(value) => commitFormula(rule, value)}
          onRule={replaceRule}
          onDelete={() => void removeRule(rule)}
          onDraftCleared={() => setError(null)}
        />
      ))}
      {error && <p className="style-rule-error">{error}</p>}
    </div>
  );
}

/**
 * The heading and the menu behind *Rule*: the rules worth offering for the
 * column somebody is looking at.
 *
 * Offered by type, because the reading is decided by type — a ramp over text
 * has no ends, "above average" over a label means nothing — and every one of
 * them is an ordinary rule once made, formula and all, right there on the row
 * to edit. That is the difference between a preset and a mode.
 */
function NewRuleMenu({
  frameName,
  subject,
  onAdd,
}: {
  frameName: string;
  subject: Column | undefined;
  onAdd: (preset?: StylePreset) => Promise<void>;
}) {
  // Where the menu hangs, in window coordinates, or null when it is closed.
  // The inspector scrolls, so the anchor is taken at the click.
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    if (!menuAt) return;
    const dismiss = () => setMenuAt(null);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") dismiss();
    };
    // The surface stops its own pointerdown, so anywhere else closes it --
    // the same bargain every other menu in the app makes.
    window.addEventListener("pointerdown", dismiss);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", dismiss);
      window.removeEventListener("keydown", onKey);
    };
  }, [menuAt]);
  const add = (preset?: StylePreset) => {
    setMenuAt(null);
    void onAdd(preset);
  };
  return (
    <>
      <div className="style-rules-heading">
        <span>Rules</span>
        <button
          className="style-rule-add"
          aria-label="Add rule"
          onPointerDown={(event) => {
            // On the press, and stopped here: the same pointerdown that
            // opens the menu would otherwise reach the window listener that
            // closes it.
            event.stopPropagation();
            const bounds = event.currentTarget.getBoundingClientRect();
            setMenuAt(menuAt ? null : { x: bounds.right - 224, y: bounds.bottom + 3 });
          }}
        >
          <Plus size={12} /> Rule
        </button>
      </div>
      {menuAt && subject && (
        <ContextMenuSurface x={menuAt.x} y={menuAt.y}>
          <div className="context-menu-heading">
            <span>New rule</span>
            <strong>
              {frameName} / {subject.name}
            </strong>
          </div>
          {stylePresets(subject).map((preset) => (
            <button key={preset.id} onClick={() => add(preset)}>
              <Palette size={14} />
              {preset.label}
            </button>
          ))}
          <button onClick={() => add()}>
            <Pencil size={14} />
            Write my own
          </button>
        </ContextMenuSurface>
      )}
    </>
  );
}
