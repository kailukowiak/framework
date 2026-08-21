import { describe, expect, it } from "vitest";
import { carryCaret } from "./carryCaret";

/** The caret's offset, written with `|` where it is. */
function at(text: string): [string, number] {
  return [text.replace("|", ""), text.indexOf("|")];
}

/** Carrying `before`'s caret into `after`, both written with a `|`. */
function carried(before: string, after: string): string {
  const [from, mark] = at(before);
  const to = at(after)[0];
  const landed = carryCaret(from, to, mark);
  return `${to.slice(0, landed)}|${to.slice(landed)}`;
}

describe("carryCaret", () => {
  it("leaves a caret alone when nothing came back changed", () => {
    expect(carried("x = 1\ny| = 2", "x = 1\ny = 2")).toBe("x = 1\ny| = 2");
  });

  // The report that started this: type `10` onto the end of `revenue` and the
  // document rewrites the two lines below to match. Both of them are under
  // the caret, so the caret must not move at all.
  it("stays put when the lines rewritten are below it", () => {
    expect(
      carried(
        "revenue10| = 250000\nshare = margin / revenue\ntax = revenue * rate",
        "revenue10 = 250000\nshare = margin / revenue10\ntax = revenue10 * rate"
      )
    ).toBe("revenue10| = 250000\nshare = margin / revenue10\ntax = revenue10 * rate");
  });

  // The one an offset gets wrong: everything above grew, so the same number
  // of characters in is now two characters short of where the author was.
  it("moves with the text when the rewrite is above it", () => {
    expect(carried("share = margin / revenue\ntax = rate|", "share = margin / revenue10\ntax = rate")).toBe(
      "share = margin / revenue10\ntax = rate|"
    );
  });

  it("moves with the text when the rewrite above it shrank", () => {
    expect(carried("gross = 1\nnet = gross|", "g = 1\nnet = g")).toBe("g = 1\nnet = g|");
  });

  it("holds the near edge of a change it was inside", () => {
    const landed = carryCaret("abcdef", "axyzf", 3);
    expect(landed).toBeGreaterThanOrEqual(1);
    expect(landed).toBeLessThanOrEqual(4);
  });

  it("survives text arriving empty", () => {
    expect(carryCaret("x = 1", "", 4)).toBe(0);
  });

  it("survives text arriving where there was none", () => {
    expect(carryCaret("", "x = 1", 0)).toBe(0);
  });

  // A rewrite either side of the caret leaves it in the middle region, which
  // cannot be placed exactly. It still has to land somewhere real.
  it("always lands inside the text it is given", () => {
    const before = "a = 1\nb = a\nc = a";
    const after = "aa = 1\nb = aa\nc = aa";
    for (let mark = 0; mark <= before.length; mark += 1) {
      const landed = carryCaret(before, after, mark);
      expect(landed).toBeGreaterThanOrEqual(0);
      expect(landed).toBeLessThanOrEqual(after.length);
    }
  });
});
