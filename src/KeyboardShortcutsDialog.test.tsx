// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { KeyboardShortcutsDialog } from "./KeyboardShortcutsDialog";

describe("KeyboardShortcutsDialog", () => {
  it("lists canvas window commands and closes from the keyboard", () => {
    const close = vi.fn();
    render(<KeyboardShortcutsDialog onClose={close} />);

    expect(screen.getByText("Arrange cards left to right")).toBeTruthy();
    expect(screen.getByText("Fit selected card to window")).toBeTruthy();
    expect(screen.getByText("Collapse or expand selected card")).toBeTruthy();
    expect(screen.getByText("Cycle cards forward or backward")).toBeTruthy();
    expect(screen.getByText("Wrangle")).toBeTruthy();
    expect(screen.getByText("Formula block")).toBeTruthy();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(close).toHaveBeenCalledOnce();
  });
});
