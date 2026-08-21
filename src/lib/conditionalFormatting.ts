// Editing conditional-formatting rules: which piece of a rule the format
// controls are pointed at, and what a style change to that piece does to the
// rule.
//
// A rule holds style in a different place depending on what its formula
// returns -- one style for a condition, one per listed value for a category,
// a color per stop of a ramp -- and the panel that edits them is the same
// panel that edits a cell. The reconciliation is here: every one of those
// places is a *stop*, a stop has a `FrameCellStyle`, and pointing the
// controls at a stop is the same gesture as pointing them at a cell.

import { CATEGORY_FILLS } from "./palette";
import type {
  DataType,
  FrameCellStyle,
  FrameStyleCase,
  FrameStyleOutput,
  FrameStyleRule,
  FrameStyleRuleInput,
  FrameStyleScale,
} from "./types";

/** The fill a new rule starts with, so a rule is visible before it is dressed. */
const SEED_FILL = "#fff0c7";

export { CATEGORY_FILLS };


/** The two ends and the turn of the ramp a heatmap preset starts from. */
const HEATMAP = { low: "#ffffff", high: "#8da293" };
/** Loss, neutral, gain — the diverging ramp, in this document's register. */
const DIVERGING = { low: "#c9755a", mid: "#f6f3ec", high: "#5c8a6d" };

/** One editable piece of a rule: something that has a style of its own. */
export type RuleStop =
  | { kind: "condition" }
  | { kind: "case"; value: string }
  | { kind: "other" }
  | { kind: "scale"; end: ScaleEnd };

export type ScaleEnd = "low" | "mid" | "high";

export const emptyStyle = (): FrameCellStyle => ({
  bold: null,
  italic: null,
  underline: null,
  textColor: null,
  fillColor: null,
  alignment: null,
  lineStyle: null,
});

export function sameStop(left: RuleStop, right: RuleStop): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

/** Every stop a rule currently has, in the order the panel lists them. */
export function ruleStops(rule: FrameStyleRule): RuleStop[] {
  switch (rule.output.kind) {
    case "condition":
      return [{ kind: "condition" }];
    case "category":
      return [
        ...rule.output.cases.map((entry) => ({ kind: "case" as const, value: entry.value })),
        { kind: "other" },
      ];
    case "scale":
      return [
        { kind: "scale", end: "low" },
        // Only when there is one: a ramp with no middle has two stops, and
        // showing an empty third would be inventing a color nobody chose.
        ...(hasScaleMid(rule.output.scale)
          ? [{ kind: "scale" as const, end: "mid" as const }]
          : []),
        { kind: "scale", end: "high" },
      ];
  }
}

/** What a stop stands for, said in the fewest words that stay honest. */
export function stopLabel(rule: FrameStyleRule, stop: RuleStop): string {
  switch (stop.kind) {
    case "condition":
      return "when true";
    case "case":
      return stop.value;
    case "other":
      return "anything else";
    case "scale":
      return stop.end === "low" ? "lowest" : stop.end === "mid" ? "middle" : "highest";
  }
}

/**
 * The style a stop carries right now.
 *
 * A ramp stop is a color rather than a style, so it is presented as the style
 * it produces — which is also what makes the ordinary color controls able to
 * edit it without knowing it is a ramp.
 */
export function stopStyle(rule: FrameStyleRule, stop: RuleStop): FrameCellStyle {
  const output = rule.output;
  if (output.kind === "condition" && stop.kind === "condition") return output.style;
  if (output.kind === "category" && stop.kind === "case")
    return (
      output.cases.find((entry) => entry.value === stop.value)?.style ?? emptyStyle()
    );
  if (output.kind === "category" && stop.kind === "other")
    return output.other ?? emptyStyle();
  if (output.kind === "scale" && stop.kind === "scale") {
    return {
      ...emptyStyle(),
      textColor: scaleStop(output.scale.text ?? null, stop.end),
      fillColor: scaleStop(output.scale.fill ?? null, stop.end),
    };
  }
  return emptyStyle();
}

type ColorScale = NonNullable<FrameStyleScale["fill"]>;

/** The color a ramp keeps at one of its three places, if it has one there. */
export function scaleStop(
  scale: ColorScale | null,
  end: ScaleEnd
): string | null {
  if (!scale) return null;
  if (end === "low") return scale.low;
  if (end === "high") return scale.high;
  return scale.mid ?? null;
}

/** The same ramp with one stop recolored. */
export function scaleWithStop(
  scale: ColorScale,
  end: ScaleEnd,
  color: string
): ColorScale {
  if (end === "low") return { ...scale, low: color };
  if (end === "high") return { ...scale, high: color };
  return { ...scale, mid: color };
}

/** Whether either property has the optional turn at the middle stop. */
export function hasScaleMid(scale: FrameStyleScale): boolean {
  return Boolean(scale.text?.mid || scale.fill?.mid);
}

/** The color properties a scale currently paints, for the active-target line. */
export function scalePropertyLabel(scale: FrameStyleScale): string {
  if (scale.text && scale.fill) return "Text + fill";
  return scale.text ? "Text" : "Fill";
}

/** A newly-added property starts flat until its other stops are recolored. */
function colorScale(color: string, middle: boolean): ColorScale {
  return { low: color, high: color, mid: middle ? color : null };
}

/**
 * The rule that results from changing one stop's style.
 *
 * A ramp takes color only, but text and fill are independent ramps over the
 * same positions. Adding one property starts it flat at the chosen color;
 * its other stops can then be moved without disturbing the property already
 * there. Clearing an end clears that property because a one-ended ramp has
 * no defined interpolation; clearing a middle turns just that property back
 * into a two-colour ramp.
 */
export function ruleWithStopStyle(
  rule: FrameStyleRule,
  stop: RuleStop,
  style: FrameCellStyle
): FrameStyleRule {
  const output = rule.output;
  if (output.kind === "condition" && stop.kind === "condition")
    return { ...rule, output: { ...output, style } };
  if (output.kind === "category" && stop.kind === "case") {
    const cases: FrameStyleCase[] = output.cases.map((entry) =>
      entry.value === stop.value ? { ...entry, style } : entry
    );
    return { ...rule, output: { ...output, cases } };
  }
  if (output.kind === "category" && stop.kind === "other")
    return { ...rule, output: { ...output, other: styleIsEmpty(style) ? null : style } };
  if (output.kind === "scale" && stop.kind === "scale") {
    const update = (current: ColorScale | null | undefined, color: string | null) => {
      if (color)
        return scaleWithStop(
          current ?? colorScale(color, stop.end === "mid"),
          stop.end,
          color
        );
      // A missing middle means this property is intentionally a two-colour
      // ramp. It must not erase that whole ramp when the other property is
      // given a middle of its own. At an end, by contrast, there is no valid
      // one-ended ramp, so clearing the colour clears that property.
      if (stop.end === "mid" && current) return { ...current, mid: null };
      return null;
    };
    const scale: FrameStyleScale = {
      text: update(output.scale.text, style.textColor),
      fill: update(output.scale.fill, style.fillColor),
    };
    // The engine correctly refuses a scale that paints nothing. The panel
    // disables clearing the last property, and this guard keeps a stale UI
    // gesture from manufacturing an invalid operation anyway.
    if (!scale.text && !scale.fill) return rule;
    return { ...rule, output: { ...output, scale } };
  }
  return rule;
}

/**
 * The rule without one of its stops.
 *
 * Only two stops can be dropped, and for the same reason: they are the two
 * somebody added. A category's values are filled from the data, so a rule
 * over a column with a value nobody cares about needs a way to stop caring
 * about it; a ramp's middle is opt-in, so it is opt-out. The stops that
 * remain — a condition's one style, a ramp's two ends, the catch-all — are
 * what the rule *is*, and removing one would leave a rule that says nothing.
 */
export function ruleWithoutStop(
  rule: FrameStyleRule,
  stop: RuleStop
): FrameStyleRule | null {
  const output = rule.output;
  if (output.kind === "category" && stop.kind === "case") {
    const cases = output.cases.filter((entry) => entry.value !== stop.value);
    // The engine refuses a rule that styles nothing, which is right — so the
    // last value going means the catch-all has to hold the rule up.
    const other = cases.length === 0 && !output.other ? fill(SEED_FILL) : output.other;
    return { ...rule, output: { ...output, cases, other } };
  }
  if (output.kind === "scale" && stop.kind === "scale" && stop.end === "mid")
    return {
      ...rule,
      output: {
        ...output,
        scale: {
          text: output.scale.text ? { ...output.scale.text, mid: null } : null,
          fill: output.scale.fill ? { ...output.scale.fill, mid: null } : null,
        },
      },
    };
  return null;
}

/** The same ramp with a middle, halfway between its ends until moved. */
export function ruleWithScaleMid(
  rule: FrameStyleRule,
  property?: "text" | "fill"
): FrameStyleRule | null {
  if (rule.output.kind !== "scale") return null;
  const scale = rule.output.scale;
  const withMiddle = (colors: ColorScale | null | undefined) =>
    colors
      ? { ...colors, mid: mixColors(colors.low, colors.high, 0.5) }
      : null;
  const addText = (!property || property === "text") && scale.text && !scale.text.mid;
  const addFill = (!property || property === "fill") && scale.fill && !scale.fill.mid;
  if (!addText && !addFill) return null;
  return {
    ...rule,
    output: {
      ...rule.output,
      scale: {
        text: addText ? withMiddle(scale.text) : scale.text,
        fill: addFill ? withMiddle(scale.fill) : scale.fill,
      },
    },
  };
}

/** Halfway between two `#rrggbb` colors, channel by channel. */
export function mixColors(low: string, high: string, position: number): string {
  const channels = (color: string) =>
    [1, 3, 5].map((offset) => parseInt(color.slice(offset, offset + 2), 16) || 0);
  const [from, to] = [channels(low), channels(high)];
  return `#${from
    .map((value, index) =>
      Math.round(value + (to[index] - value) * position)
        .toString(16)
        .padStart(2, "0")
    )
    .join("")}`;
}

export function styleIsEmpty(style: FrameCellStyle): boolean {
  return Object.values(style).every((value) => value === null || value === undefined);
}

/**
 * A stored rule as the operation takes it: the formula back as text, which
 * is the only form the interface ever holds a formula in.
 */
export function ruleInput(
  rule: FrameStyleRule,
  formulas: Record<string, string>
): FrameStyleRuleInput {
  return {
    id: rule.id,
    formula: formulas[rule.id] ?? "",
    columnId: rule.columnId ?? null,
    output: rule.output,
  };
}

/**
 * A category rule over the values the engine actually found, each with a
 * color, ready to edit.
 *
 * This is what "auto" means for text: nobody types out the six statuses
 * their column has in order to color them, because the column already knows
 * what they are. Values already styled keep exactly the style they had —
 * refilling after new rows arrive adds the newcomers and disturbs nothing —
 * and new ones take the next fill no listed value is already wearing, so a
 * second pass does not hand out a color that is already on screen.
 *
 * Past the end of the palette the catch-all comes back, because a mapping
 * with more entries than there are tellable-apart colors is not a mapping.
 */
export function categoryOutput(
  values: string[],
  existing?: Extract<FrameStyleOutput, { kind: "category" }>
): FrameStyleOutput {
  const listed = existing?.cases ?? [];
  const kept = listed.filter((entry) => values.includes(entry.value));
  const taken = new Set(
    kept.map((entry) => entry.style.fillColor).filter((color): color is string => !!color)
  );
  const unused = CATEGORY_FILLS.filter((color) => !taken.has(color));
  let next = 0;
  const cases = values.slice(0, CATEGORY_FILLS.length).map((value) => {
    const already = kept.find((entry) => entry.value === value);
    if (already) return already;
    // Round-robin once the unused colors run out: repeating a fill is a
    // worse answer than leaving a value unpainted, but only just, and the
    // alternative to both is inventing colors nobody picked.
    const color = unused[next] ?? CATEGORY_FILLS[next % CATEGORY_FILLS.length];
    next += 1;
    return { value, style: fill(color) };
  });
  return {
    kind: "category",
    cases,
    // The catch-all stays, always. A list filled from the data covers the
    // values that were there when it was filled, and the data moves: a live
    // CSV refreshes, a filter opens up, somebody types a section that did
    // not exist. Those rows want a color that says "not one of the ones I
    // named" rather than no color at all, which reads as a rule that
    // stopped working. An existing rule keeps whatever catch-all its owner
    // chose, including none.
    other: existing ? existing.other ?? null : fill(SEED_FILL),
  };
}

/**
 * How a rule reads a formula that returns `dataType`.
 *
 * Nobody is asked which kind of rule they want: a formula that answers true
 * or false picks rows, one that answers text sorts them into cases, one that
 * answers a number ramps them. Asking would be asking someone to restate
 * what they have already typed.
 */
export function defaultOutputFor(dataType: DataType): FrameStyleOutput {
  switch (dataType) {
    case "boolean":
      return { kind: "condition", style: fill(SEED_FILL) };
    case "string":
    case "categorical":
    case "date":
      return { kind: "category", cases: [], other: fill(SEED_FILL) };
    // Integer, number, currency, percentage: all numbers, whatever they are
    // dressed as, and a number is a position on a ramp.
    default:
      return { kind: "scale", scale: heatmap() };
  }
}

/** A column name as a formula names it. */
export function quoteColumn(name: string): string {
  return `\`${name.replace(/`/g, "``")}\``;
}

/** A rule somebody can have without writing it. */
export type StylePreset = {
  id: string;
  label: string;
  formula: string;
  output: FrameStyleOutput;
};

const fill = (color: string): FrameCellStyle => ({ ...emptyStyle(), fillColor: color });
const text = (color: string): FrameCellStyle => ({ ...emptyStyle(), textColor: color });

const heatmap = (): FrameStyleScale => ({
  text: null,
  fill: { low: HEATMAP.low, high: HEATMAP.high, mid: null },
});

/**
 * The rules worth offering for a column, most-wanted first.
 *
 * Every preset is an ordinary rule — a formula and a reading — so there is
 * nothing a preset can express that someone could not have typed, and
 * nothing about one that resists being edited afterwards. What they save is
 * the blank page: "heatmap this column" is a thing people want by name, and
 * making them write `\`Amount\`` and then find the ramp is making them spell
 * out an intention they already stated by clicking.
 *
 * They are offered by type because the reading is decided by type: a ramp
 * over text has no ends, and "above average" over a label means nothing.
 */
export function stylePresets(column: {
  name: string;
  dataType: DataType;
}): StylePreset[] {
  const name = quoteColumn(column.name);
  const blanks: StylePreset = {
    id: "blanks",
    label: "Blank cells",
    formula: `${name}.is_null()`,
    output: { kind: "condition", style: fill("#f8dfd0") },
  };
  switch (column.dataType) {
    case "integer":
    case "number":
    case "currency":
    case "percentage":
      return [
        {
          // `.normalize()` rather than the bare column: a ramp paints a
          // position between 0 and 1, and saying so in the formula is what
          // makes every variation of it an edit rather than a control --
          // `.normalize(0, 100)` to pin the ends, `.clip(...)` first to
          // ignore outliers, a `when(...)` in front to substitute a value
          // from another column.
          id: "heatmap",
          label: "Heatmap, low to high",
          formula: `${name}.normalize()`,
          output: { kind: "scale", scale: heatmap() },
        },
        {
          // The ramp the two-color one cannot express: the turn is pinned at
          // zero rather than at the middle of the data, so a column running
          // from -90 to 4,000 still reads red below zero.
          id: "diverging",
          label: "Diverging around zero",
          // Zero lands at the middle and the two directions away from it get
          // equal room, so the turn means zero rather than the midpoint of
          // whatever range the rows happen to cover.
          formula: `${name}.normalize(center=0)`,
          output: {
            kind: "scale",
            scale: {
              text: null,
              fill: {
                low: DIVERGING.low,
                high: DIVERGING.high,
                mid: DIVERGING.mid,
              },
            },
          },
        },
        {
          id: "above-average",
          label: "Above average",
          // The average of the column, not of the page: the rule's hidden
          // column is computed above the slice precisely so this is true.
          formula: `${name} > ${name}.mean()`,
          output: { kind: "condition", style: fill(SEED_FILL) },
        },
        {
          id: "top-tenth",
          label: "Top 10%",
          formula: `${name} >= ${name}.quantile(0.9)`,
          output: { kind: "condition", style: fill("#dce9df") },
        },
        {
          id: "bottom-tenth",
          label: "Bottom 10%",
          formula: `${name} <= ${name}.quantile(0.1)`,
          output: { kind: "condition", style: fill("#f6ddcd") },
        },
        {
          id: "negative",
          label: "Negative in red",
          formula: `${name} < 0`,
          output: { kind: "condition", style: text("#9a452b") },
        },
        blanks,
      ];
    case "boolean":
      return [
        {
          id: "true",
          label: "Highlight true",
          formula: name,
          output: { kind: "condition", style: fill(SEED_FILL) },
        },
        {
          id: "false",
          label: "Highlight false",
          formula: `${name}.not()`,
          output: { kind: "condition", style: fill("#f8dfd0") },
        },
        blanks,
      ];
    case "string":
    case "categorical":
      return [
        {
          // Empty on purpose: the values are filled in from the data at the
          // moment the rule is made, which is a thing only the engine can
          // answer. See `categoryOutput`.
          id: "by-value",
          label: "A color per value",
          formula: name,
          output: { kind: "category", cases: [], other: fill(SEED_FILL) },
        },
        blanks,
      ];
    case "date":
      return [
        {
          id: "weekends",
          // Monday is 1, so 6 and 7 are the weekend.
          label: "Weekends",
          formula: `${name}.dt.weekday() > 5`,
          output: { kind: "condition", style: fill("#ebdcee") },
        },
        {
          id: "future",
          label: "In the future",
          formula: `${name} > today()`,
          output: { kind: "condition", style: fill("#dce9df") },
        },
        {
          id: "stale",
          label: "Older than 30 days",
          // Read when the frame is read, not when the rule was written, so
          // "thirty days" keeps meaning thirty days from today.
          formula: `${name} < today().dt.offset_by("-30d")`,
          output: { kind: "condition", style: fill("#f6ddcd") },
        },
        blanks,
      ];
    default:
      return [blanks];
  }
}

/**
 * The readings to try for a rule whose formula has just been rewritten, best
 * guess first.
 *
 * The core is the only thing that knows what a formula returns, and it says
 * so by accepting or refusing the rule. So an edit offers what the rule
 * already was, then the other two readings — which turns "I retyped this
 * rule as `amount < 0`" into a rule that picks rows, rather than an error
 * telling someone the reading they never chose no longer fits.
 */
export function candidateOutputs(current: FrameStyleOutput): FrameStyleOutput[] {
  const byKind: Record<FrameStyleOutput["kind"], FrameStyleOutput> = {
    condition: defaultOutputFor("boolean"),
    category: defaultOutputFor("string"),
    scale: defaultOutputFor("number"),
  };
  return [
    current,
    ...(["condition", "category", "scale"] as const)
      .filter((kind) => kind !== current.kind)
      .map((kind) => byKind[kind]),
  ];
}
