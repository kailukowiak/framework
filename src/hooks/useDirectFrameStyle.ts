import { useEffect, useRef } from "react";
import {
  emptyFrameCellStyle,
  exactFrameCellStyle,
  frameStyleRules,
  frameStyles,
  sameStyleTarget,
} from "../FrameGrid";
import {
  ruleInput,
  ruleWithStopStyle,
  type RuleStop,
} from "../lib/conditionalFormatting";
import { rectangleHasFormatting } from "../lib/frameStyleRanges";
import type { OperationHandler } from "../lib/handlers";
import { styleSelectionOperation, type StyleSelection } from "../lib/styleSelection";
import type {
  ComputedFrame,
  FrameCellStyle,
  FrameObject,
  FrameStyleRule,
  FrameStyleTarget,
  Selection,
} from "../lib/types";

/**
 * The format controls' one write path: a bold, a colour, an alignment, or
 * "clear formatting", landing on whatever the controls currently point at.
 *
 * That is one of three things. A rule stop being dressed takes the style as
 * its own (the controls are the same gesture whether they paint a cell or the
 * rule that paints it). A multi-cell, multi-row or multi-column selection
 * takes it as a patch over the whole selection (`styleSelectionOperation`).
 * Anything else is the single target the selection names.
 *
 * `pendingStyle` is the style the last gesture produced, kept in a ref
 * because two gestures can land before the document echoes the first back —
 * bold then italic in quick succession must send both, not italic alone
 * layered over a stale bold. It is reseeded from the document whenever the
 * thing being formatted changes.
 *
 * `hasFormatting` answers whether "clear formatting" has anything to clear
 * for the current target; over a rectangle that means any cell inside it.
 */
export function useDirectFrameStyle({
  frame,
  computed,
  selection,
  styleSelection,
  styleTarget,
  styleRule,
  activeStop,
  exactStyle,
  ruleKey,
  onOperation,
}: {
  frame: FrameObject;
  computed: ComputedFrame;
  selection: Selection;
  styleSelection?: StyleSelection | null;
  styleTarget: FrameStyleTarget;
  styleRule: FrameStyleRule | undefined;
  activeStop: RuleStop | null;
  exactStyle: FrameCellStyle;
  /** Identity of the rule target, so a new target reseeds the pending style. */
  ruleKey: unknown;
  onOperation: OperationHandler;
}) {
  const pendingStyleRef = useRef(exactStyle);
  useEffect(() => {
    pendingStyleRef.current = exactStyle;
  }, [frame.id, exactStyle, ruleKey, selection.columnId, selection.rowId]);

  const setDirectStyle = (patch: Partial<FrameCellStyle> | null) => {
    const style = patch
      ? { ...pendingStyleRef.current, ...patch }
      : emptyFrameCellStyle();
    pendingStyleRef.current = style;
    if (styleRule && activeStop) {
      const rules = frameStyleRules(frame);
      const edited = ruleWithStopStyle(styleRule, activeStop, style);
      void onOperation({
        type: "setFrameStyleRules",
        frameId: frame.id,
        rules: rules.map((rule) =>
          ruleInput(rule.id === edited.id ? edited : rule, computed.styleRuleFormulas ?? {})
        ),
      });
      return;
    }
    if (styleSelection) {
      const operation = styleSelectionOperation(
        frame.id,
        styleSelection,
        patch,
        (target) => exactFrameCellStyle(frame, target),
        emptyFrameCellStyle()
      );
      if (operation) void onOperation(operation);
      return;
    }
    void onOperation({
      type: "setFrameStyle",
      frameId: frame.id,
      target: styleTarget,
      style,
    });
  };

  const hasFormatting =
    styleSelection?.kind === "range"
      ? rectangleHasFormatting(
          frameStyles(frame),
          styleSelection.rowIds,
          styleSelection.columnIds
        )
      : frameStyles(frame).some((entry) => sameStyleTarget(entry.target, styleTarget));

  return { setDirectStyle, hasFormatting };
}
