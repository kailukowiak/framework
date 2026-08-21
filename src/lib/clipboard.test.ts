import { afterEach, describe, expect, it, vi } from "vitest";
import { writeClipboardText } from "./clipboard";

/** A document stand-in that records what the fallback path did to it. */
function fakeScope(execCommand: () => boolean) {
  const appended: HTMLTextAreaElement[] = [];
  const body = {
    append: (node: HTMLTextAreaElement) => appended.push(node),
  } as unknown as HTMLElement;
  return {
    appended,
    scope: {
      createElement: () => {
        const node = {
          value: "",
          style: {} as CSSStyleDeclaration,
          setAttribute: () => {},
          select: () => {},
          setSelectionRange: () => {},
          remove: () => {},
        };
        return node as unknown as HTMLTextAreaElement;
      },
      body,
      execCommand,
    },
  };
}

function setClipboard(writeText: ((text: string) => Promise<void>) | undefined) {
  Object.defineProperty(globalThis, "navigator", {
    value: writeText ? { clipboard: { writeText } } : {},
    configurable: true,
    writable: true,
  });
}

afterEach(() => setClipboard(undefined));

describe("writeClipboardText", () => {
  it("uses the async clipboard when the webview has one", async () => {
    const writeText = vi.fn(async () => {});
    setClipboard(writeText);
    const { scope, appended } = fakeScope(() => true);
    expect(await writeClipboardText("hello", scope)).toBe(true);
    expect(writeText).toHaveBeenCalledWith("hello");
    // Nothing was appended, so the fallback never ran.
    expect(appended).toHaveLength(0);
  });

  it("falls back to a selection when the async clipboard rejects", async () => {
    setClipboard(async () => {
      throw new Error("NotAllowedError");
    });
    const { scope, appended } = fakeScope(() => true);
    expect(await writeClipboardText("hello", scope)).toBe(true);
    expect(appended).toHaveLength(1);
    expect(appended[0].value).toBe("hello");
  });

  it("falls back when the webview has no clipboard object at all", async () => {
    setClipboard(undefined);
    const { scope, appended } = fakeScope(() => true);
    expect(await writeClipboardText("hello", scope)).toBe(true);
    expect(appended).toHaveLength(1);
  });

  it("reports failure rather than claiming a copy that did not happen", async () => {
    setClipboard(undefined);
    const { scope } = fakeScope(() => false);
    expect(await writeClipboardText("hello", scope)).toBe(false);
  });

  it("reports failure when the fallback itself throws", async () => {
    setClipboard(undefined);
    const { scope } = fakeScope(() => {
      throw new Error("unsupported");
    });
    expect(await writeClipboardText("hello", scope)).toBe(false);
  });
});
