import { describe, expect, it } from "vitest";
import {
  CATEGORY_FILLS,
  candidateOutputs,
  categoryOutput,
  defaultOutputFor,
  quoteColumn,
  stylePresets,
  emptyStyle,
  ruleInput,
  ruleStops,
  ruleWithScaleMid,
  ruleWithStopStyle,
  ruleWithoutStop,
  stopLabel,
  stopStyle,
} from "./conditionalFormatting";
import type { FrameStyleOutput, FrameStyleRule, FrameStyleScale } from "./types";

const rule = (overrides: Partial<FrameStyleRule>): FrameStyleRule => ({
  id: "rule-1",
  formula: { expression: null },
  columnId: null,
  output: { kind: "condition", style: { ...emptyStyle(), bold: true } },
  ...overrides,
});

const ramp = (scale: Partial<FrameStyleScale> = {}): FrameStyleRule =>
  rule({
    output: {
      kind: "scale",
      scale: {
        text: null,
        fill: { low: "#ffffff", high: "#315c49", mid: null },
        ...scale,
      },
    },
  });

describe("ruleStops", () => {
  it("gives a condition one stop and a ramp its two ends", () => {
    expect(ruleStops(rule({}))).toEqual([{ kind: "condition" }]);
    expect(ruleStops(ramp())).toEqual([
      { kind: "scale", end: "low" },
      { kind: "scale", end: "high" },
    ]);
  });

  it("gives a ramp a third stop once it has a middle, between its ends", () => {
    const diverging = ruleWithScaleMid(ramp());
    expect(diverging && ruleStops(diverging)).toEqual([
      { kind: "scale", end: "low" },
      { kind: "scale", end: "mid" },
      { kind: "scale", end: "high" },
    ]);
    // Halfway between the ends until moved, so adding one changes where the
    // ramp turns rather than what it looks like at either end.
    expect(diverging && stopStyle(diverging, { kind: "scale", end: "mid" }).fillColor).toBe(
      "#98aea4"
    );
    expect(diverging && stopLabel(diverging, { kind: "scale", end: "mid" })).toBe("middle");
    // And it is the one ramp stop that can go again.
    expect(ruleWithoutStop(ramp(), { kind: "scale", end: "low" })).toBeNull();
    expect(
      diverging && ruleWithoutStop(diverging, { kind: "scale", end: "mid" })
    ).toEqual(ramp());
  });

  it("lists a category's values and keeps the catch-all last", () => {
    const categorical = rule({
      output: {
        kind: "category",
        cases: [
          { value: "Refunded", style: { ...emptyStyle(), fillColor: "#f8dfd0" } },
          { value: "Open", style: { ...emptyStyle(), bold: true } },
        ],
        other: null,
      },
    });
    expect(ruleStops(categorical)).toEqual([
      { kind: "case", value: "Refunded" },
      { kind: "case", value: "Open" },
      { kind: "other" },
    ]);
    expect(ruleStops(categorical).map((stop) => stopLabel(categorical, stop))).toEqual([
      "Refunded",
      "Open",
      "anything else",
    ]);
  });
});

describe("stopStyle", () => {
  it("presents a ramp end as the style it produces", () => {
    const ramp = rule({
      output: {
        kind: "scale",
        scale: {
          text: { low: "#ffffff", high: "#315c49", mid: null },
          fill: null,
        },
      },
    });
    expect(stopStyle(ramp, { kind: "scale", end: "high" }).textColor).toBe("#315c49");
    expect(stopStyle(ramp, { kind: "scale", end: "high" }).fillColor).toBeNull();
  });

  it("answers an empty style for a stop the rule does not have", () => {
    expect(stopStyle(rule({}), { kind: "other" })).toEqual(emptyStyle());
  });
});

describe("ruleWithStopStyle", () => {
  it("replaces the style of the case it names and leaves the others alone", () => {
    const categorical = rule({
      output: {
        kind: "category",
        cases: [
          { value: "Refunded", style: { ...emptyStyle(), fillColor: "#f8dfd0" } },
          { value: "Open", style: { ...emptyStyle(), bold: true } },
        ],
        other: null,
      },
    });
    const edited = ruleWithStopStyle(categorical, { kind: "case", value: "Open" }, {
      ...emptyStyle(),
      italic: true,
    });
    expect(edited.output).toEqual({
      kind: "category",
      cases: [
        { value: "Refunded", style: { ...emptyStyle(), fillColor: "#f8dfd0" } },
        { value: "Open", style: { ...emptyStyle(), italic: true } },
      ],
      other: null,
    });
  });

  it("drops the catch-all when it is cleared rather than storing an empty style", () => {
    const categorical = rule({
      output: {
        kind: "category",
        cases: [],
        other: { ...emptyStyle(), fillColor: "#fff0c7" },
      },
    });
    expect(ruleWithStopStyle(categorical, { kind: "other" }, emptyStyle()).output).toEqual({
      kind: "category",
      cases: [],
      other: null,
    });
  });

  it("adds a second color ramp without discarding the first", () => {
    const edited = ruleWithStopStyle(ramp(), { kind: "scale", end: "low" }, {
      ...emptyStyle(),
      textColor: "#9a452b",
      fillColor: "#ffffff",
    });
    expect(edited.output).toEqual({
      kind: "scale",
      scale: {
        text: { low: "#9a452b", high: "#9a452b", mid: null },
        fill: { low: "#ffffff", high: "#315c49", mid: null },
      },
    });
  });

  it("clears one property while leaving the other ramp alone", () => {
    const both = ruleWithStopStyle(ramp(), { kind: "scale", end: "low" }, {
      ...emptyStyle(),
      textColor: "#9a452b",
      fillColor: "#ffffff",
    });
    const fillOnly = ruleWithStopStyle(both, { kind: "scale", end: "low" }, {
      ...emptyStyle(),
      fillColor: "#ffffff",
    });
    expect(fillOnly.output.kind === "scale" && fillOnly.output.scale.text).toBeNull();
    expect(fillOnly.output.kind === "scale" && fillOnly.output.scale.fill?.high).toBe(
      "#315c49"
    );
  });

  it("lets fill have a middle while text remains a two-colour ramp", () => {
    const both = ramp({
      text: { low: "#315cbb", high: "#b43ca8", mid: null },
      fill: { low: "#c94f45", mid: "#f2cf45", high: "#4e9a62" },
    });
    const edited = ruleWithStopStyle(both, { kind: "scale", end: "mid" }, {
      ...stopStyle(both, { kind: "scale", end: "mid" }),
      fillColor: "#e5bd36",
    });
    expect(edited.output.kind === "scale" && edited.output.scale).toEqual({
      text: { low: "#315cbb", high: "#b43ca8", mid: null },
      fill: { low: "#c94f45", mid: "#e5bd36", high: "#4e9a62" },
    });
  });

  it("does not copy fill's middle when text is first added at an end", () => {
    const fillOnly = ramp({
      fill: { low: "#c94f45", mid: "#f2cf45", high: "#4e9a62" },
    });
    const edited = ruleWithStopStyle(fillOnly, { kind: "scale", end: "low" }, {
      ...stopStyle(fillOnly, { kind: "scale", end: "low" }),
      textColor: "#315cbb",
    });
    expect(edited.output.kind === "scale" && edited.output.scale.text).toEqual({
      low: "#315cbb",
      mid: null,
      high: "#315cbb",
    });
  });

  it("lets each category carry an independent text and fill pair", () => {
    const categorical = rule({
      output: {
        kind: "category",
        cases: [{ value: "Late", style: { ...emptyStyle(), fillColor: "#c94f45" } }],
        other: null,
      },
    });
    const edited = ruleWithStopStyle(categorical, { kind: "case", value: "Late" }, {
      ...stopStyle(categorical, { kind: "case", value: "Late" }),
      textColor: "#315cbb",
    });
    expect(
      edited.output.kind === "category" && edited.output.cases[0].style
    ).toMatchObject({ textColor: "#315cbb", fillColor: "#c94f45" });
  });

  it("leaves a ramp alone when handed something that is not a color", () => {
    const ramp = rule({
      output: {
        kind: "scale",
        scale: {
          text: null,
          fill: { low: "#ffffff", high: "#315c49", mid: null },
        },
      },
    });
    expect(ruleWithStopStyle(ramp, { kind: "scale", end: "low" }, {
      ...emptyStyle(),
      bold: true,
    })).toEqual(ramp);
  });
});

describe("ruleInput", () => {
  it("carries the formula back as the text the core rendered", () => {
    expect(ruleInput(rule({ columnId: "col-1" }), { "rule-1": "`Amount` < 0" })).toEqual({
      id: "rule-1",
      formula: "`Amount` < 0",
      columnId: "col-1",
      output: { kind: "condition", style: { ...emptyStyle(), bold: true } },
    });
  });
});

describe("defaultOutputFor and candidateOutputs", () => {
  it("reads each type the way that type can be read", () => {
    expect(defaultOutputFor("boolean").kind).toBe("condition");
    expect(defaultOutputFor("string").kind).toBe("category");
    expect(defaultOutputFor("categorical").kind).toBe("category");
    expect(defaultOutputFor("number").kind).toBe("scale");
    expect(defaultOutputFor("integer").kind).toBe("scale");
  });

  it("offers the rule's own reading first and the others behind it", () => {
    const current = defaultOutputFor("boolean");
    expect(candidateOutputs(current)[0]).toBe(current);
    expect(candidateOutputs(current).map((output) => output.kind)).toEqual([
      "condition",
      "category",
      "scale",
    ]);
  });
});

describe("stylePresets", () => {
  it("offers a heatmap and the aggregate rules for a number, whatever it is dressed as", () => {
    for (const dataType of ["integer", "number", "currency", "percentage"] as const) {
      const presets = stylePresets({ name: "Amount", dataType });
      expect(presets.map((preset) => preset.id)).toEqual([
        "heatmap",
        "diverging",
        "above-average",
        "top-tenth",
        "bottom-tenth",
        "negative",
        "blanks",
      ]);
      // Three colors and nothing else: where a row sits between them is the
      // formula's answer, which is why the ramp carries no numbers.
      expect(presets[0].output).toEqual({
        kind: "scale",
        scale: {
          text: null,
          fill: { low: "#ffffff", high: "#8da293", mid: null },
        },
      });
      // A ramp paints a position between 0 and 1, and the formula is what
      // says where each row sits -- so the preset writes the mapping rather
      // than leaving numbers on the rule for a control to edit.
      expect(presets[0].formula).toBe("`Amount`.normalize()");
      // The ramp two colors cannot say: zero lands at the turn because the
      // formula puts it there, not because the rule carries the number.
      expect(presets[1].formula).toBe("`Amount`.normalize(center=0)");
      expect(
        presets[1].output.kind === "scale" && presets[1].output.scale.fill?.mid
      ).toBeTruthy();
      expect(presets[2].formula).toBe("`Amount` > `Amount`.mean()");
    }
  });

  it("offers text the reading text has, and never a ramp", () => {
    for (const dataType of ["string", "categorical"] as const) {
      const presets = stylePresets({ name: "Status", dataType });
      expect(presets.map((preset) => preset.id)).toEqual(["by-value", "blanks"]);
      // Empty on purpose -- the values are filled in from the data by
      // `categoryOutput` at the moment the rule is made.
      expect(presets[0].output).toEqual({
        kind: "category",
        cases: [],
        other: { ...emptyStyle(), fillColor: "#fff0c7" },
      });
      expect(presets[0].output.kind).toBe("category");
      expect(presets.some((preset) => preset.output.kind === "scale")).toBe(false);
    }
  });

  it("falls back to the one rule any column can take", () => {
    expect(stylePresets({ name: "Signed", dataType: "date" }).map((p) => p.id)).toEqual([
      "weekends",
      "future",
      "stale",
      "blanks",
    ]);
    expect(stylePresets({ name: "Paid", dataType: "boolean" }).map((p) => p.id)).toEqual([
      "true",
      "false",
      "blanks",
    ]);
  });

  it("writes column names the way a formula names them", () => {
    expect(quoteColumn("Amount")).toBe("`Amount`");
    // A backtick in a name is doubled, not dropped: the formula has to be
    // parseable, and the column is still called what it is called.
    expect(quoteColumn("Net `Revenue`")).toBe("`Net ``Revenue```");
    expect(stylePresets({ name: "a`b", dataType: "number" })[2].formula).toBe(
      "`a``b` > `a``b`.mean()"
    );
  });
});

describe("categoryOutput", () => {
  const fills = (output: FrameStyleOutput) =>
    output.kind === "category"
      ? output.cases.map((entry) => [entry.value, entry.style.fillColor])
      : [];

  it("hands every value the engine found a color of its own", () => {
    const output = categoryOutput(["Line", "Section", "Total"]);
    expect(fills(output)).toEqual([
      ["Line", CATEGORY_FILLS[0]],
      ["Section", CATEGORY_FILLS[1]],
      ["Total", CATEGORY_FILLS[2]],
    ]);
    // And the catch-all stays even though the list covers every value that
    // was there when it was filled: the data moves -- a live CSV refreshes,
    // a filter opens up, somebody types a section that did not exist -- and
    // those rows want a color saying "not one of the named ones" rather
    // than no color, which reads as a rule that stopped working.
    expect(output.kind === "category" && output.other?.fillColor).toBe("#fff0c7");
  });

  it("keeps what is already styled and only dresses the newcomers", () => {
    const first = categoryOutput(["Line", "Section"]);
    const edited: FrameStyleOutput = {
      kind: "category",
      cases: [
        { value: "Line", style: { ...emptyStyle(), fillColor: "#9a452b", bold: true } },
        ...(first.kind === "category" ? first.cases.slice(1) : []),
      ],
      other: null,
    };
    const refilled = categoryOutput(["Line", "Section", "Total"], edited as never);
    expect(fills(refilled)[0]).toEqual(["Line", "#9a452b"]);
    expect(refilled.kind === "category" && refilled.cases[0].style.bold).toBe(true);
    expect(fills(refilled)[1]).toEqual(["Section", CATEGORY_FILLS[1]]);
    // The newcomer takes a color nothing on screen is already wearing --
    // not the first of the palette, which "Section" already has.
    expect(fills(refilled)[2]).toEqual(["Total", CATEGORY_FILLS[0]]);
  });

  it("drops values the formula no longer answers", () => {
    const before = categoryOutput(["Line", "Section"]);
    const after = categoryOutput(["Line"], before as never);
    expect(fills(after)).toEqual([["Line", CATEGORY_FILLS[0]]]);
  });

  it("colors as many values as it has colors, and leaves the rest to the catch-all", () => {
    const many = Array.from({ length: CATEGORY_FILLS.length + 4 }, (_, index) => `v${index}`);
    const output = categoryOutput(many);
    expect(output.kind === "category" && output.cases).toHaveLength(CATEGORY_FILLS.length);
    expect(output.kind === "category" && output.other?.fillColor).toBe("#fff0c7");
  });

  it("leaves an existing rule's own catch-all alone, including none", () => {
    const chosen: FrameStyleOutput = {
      kind: "category",
      cases: [{ value: "Line", style: { ...emptyStyle(), fillColor: "#dce9df" } }],
      other: null,
    };
    const refilled = categoryOutput(["Line", "Section"], chosen as never);
    expect(refilled.kind === "category" && refilled.other).toBeNull();
  });

  it("keeps a rule paintable when a column turns out to be empty", () => {
    const output = categoryOutput([]);
    expect(output.kind === "category" && output.cases).toEqual([]);
    expect(output.kind === "category" && output.other?.fillColor).toBe("#fff0c7");
  });
});

describe("ruleWithoutStop", () => {
  const categorical = (values: string[]) =>
    rule({ output: categoryOutput(values) });

  it("removes one value and leaves the rest of the mapping alone", () => {
    const next = ruleWithoutStop(categorical(["Line", "Section", "Total"]), {
      kind: "case",
      value: "Section",
    });
    expect(next?.output.kind === "category" && next.output.cases.map((c) => c.value)).toEqual([
      "Line",
      "Total",
    ]);
  });

  it("hands the last value's rule back to the catch-all rather than emptying it", () => {
    // The engine refuses a rule that styles nothing, which is right -- so
    // taking the last value out has to leave something behind.
    const next = ruleWithoutStop(categorical(["Line"]), { kind: "case", value: "Line" });
    expect(next?.output.kind === "category" && next.output.cases).toEqual([]);
    expect(next?.output.kind === "category" && next.output.other?.fillColor).toBe("#fff0c7");
  });

  it("refuses to remove the stops that are what the rule is", () => {
    expect(ruleWithoutStop(rule({}), { kind: "condition" })).toBeNull();
    expect(ruleWithoutStop(categorical(["Line"]), { kind: "other" })).toBeNull();
    expect(ruleWithoutStop(ramp(), { kind: "scale", end: "high" })).toBeNull();
  });
});

describe("a ramp's stops", () => {
  it("holds three colors and no numbers, because the formula holds those", () => {
    const diverging = ruleWithScaleMid(ramp());
    const scale = diverging?.output.kind === "scale" ? diverging.output.scale : null;
    // Low, middle, high -- 0, 0.5 and 1. Where a row lands between them is
    // the formula's answer, so there is nothing else on the rule to set.
    expect(Object.keys(scale ?? {}).sort()).toEqual(["fill", "text"]);
    expect(typeof scale?.fill?.low).toBe("string");
    expect(typeof scale?.fill?.mid).toBe("string");
  });

  it("changes one stop's color and leaves the others alone", () => {
    const edited = ruleWithStopStyle(ramp(), { kind: "scale", end: "high" }, {
      ...emptyStyle(),
      fillColor: "#9a452b",
    });
    const scale = edited.output.kind === "scale" ? edited.output.scale : null;
    expect(scale?.fill?.high).toBe("#9a452b");
    expect(scale?.fill?.low).toBe("#ffffff");
  });

  it("adds and removes a middle for one color property at a time", () => {
    const both = ramp({
      text: { low: "#315cbb", high: "#b43ca8", mid: null },
    });
    const fillMiddle = ruleWithScaleMid(both, "fill");
    expect(
      fillMiddle?.output.kind === "scale" && fillMiddle.output.scale.fill?.mid
    ).toBe("#98aea4");
    expect(
      fillMiddle?.output.kind === "scale" && fillMiddle.output.scale.text?.mid
    ).toBeNull();

    const textMiddle = fillMiddle && ruleWithScaleMid(fillMiddle, "text");
    expect(
      textMiddle?.output.kind === "scale" && textMiddle.output.scale.text?.mid
    ).toBe("#734cb2");
  });
});
