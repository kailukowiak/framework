import { describe, expect, it } from "vitest";
import { applicationShortcut } from "./applicationShortcuts";

const shortcut = (key: string, init: KeyboardEventInit = {}) =>
  applicationShortcut({
    key,
    code: init.altKey ? `Key${key.toUpperCase()}` : "",
    metaKey: true,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...init,
  } as KeyboardEvent);

describe("applicationShortcut", () => {
  it("maps inspector tabs and insertion without collisions", () => {
    expect(shortcut("1")).toBe("inspector-selection");
    expect(shortcut("2")).toBe("inspector-format");
    expect(shortcut("3")).toBe("inspector-wrangle");
    expect(shortcut("b", { altKey: true })).toBe("add-block");
    expect(shortcut("f", { altKey: true })).toBe("add-frame");
    expect(shortcut("f", { shiftKey: true })).toBe("fit");
  });

  it("keeps plain save distinct from save as", () => {
    expect(shortcut("s")).toBe("save");
    expect(shortcut("s", { shiftKey: true })).toBe("save-as");
  });

  it("keeps a new document distinct from a new window", () => {
    expect(shortcut("n")).toBe("new");
    expect(shortcut("n", { shiftKey: true })).toBe("new-window");
  });
});
