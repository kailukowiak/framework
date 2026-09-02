// @vitest-environment jsdom

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useApplicationMenu } from "./useApplicationMenu";

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
});
