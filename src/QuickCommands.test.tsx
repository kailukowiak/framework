// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  QuickCommands,
  quickCommandItems,
  type QuickCommand,
} from "./QuickCommands";

const commands = (run = vi.fn()): QuickCommand[] => [
  { id: "open", label: "Open…", group: "Document", shortcut: "⌘O", run },
  { id: "scratchwork", label: "Open Scratchwork", group: "Canvas", run: vi.fn() },
  {
    id: "disabled",
    label: "Unavailable action",
    group: "View",
    disabled: true,
    run: vi.fn(),
  },
];

afterEach(cleanup);

describe("QuickCommands", () => {
  it("indexes the workbook Scratchwork window action", () => {
    const run = vi.fn();
    const items = quickCommandItems(
      { "open-scratchwork-window": run },
      { canUndo: false, canRedo: false, hasSelectedView: false }
    );
    const item = items.find((candidate) => candidate.id === "scratchwork-window");
    expect(item?.label).toBe("Open Scratchwork Window");
    item?.run();
    expect(run).toHaveBeenCalledOnce();
  });

  it("filters commands and runs the selected action from the keyboard", () => {
    const run = vi.fn();
    const close = vi.fn();
    render(<QuickCommands commands={commands(run)} onClose={close} />);

    const search = screen.getByLabelText("Search commands");
    fireEvent.change(search, { target: { value: "open" } });
    expect(screen.getByText("Open…")).toBeTruthy();
    expect(screen.queryByText("Unavailable action")).toBeNull();

    fireEvent.keyDown(search, { key: "Enter" });
    expect(close).toHaveBeenCalledOnce();
    expect(run).toHaveBeenCalledOnce();
  });

  it("moves through the visible commands and closes with Escape", () => {
    const close = vi.fn();
    render(<QuickCommands commands={commands()} onClose={close} />);
    const search = screen.getByLabelText("Search commands");

    fireEvent.keyDown(search, { key: "ArrowDown" });
    expect(screen.getByText("Open Scratchwork").closest("button")?.getAttribute("aria-selected"))
      .toBe("true");
    fireEvent.keyDown(search, { key: "Escape" });
    expect(close).toHaveBeenCalledOnce();
  });
});
