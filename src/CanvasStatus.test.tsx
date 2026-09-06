// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CanvasStatus } from "./CanvasStatus";
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

describe("CanvasStatus", () => {
  it("no longer carries the Quick Commands launcher, which lives on the rail mark", () => {
    const { container } = render(<CanvasStatus {...baseProps} />);
    expect(screen.queryByRole("button", { name: "Quick Commands" })).toBeNull();
    expect(container.querySelector(".canvas-status")).toBeTruthy();
  });
});
