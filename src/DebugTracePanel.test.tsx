// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { DebugTracePanel } from "./DebugTracePanel";
import { clearMocks, serveInvoke } from "./test/support";

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("DebugTracePanel", () => {
  it("opens a dependency trace and the query plan behind its frame", async () => {
    serveInvoke({
      dependency_graph: () => ({
        objectId: "note",
        name: "Narrative",
        kind: "other",
        formula: null,
        display: null,
        error: null,
        children: [
          {
            objectId: "sales",
            name: "Sales",
            kind: "frame",
            formula: null,
            display: null,
            error: null,
            children: [],
          },
        ],
      }),
      get_frame_query_plan: () => ({
        logical: "LOGICAL PLAN",
        optimized: "OPTIMIZED PLAN",
      }),
    });
    const user = userEvent.setup();
    render(<DebugTracePanel objectId="note" />);

    await user.click(screen.getByText("Trace dependencies"));
    expect(await screen.findByText("Sales")).not.toBeNull();

    await user.click(screen.getByText("Query plan"));
    expect(await screen.findByText("OPTIMIZED PLAN")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "As written" }));
    expect(screen.getByText("LOGICAL PLAN")).not.toBeNull();
  });
});
