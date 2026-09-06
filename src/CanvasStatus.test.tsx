// @vitest-environment jsdom

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CanvasStatus } from "./CanvasStatus";
import { QuickCommands, type QuickCommand } from "./QuickCommands";
import { DEFAULT_CANVAS_ZOOM } from "./lib/canvasZoom";

afterEach(cleanup);

const baseProps = {
  withInspector: false,
  withCollapsedInspector: false,
  document: { scenarios: [], activeScenario: null },
  context: null,
  focus: null,
  documentPath: "/tmp/example.fw",
  staleCount: 0,
  refreshing: false,
  zoom: DEFAULT_CANVAS_ZOOM,
  onOperation: vi.fn(),
  onSave: vi.fn(),
  onRefresh: vi.fn(),
  onZoom: vi.fn(),
};

describe("CanvasStatus — Quick Commands launcher", () => {
  it("renders a button with the Quick Commands accessible name and shortcut hint", () => {
    render(<CanvasStatus {...baseProps} onOpenQuickCommands={vi.fn()} />);
    const button = screen.getByRole("button", { name: "Quick Commands" });
    expect(button.title).toContain("⇧⌘P");
    expect(button.getAttribute("data-shortcut")).toBe("⇧⌘P");
  });

  it("opens the Quick Commands palette when clicked", () => {
    const commands: QuickCommand[] = [
      { id: "open", label: "Open…", group: "Document", run: vi.fn() },
    ];

    function Harness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <CanvasStatus {...baseProps} onOpenQuickCommands={() => setOpen(true)} />
          {open && (
            <QuickCommands commands={commands} onClose={() => setOpen(false)} />
          )}
        </>
      );
    }

    render(<Harness />);
    expect(screen.queryByLabelText("Search commands")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Quick Commands" }));

    expect(screen.getByLabelText("Search commands")).toBeTruthy();
  });
});
