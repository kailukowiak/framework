// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MODIFIER_HINTS_ATTRIBUTE, useModifierHints } from "./useModifierHints";

// The claim under test: hints are up only while a bare shortcut modifier is
// held. Every way of stopping holding it — including the ways macOS never
// tells us about — puts them down again.

const hinting = () => document.documentElement.hasAttribute(MODIFIER_HINTS_ATTRIBUTE);

const press = (key: string, options: KeyboardEventInit = {}) =>
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key, ...options }));
  });

const release = (key: string) =>
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keyup", { key }));
  });

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute(MODIFIER_HINTS_ATTRIBUTE);
});

describe("useModifierHints", () => {
  it("raises hints while Meta is held and drops them when it is released", () => {
    renderHook(() => useModifierHints());
    expect(hinting()).toBe(false);
    press("Meta");
    expect(hinting()).toBe(true);
    release("Meta");
    expect(hinting()).toBe(false);
  });

  it("treats Control the same way, because the shortcut map does", () => {
    renderHook(() => useModifierHints());
    press("Control");
    expect(hinting()).toBe(true);
    release("Control");
    expect(hinting()).toBe(false);
  });

  it("drops hints as soon as a shortcut is chosen, so ⌘Z leaves nothing stuck", () => {
    renderHook(() => useModifierHints());
    press("Meta");
    press("z", { metaKey: true });
    expect(hinting()).toBe(false);
    // The Meta keyup that macOS may never deliver is not what cleared them.
    release("z");
    expect(hinting()).toBe(false);
  });

  it("drops hints when the window loses focus", () => {
    renderHook(() => useModifierHints());
    press("Meta");
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expect(hinting()).toBe(false);
  });

  it("drops hints when the window is hidden", () => {
    renderHook(() => useModifierHints());
    press("Meta");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(hinting()).toBe(false);
  });

  it("leaves nothing behind when it unmounts", () => {
    const { unmount } = renderHook(() => useModifierHints());
    press("Meta");
    unmount();
    expect(hinting()).toBe(false);
    press("Meta");
    expect(hinting()).toBe(false);
  });
});
