// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { readRecentColors, withRecentColor, writeRecentColors } from "./recentColors";

const stored = new Map<string, string>();
const localStorage = {
  get length() {
    return stored.size;
  },
  clear: () => stored.clear(),
  getItem: (key: string) => stored.get(key) ?? null,
  key: (index: number) => [...stored.keys()][index] ?? null,
  removeItem: (key: string) => stored.delete(key),
  setItem: (key: string, value: string) => stored.set(key, value),
};

beforeEach(() => {
  stored.clear();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: localStorage,
  });
});

describe("recent custom colors", () => {
  it("keeps five unique colors, newest first", () => {
    let colors: string[] = [];
    for (const color of ["#111111", "#222222", "#333333", "#444444", "#555555", "#666666"])
      colors = withRecentColor(colors, color);
    expect(colors).toEqual(["#666666", "#555555", "#444444", "#333333", "#222222"]);
    expect(withRecentColor(colors, "#444444")).toEqual([
      "#444444",
      "#666666",
      "#555555",
      "#333333",
      "#222222",
    ]);
  });

  it("scopes text and fill independently to each document", () => {
    writeRecentColors("one", "text", ["#315cbb"]);
    writeRecentColors("one", "fill", ["#c94f45"]);
    writeRecentColors("two", "text", ["#b43ca8"]);
    expect(readRecentColors("one", "text")).toEqual(["#315cbb"]);
    expect(readRecentColors("one", "fill")).toEqual(["#c94f45"]);
    expect(readRecentColors("two", "text")).toEqual(["#b43ca8"]);
  });

  it("ignores malformed stored values and normalizes duplicate hex", () => {
    window.localStorage.setItem(
      "framework.document-colors.one.text",
      JSON.stringify(["#ABCDEF", "nope", "#abcdef", 3])
    );
    expect(readRecentColors("one", "text")).toEqual(["#abcdef"]);
  });
});
