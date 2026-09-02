// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

const tauriInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => tauriInvoke(...args),
}));

const reportCommandFailure = vi.fn();

vi.mock("./errorReporting", () => ({
  reportCommandFailure: (...args: unknown[]) => reportCommandFailure(...args),
}));

describe("invoke", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("passes a successful call through untouched", async () => {
    const { invoke } = await import("./invoke");
    tauriInvoke.mockResolvedValue({ ok: true });

    const result = await invoke("get_document", { path: "/tmp/x.fw" });

    expect(result).toEqual({ ok: true });
    expect(tauriInvoke).toHaveBeenCalledWith(
      "get_document",
      { path: "/tmp/x.fw" },
      undefined
    );
    expect(reportCommandFailure).not.toHaveBeenCalled();
  });

  it("reports a rejection with the command name and argument key names, then rethrows", async () => {
    const { invoke } = await import("./invoke");
    const failure = new Error("disk is full");
    tauriInvoke.mockRejectedValue(failure);

    await expect(
      invoke("save_document_as_dialog", { path: "/tmp/x.fw", safeMode: false })
    ).rejects.toBe(failure);

    expect(reportCommandFailure).toHaveBeenCalledWith(
      "save_document_as_dialog",
      failure,
      ["path", "safeMode"]
    );
  });

  it("reports a rejection with no argument keys when the call took none", async () => {
    const { invoke } = await import("./invoke");
    const failure = new Error("boom");
    tauriInvoke.mockRejectedValue(failure);

    await expect(invoke("exit_safe_mode")).rejects.toBe(failure);

    expect(reportCommandFailure).toHaveBeenCalledWith(
      "exit_safe_mode",
      failure,
      undefined
    );
  });
});
