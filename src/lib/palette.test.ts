import { describe, expect, it } from "vitest";
import {
  CATEGORY_FILLS,
  FILL_SWATCHES,
  INK_SWATCHES,
  paletteColorName,
  reflectColor,
  storedColor,
  themedColor,
} from "./palette";

/** WCAG relative luminance, which is what a contrast ratio is made of. */
function luminance(color: string): number {
  const channels = [1, 3, 5].map((at) => {
    const value = parseInt(color.slice(at, at + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(one: string, other: string): number {
  const [high, low] = [luminance(one), luminance(other)].sort((a, b) => b - a);
  return (high + 0.05) / (low + 0.05);
}

// The app's own paper and ink, which is what a styled cell sits on and what
// its text is written in when the style does not say otherwise.
const PAPER = { light: "#fbfaf7", dark: "#262720" };
const INK = { light: "#20221f", dark: "#eeece2" };

describe("reflectColor", () => {
  it("returns what it was given, so a color survives a round trip through dark mode", () => {
    // The property the whole scheme rests on: a color picked while the
    // window is dark is stored as its light twin, and has to come back
    // looking like what was picked.
    //
    // Close rather than identical, and measured as a contrast ratio because
    // that is the scale a person sees on. Two things stop it being exact:
    // the lightness bounds, which hold white off the ceiling and give it
    // back a shade under white, and sRGB, which cannot hold every hue at
    // every lightness -- a pale yellow reflected into a dark one loses a
    // little chroma it cannot get back. Both are invisible at these sizes
    // and neither compounds, because nothing reflects more than once.
    for (const color of [...FILL_SWATCHES, ...INK_SWATCHES]) {
      expect(contrast(color, reflectColor(reflectColor(color)))).toBeLessThan(1.15);
    }
  });

  it("keeps every fill as readable in dark mode as it was in light", () => {
    // The complaint this exists to answer: a fill picked to sit quietly
    // behind dark text becomes a bright block behind light text, and the
    // text stops being readable. A reflection holds the contrast because it
    // holds the distance from the paper.
    for (const fill of CATEGORY_FILLS) {
      const light = contrast(fill, INK.light);
      const dark = contrast(reflectColor(fill), INK.dark);
      expect(light).toBeGreaterThan(7);
      expect(dark).toBeGreaterThan(7);
      // Within a third of each other, so the same rule reads with the same
      // emphasis in either theme rather than shouting in one of them.
      expect(Math.abs(light - dark) / light).toBeLessThan(0.33);
    }
  });

  it("keeps every ink readable against the paper it lands on", () => {
    for (const ink of INK_SWATCHES) {
      expect(contrast(ink, PAPER.light)).toBeGreaterThan(4.5);
      expect(contrast(reflectColor(ink), PAPER.dark)).toBeGreaterThan(4.5);
    }
  });

  it("reflects the app's own ink and paper onto each other", () => {
    // Not a coincidence to preserve for its own sake -- it is the check that
    // the constant is still describing this theme. A restyled application
    // whose paper moved should fail here rather than quietly drift.
    expect(contrast(reflectColor(INK.light), INK.dark)).toBeLessThan(1.2);
    expect(contrast(reflectColor(PAPER.light), PAPER.dark)).toBeLessThan(1.5);
  });

  it("holds a ramp's order, so a heatmap still reads low to high", () => {
    // The engine mixes ramp ends in light-mode hex and hands back the
    // mixture, which lands here as an ordinary color. If reflection were not
    // monotonic the ramp would fold over somewhere in the middle and stop
    // meaning anything.
    //
    // Order rather than distance from the paper: a ramp that starts at white
    // starts *lighter* than the light paper, so its reflection starts darker
    // than the dark paper and crosses it on the way up. That crossing is
    // faithful -- it happens in light mode too -- and it is why an unsigned
    // distance is the wrong thing to watch here.
    const ramp = ["#ffffff", "#e6ece7", "#c9d6cd", "#8da293"];
    const reflected = ramp.map((color) => luminance(reflectColor(color)));
    for (let index = 1; index < reflected.length; index += 1) {
      expect(reflected[index]).toBeGreaterThan(reflected[index - 1]);
    }
  });

  it("leaves anything that is not a plain hex exactly as it found it", () => {
    for (const value of ["", "red", "#abc", "var(--accent)", "#12345g"]) {
      expect(reflectColor(value)).toBe(value);
    }
  });
});

describe("themedColor", () => {
  it("hands CSS both answers and lets it choose", () => {
    expect(themedColor("#fff0c7")).toBe(`light-dark(#fff0c7, ${reflectColor("#fff0c7")})`);
    // Nothing set is nothing written, so a style with no fill does not paint
    // one over whatever it is cascading from.
    expect(themedColor(null)).toBeUndefined();
    expect(themedColor(undefined)).toBeUndefined();
  });
});

describe("storedColor", () => {
  it("writes down the light-mode value whichever theme somebody picked in", () => {
    expect(storedColor("#fff0c7", false)).toBe("#fff0c7");
    expect(storedColor("#2a1e00", true)).toBe(reflectColor("#2a1e00"));
  });
});

describe("the palette itself", () => {
  it("offers colors that are all distinct, and enough of them", () => {
    // Two swatches the same color is one swatch and a bug, and it would show
    // up as a category rule handing two values the same fill.
    expect(new Set(FILL_SWATCHES).size).toBe(FILL_SWATCHES.length);
    expect(new Set(INK_SWATCHES).size).toBe(INK_SWATCHES.length);
    expect(CATEGORY_FILLS.length).toBe(16);
    expect(FILL_SWATCHES.length).toBe(18);
    expect(INK_SWATCHES.length).toBe(18);
  });

  it("writes every swatch as a plain six-digit hex", () => {
    // The engine parses these when it mixes a ramp, and it takes #rrggbb and
    // nothing else. A shorthand here would be a rule the core refuses.
    for (const color of [...FILL_SWATCHES, ...INK_SWATCHES]) {
      expect(color).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  it("gives every fixed swatch a stable human name", () => {
    for (const color of [...FILL_SWATCHES, ...INK_SWATCHES]) {
      expect(paletteColorName(color)).toBeTruthy();
    }
    expect(paletteColorName("#123456")).toBeNull();
  });
});
