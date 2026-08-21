// The colors somebody can put on a cell, and what those colors do when the
// window is in dark mode.
//
// A stored color is one hex, always the light-mode one. Dark mode is derived
// from it rather than stored beside it, because the alternative is a document
// that carries two answers for every color and a person who has to keep them
// agreeing. The derivation is a reflection: the same hue, the same chroma,
// and a lightness as far from the dark background as the original was from
// the light one. What that preserves is the thing a color was chosen for --
// a highlight stays as loud as it was, a heatmap's low end stays as quiet,
// and text keeps its contrast against the paper it sits on.
//
// The rendering is left to CSS rather than done here: a style comes out as
// `light-dark(stored, reflected)`, so the window follows the system theme at
// the moment it changes, with no re-render and nothing to subscribe to.

import { useEffect, useState } from "react";

/** How far apart the two themes' papers sit, in OKLCH lightness. */
//
// Reading: paper is L 0.985 light and L 0.269 dark, ink is L 0.249 light and
// L 0.942 dark. Those two pairs sum to 1.254 and 1.191, and this is between
// them -- one constant that lands the app's own ink and paper on each other
// within a percent or two, rather than two rules that would need a color to
// declare which one it is.
const REFLECT = 1.2;

/** Lightness bounds, so a reflection cannot come back as pure black or white. */
const [FLOOR, CEILING] = [0.12, 0.96];

/**
 * The eight hues everything else is built from.
 *
 * Eight is about as many as anyone tells apart at a glance, so it is the
 * number of categories a color can distinguish and therefore the number the
 * palette has. Every other row here is these hues at a different lightness,
 * which is what makes fills and text read as one set rather than two.
 */
const FILL_SOFT = [
  "#dce9df",
  "#f6ecc8",
  "#f6ddcd",
  "#f4dade",
  "#ebdcee",
  "#dcdef0",
  "#d6e6f3",
  "#d3e9e9",
] as const;

/** The same eight, deepened -- for a ninth category, and for emphasis. */
const FILL_DEEP = [
  "#bcd4c2",
  "#ecdb9e",
  "#ecc2a8",
  "#e6b6bd",
  "#d5b8db",
  "#b9bde0",
  "#aec9e4",
  "#a9d2d2",
] as const;

/**
 * The fills a category rule hands out, in order, one per value it found.
 *
 * A category rule is a *mapping* — this value looks like this — and the work
 * nobody wants to do is inventing a dozen colors that are tellable apart and
 * still look like they belong to the same document. So the mapping arrives
 * already made and every square of it is editable, which is the difference
 * between a default and a decision.
 *
 * Soft row first, because eight is usually enough and the soft row is the
 * quieter one; past eight the deep row is a better answer than a ninth hue
 * nobody can name.
 */
export const CATEGORY_FILLS = [...FILL_SOFT, ...FILL_DEEP];

/** Every fill on offer: the paper it sits on, then the eight, then deeper. */
export const FILL_SWATCHES = ["#ffffff", ...FILL_SOFT, "#e6e4dc", ...FILL_DEEP];

/**
 * Text colors: the same eight hues at two weights a person can read.
 *
 * Deep row first, at roughly ten to one against the paper, which is what
 * ordinary text wants. The mid row is nearer six to one — still comfortably
 * past the accessibility floor, and the right weight for a value that should
 * be colored without shouting.
 */
export const INK_SWATCHES = [
  "#20221f",
  "#174829",
  "#4a3c00",
  "#5b3111",
  "#5d2b36",
  "#4f2f55",
  "#363863",
  "#104161",
  "#00494a",
  "#636460",
  "#327348",
  "#756213",
  "#8c5329",
  "#904a5a",
  "#7b5084",
  "#5a5c98",
  "#296995",
  "#007376",
];

const HUE_NAMES = ["Sage", "Amber", "Coral", "Rose", "Lilac", "Indigo", "Sky", "Teal"];

/** A quiet human name for every fixed swatch; custom colors keep their hex. */
const PALETTE_NAMES = new Map<string, string>([
  ["#ffffff", "Paper"],
  ["#e6e4dc", "Neutral"],
  ["#20221f", "Ink"],
  ["#636460", "Grey ink"],
  ...FILL_SOFT.map((color, index) => [color, HUE_NAMES[index]] as const),
  ...FILL_DEEP.map(
    (color, index) => [color, `${HUE_NAMES[index]} strong`] as const
  ),
  ...INK_SWATCHES.slice(1, 9).map(
    (color, index) => [color, `${HUE_NAMES[index]} ink`] as const
  ),
  ...INK_SWATCHES.slice(10).map(
    (color, index) => [color, `${HUE_NAMES[index]} mid`] as const
  ),
]);

export function paletteColorName(color: string): string | null {
  return PALETTE_NAMES.get(color.toLowerCase()) ?? null;
}

/**
 * The same color as the other theme would have written it.
 *
 * An involution: reflecting twice returns the lightness it started from, so
 * a color picked in dark mode can be stored as its light-mode twin and come
 * back looking exactly as it was picked. That is what lets one stored hex
 * mean the same thing in both themes without anybody choosing twice.
 */
export function reflectColor(color: string): string {
  const parsed = parseHex(color);
  if (!parsed) return color;
  const [lightness, chroma, hue] = rgbToOklch(parsed);
  return oklchToHex(
    Math.min(CEILING, Math.max(FLOOR, REFLECT - lightness)),
    chroma,
    hue
  );
}

/**
 * A stored color as CSS should paint it: itself in light, its reflection in
 * dark. Left to `light-dark()` so the window follows the system at the
 * moment it changes rather than at the moment React last rendered.
 */
export function themedColor(color: string): string;
export function themedColor(color: string | null | undefined): string | undefined;
export function themedColor(color: string | null | undefined): string | undefined {
  if (!color) return undefined;
  return `light-dark(${color}, ${reflectColor(color)})`;
}

/**
 * What to write down for a color somebody just picked out of a swatch or a
 * native color well.
 *
 * The document stores the light-mode value, always — one canonical answer,
 * and the one the engine mixes ramp ends in. So a pick made while the window
 * is dark is stored reflected, which is what makes "the color I chose" and
 * "the color on the cell" the same color in either theme.
 */
export function storedColor(picked: string, dark: boolean): string {
  return dark ? reflectColor(picked) : picked;
}

function parseHex(color: string): [number, number, number] | null {
  if (!/^#[0-9a-fA-F]{6}$/.test(color)) return null;
  return [1, 3, 5].map((at) => parseInt(color.slice(at, at + 2), 16)) as [
    number,
    number,
    number
  ];
}

// sRGB <-> OKLCH, the published matrices. OKLCH rather than HSL because HSL
// lightness is not a lightness: #ffff00 and #0000ff both sit at 50% of it and
// nobody has ever confused the two. A reflection has to happen somewhere the
// numbers mean what they look like, or a yellow and a blue reflected the same
// amount come back different distances from the paper.
const toLinear = (channel: number) => {
  const value = channel / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
};

const toChannel = (linear: number) => {
  const clamped = Math.min(1, Math.max(0, linear));
  const value =
    clamped <= 0.0031308 ? clamped * 12.92 : 1.055 * clamped ** (1 / 2.4) - 0.055;
  return Math.round(Math.min(1, Math.max(0, value)) * 255);
};

function rgbToOklch([red, green, blue]: [number, number, number]): [
  number,
  number,
  number
] {
  const [r, g, b] = [toLinear(red), toLinear(green), toLinear(blue)];
  const long = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const medium = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const short = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const lightness = 0.2104542553 * long + 0.793617785 * medium - 0.0040720468 * short;
  const a = 1.9779984951 * long - 2.428592205 * medium + 0.4505937099 * short;
  const b2 = 0.0259040371 * long + 0.7827717662 * medium - 0.808675766 * short;
  return [lightness, Math.hypot(a, b2), Math.atan2(b2, a)];
}

function oklchToHex(lightness: number, chroma: number, hue: number): string {
  const a = chroma * Math.cos(hue);
  const b = chroma * Math.sin(hue);
  const long = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const medium = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const short = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const channels = [
    4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short,
    -1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short,
    -0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short,
  ];
  return `#${channels.map((c) => toChannel(c).toString(16).padStart(2, "0")).join("")}`;
}

/**
 * Whether the window is currently dark.
 *
 * Rendering does not need this — a `light-dark()` pair leaves that choice to
 * CSS. What needs it is *writing* a color: a swatch has to store the light
 * value of what somebody is looking at, and a plot has to draw its own SVG
 * with no stylesheet to consult.
 */
export function usePrefersDarkMode(): boolean {
  const query = "(prefers-color-scheme: dark)";
  const [dark, setDark] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const media = window.matchMedia(query);
    const onChange = () => setDark(media.matches);
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);
  return dark;
}
