// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";

const reportComponentError = vi.fn();

vi.mock("./lib/errorReporting", () => ({
  reportComponentError: (...args: unknown[]) => reportComponentError(...args),
}));

function Bomb(): never {
  throw new Error("scenario blew up");
}

describe("ErrorBoundary", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders its children when nothing throws", () => {
    render(
      <ErrorBoundary>
        <p>document canvas</p>
      </ErrorBoundary>
    );

    expect(screen.getByText("document canvas")).toBeTruthy();
  });

  it("renders the fallback text and error message when a child throws", () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    );

    expect(
      screen.getByText("FrameWork hit an error it could not recover from.")
    ).toBeTruthy();
    expect(screen.getByText("scenario blew up")).toBeTruthy();
    expect(reportComponentError).toHaveBeenCalledWith(
      expect.any(Error),
      expect.anything()
    );

    consoleError.mockRestore();
  });

  it("shows a Reload button that reloads the page", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const reload = vi.fn();
    const originalLocation = window.location;
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { ...originalLocation, reload },
    });

    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>
    );

    const button = screen.getByRole("button", { name: "Reload" });
    button.click();
    expect(reload).toHaveBeenCalledOnce();

    Object.defineProperty(window, "location", {
      configurable: true,
      value: originalLocation,
    });
  });
});
