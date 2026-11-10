import { describe, expect, it } from "vitest";
import { formatFormulaChains } from "./formulaFormatting";
import {
  numberText,
  readValueFilterPredicate,
  valueFilterPredicate,
  writable,
} from "./valueFilter";

const column = { name: "Re`gion" };

describe("value filter spelling", () => {
  it("writes the engine's canonical text and reads it back", () => {
    for (const values of [
      ["a", "b \"q\", c"],
      [1, 2.5, true],
      [null],
      ["x", null],
      [],
    ]) {
      const text = valueFilterPredicate(column, values);
      expect(readValueFilterPredicate(text, column)).toEqual(values);
    }
    expect(valueFilterPredicate({ name: "A" }, ["x", null])).toBe(
      '`A`.is_in(["x"]) | `A`.is_null()'
    );
  });

  it("does not claim a predicate it would spell differently", () => {
    expect(readValueFilterPredicate('`A`.is_in(["x","y"])', { name: "A" })).toBeUndefined();
    expect(readValueFilterPredicate('`A` == "x"', { name: "A" })).toBeUndefined();
  });
});

describe("value filter spelling matches the engine", () => {
  // The same strings, in the same order, as
  // `the_pickers_predicate_spellings_are_the_engines_own` in
  // crates/framework-core/src/engine/distinct.rs, which proves the engine
  // parses and re-renders each one unchanged. Keep the two lists together.
  const cases: Array<[Parameters<typeof valueFilterPredicate>[1], string]> = [
    [["Accessories", "Home office"], '`Category`.is_in(["Accessories", "Home office"])'],
    [
      ['say "hi"', "back\\slash", "café"],
      '`Category`.is_in(["say \\"hi\\"", "back\\\\slash", "café"])',
    ],
    [[1, -2, 30], "`Category`.is_in([1, -2, 30])"],
    [[1.5, -0.25, 19.99], "`Category`.is_in([1.5, -0.25, 19.99])"],
    [[true, false], "`Category`.is_in([True, False])"],
    [[null], "`Category`.is_null()"],
    [["a", "b", null], '`Category`.is_in(["a", "b"]) | `Category`.is_null()'],
    [[1, 2.5, null], "`Category`.is_in([1, 2.5]) | `Category`.is_null()"],
    [[true, null], "`Category`.is_in([True]) | `Category`.is_null()"],
  ];

  it.each(cases)("writes %j as the engine's own text", (values, expected) => {
    const text = valueFilterPredicate({ name: "Category" }, values);
    expect(text).toBe(expected);
    expect(readValueFilterPredicate(text, { name: "Category" })).toEqual(values);
    // Every saved formula passes through the chain formatter, which breaks
    // the chain over lines; the picker must still recognise its filter.
    const saved = formatFormulaChains(text).source;
    expect(readValueFilterPredicate(saved, { name: "Category" })).toEqual(values);
  });

  it("spells numbers the way the engine renders them", () => {
    expect(numberText(3)).toBe("3");
    expect(numberText(0.1)).toBe("0.1");
    expect(numberText(12.3456)).toBe("12.3456");
    expect(numberText(1e-7)).toBe("0");
  });

  it("refuses values a four-decimal spelling would change", () => {
    expect(writable([1, 2.5, "x", null])).toBe(true);
    expect(writable([0.12345])).toBe(false);
  });
});
