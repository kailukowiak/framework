// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

const tauriInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => tauriInvoke(...args),
}));

async function loadModule() {
  return import("./errorReporting");
}

describe("errorReporting", () => {
  afterEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
    vi.restoreAllMocks();
  });

  it("reportError logs to the console and forwards an error-level event", async () => {
    const { reportError } = await loadModule();
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    reportError("something broke", "some context");
    await Promise.resolve();
    await Promise.resolve();

    expect(consoleError).toHaveBeenCalled();
    expect(tauriInvoke).toHaveBeenCalledWith("log_frontend_event", {
      level: "error",
      message: "something broke",
      context: "some context",
    });
  });

  it("reportWarning logs to the console and forwards a warn-level event", async () => {
    const { reportWarning } = await loadModule();
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    reportWarning("heads up");
    await Promise.resolve();
    await Promise.resolve();

    expect(consoleWarn).toHaveBeenCalled();
    expect(tauriInvoke).toHaveBeenCalledWith("log_frontend_event", {
      level: "warn",
      message: "heads up",
      context: undefined,
    });
  });

  it("reportCommandFailure names the command and includes argument keys, never values", async () => {
    const { reportCommandFailure } = await loadModule();
    vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    reportCommandFailure(
      "save_document_as_dialog",
      new Error("disk full"),
      ["path", "password"]
    );
    await Promise.resolve();
    await Promise.resolve();

    const [, payload] = tauriInvoke.mock.calls[0];
    expect(payload.message).toBe(
      "command failed: save_document_as_dialog: disk full"
    );
    expect(payload.context).toBe("args: path, password");
    expect(payload.context).not.toContain("hunter2");
  });

  it("reportIgnoredFailure is a curried warn-level reporter usable directly in .catch", async () => {
    const { reportIgnoredFailure } = await loadModule();
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    await Promise.reject(new Error("nope")).catch(
      reportIgnoredFailure("history menu state")
    );
    await Promise.resolve();

    expect(consoleWarn).toHaveBeenCalled();
    expect(tauriInvoke).toHaveBeenCalledWith("log_frontend_event", {
      level: "warn",
      message: "ignored failure: history menu state: nope",
      context: undefined,
    });
  });

  it("swallows a failure of the forwarding call itself instead of recursing", async () => {
    const { reportError } = await loadModule();
    vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockRejectedValue(new Error("log_frontend_event unavailable"));

    expect(() => reportError("something broke")).not.toThrow();
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();

    // Exactly the one attempt: the rejected forward is swallowed rather than
    // reported through `reportError` again, which would otherwise call
    // `invoke` a second time and recurse forever.
    expect(tauriInvoke).toHaveBeenCalledTimes(1);
  });

  it("stays silent when Tauri is entirely absent", async () => {
    const { reportError } = await loadModule();
    vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockRejectedValue(new Error("no such command"));

    expect(() => reportError("still works without a backend")).not.toThrow();
    await Promise.resolve();
    await Promise.resolve();
  });

  it("installGlobalErrorReporting forwards an uncaught error", async () => {
    const { installGlobalErrorReporting } = await loadModule();
    vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    installGlobalErrorReporting();
    window.dispatchEvent(
      new ErrorEvent("error", {
        error: new Error("render blew up"),
        message: "render blew up",
        filename: "App.tsx",
        lineno: 12,
        colno: 3,
      })
    );
    await Promise.resolve();
    await Promise.resolve();

    const call = tauriInvoke.mock.calls.find(
      ([, payload]) => payload.message === "render blew up"
    );
    expect(call).toBeTruthy();
  });

  it("installGlobalErrorReporting forwards an unhandled promise rejection", async () => {
    const { installGlobalErrorReporting } = await loadModule();
    vi.spyOn(console, "error").mockImplementation(() => {});
    tauriInvoke.mockResolvedValue(undefined);

    installGlobalErrorReporting();
    const rejection = Object.assign(new Event("unhandledrejection"), {
      reason: new Error("promise blew up"),
      promise: Promise.reject().catch(() => {}),
    });
    window.dispatchEvent(rejection);
    await Promise.resolve();
    await Promise.resolve();

    const call = tauriInvoke.mock.calls.find(
      ([, payload]) => payload.message === "promise blew up"
    );
    expect(call).toBeTruthy();
  });
});
