import { describe, expect, it } from "vitest";
import {
  argumentAt,
  shouldOfferFormulaSuggestions,
} from "./FormulaCompletion";

describe("argumentAt", () => {
  it("finds the active parameter in an ordinary signature", () => {
    expect(argumentAt("clip(min, max)", 1)).toBe("max");
  });

  it("does not split a list-shaped parameter at its inner commas", () => {
    expect(argumentAt("concat_str([expressions, ...], separator)", 1)).toBe(
      "separator"
    );
  });

  it("returns no parameter beyond the signature", () => {
    expect(argumentAt("round(decimals)", 2)).toBeNull();
  });
});

describe("completion visibility", () => {
  it("waits for three implicit characters", () => {
    expect(shouldOfferFormulaSuggestions("lo", 2, "lo", true)).toBe(false);
    expect(shouldOfferFormulaSuggestions("low", 3, "low", true)).toBe(true);
  });

  it("opens immediately for explicit method and identifier requests", () => {
    expect(shouldOfferFormulaSuggestions("`amount`.", 9, ".", true)).toBe(true);
    expect(shouldOfferFormulaSuggestions("`", 1, "`", true)).toBe(true);
  });

  it("does not offer unrelated completions after a finished expression", () => {
    const source = "`name`.str.to_lowercase() ";
    expect(
      shouldOfferFormulaSuggestions(source, source.length, "", true)
    ).toBe(false);
  });

  it("stays out of strings and numeric literals", () => {
    expect(
      shouldOfferFormulaSuggestions('`currency` == "CAD', 18, "CAD", true)
    ).toBe(false);
    expect(shouldOfferFormulaSuggestions("123", 3, "123", true)).toBe(false);
  });

  it("stays dismissed until the draft or cursor changes", () => {
    expect(
      shouldOfferFormulaSuggestions("lower", 5, "lower", true, "lower\u00005")
    ).toBe(false);
    expect(
      shouldOfferFormulaSuggestions("lowerc", 6, "lowerc", true, "lower\u00005")
    ).toBe(true);
  });
});
