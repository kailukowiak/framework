// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { UpdateDialog } from "./UpdateDialog";

function noop() {}

describe("UpdateDialog", () => {
  afterEach(() => {
    cleanup();
  });

  it("names the version on offer and distinguishes Skip from Later", () => {
    const skip = vi.fn();
    const dismiss = vi.fn();
    render(
      <UpdateDialog
        status={{ kind: "available", version: "0.1.2", notes: null }}
        progress={null}
        onInstall={noop}
        onSkip={skip}
        onDismiss={dismiss}
      />
    );

    expect(screen.getByText("FrameWork 0.1.2")).toBeTruthy();
    fireEvent.click(screen.getByText("Later"));
    expect(dismiss).toHaveBeenCalledTimes(1);
    expect(skip).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText("Skip this version"));
    expect(skip).toHaveBeenCalledTimes(1);
  });

  it("shows release notes when the release carries them", () => {
    render(
      <UpdateDialog
        status={{ kind: "available", version: "0.1.2", notes: "Fixes the grid." }}
        progress={null}
        onInstall={noop}
        onSkip={noop}
        onDismiss={noop}
      />
    );
    expect(screen.getByText("Fixes the grid.")).toBeTruthy();
  });

  it("renders release notes as markdown rather than preformatted text", () => {
    render(
      <UpdateDialog
        status={{
          kind: "available",
          version: "0.1.2",
          notes: "## Download\n\n| System | File |\n| --- | --- |\n| macOS | `.dmg` |",
        }}
        progress={null}
        onInstall={noop}
        onSkip={noop}
        onDismiss={noop}
      />
    );
    expect(screen.getByRole("heading", { name: "Download" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "System" })).toBeTruthy();
    expect(screen.getByRole("cell", { name: "macOS" })).toBeTruthy();
  });

  it("reports download progress against the total once it is known", () => {
    render(
      <UpdateDialog
        status={{ kind: "installing", version: "0.1.2", notes: null }}
        progress={{ received: 2_097_152, total: 37_748_736 }}
        onInstall={noop}
        onSkip={noop}
        onDismiss={noop}
      />
    );
    expect(screen.getByText("Downloading — 2.0 MB of 36.0 MB")).toBeTruthy();
  });

  // While installing, dismissing would leave a half-replaced application, so
  // every route out of the dialog is closed rather than merely discouraged.
  it("cannot be dismissed while installing", () => {
    const dismiss = vi.fn();
    render(
      <UpdateDialog
        status={{ kind: "installing", version: "0.1.2", notes: null }}
        progress={{ received: 0, total: null }}
        onInstall={noop}
        onSkip={noop}
        onDismiss={dismiss}
      />
    );
    expect(screen.queryByLabelText("Close")).toBeNull();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(dismiss).not.toHaveBeenCalled();
  });

  it("points package-manager installs at their package manager", () => {
    render(
      <UpdateDialog
        status={{ kind: "unsupported" }}
        progress={null}
        onInstall={noop}
        onSkip={noop}
        onDismiss={noop}
      />
    );
    expect(screen.getByText("apt")).toBeTruthy();
    expect(screen.getByText("dnf")).toBeTruthy();
    expect(screen.queryByText("Install and restart")).toBeNull();
  });

  it("says so plainly when there is nothing to install", () => {
    render(
      <UpdateDialog
        status={{ kind: "up-to-date" }}
        progress={null}
        onInstall={noop}
        onSkip={noop}
        onDismiss={noop}
      />
    );
    expect(screen.getByText("FrameWork is up to date.")).toBeTruthy();
  });
});
