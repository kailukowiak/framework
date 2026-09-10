// @vitest-environment jsdom

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { runFocusedTextHistory, useApplicationMenu } from "./useApplicationMenu";

const windowListen = vi.fn();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ listen: windowListen }),
}));

function MenuListener({ onOpen }: { onOpen: () => void }) {
  useApplicationMenu(true, { "open-document": onOpen }, vi.fn());
  return null;
}

describe("useApplicationMenu", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    Reflect.deleteProperty(document, "execCommand");
  });

  it("listens on this workbook window rather than across every webview", async () => {
    const unlisten = vi.fn();
    windowListen.mockResolvedValue(unlisten);
    const onOpen = vi.fn();

    const mounted = render(<MenuListener onOpen={onOpen} />);

    await waitFor(() => expect(windowListen).toHaveBeenCalledOnce());
    const [eventName, receive] = windowListen.mock.calls[0];
    expect(eventName).toBe("framework-menu-command");

    receive({ payload: "open-document" });
    expect(onOpen).toHaveBeenCalledOnce();

    mounted.unmount();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("gives undo to the focused formula draft instead of workbook history", async () => {
    windowListen.mockResolvedValue(vi.fn());
    const onUndo = vi.fn();

    function FormulaListener() {
      useApplicationMenu(true, { undo: onUndo }, vi.fn());
      return <textarea aria-label="Formula" autoFocus defaultValue="amount * 1.1" />;
    }

    render(<FormulaListener />);
    await waitFor(() => expect(windowListen).toHaveBeenCalledOnce());
    const execCommand = vi.fn(() => true);
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      value: execCommand,
    });

    windowListen.mock.calls[0][1]({ payload: "undo" });

    expect(execCommand).toHaveBeenCalledWith("undo");
    expect(onUndo).not.toHaveBeenCalled();
  });

  it("leaves undo as workbook history when no text editor owns focus", () => {
    const execCommand = vi.fn();
    expect(
      runFocusedTextHistory("undo", {
        activeElement: document.body,
        execCommand,
      })
    ).toBe(false);
    expect(execCommand).not.toHaveBeenCalled();
  });
});
