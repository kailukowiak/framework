// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { DerivedFrameCreator } from "./PipelineEditor";
import type { Column, FrameObject, RenderedFrameStep } from "./lib/types";
import { clearMocks, serveInvoke } from "./test/support";

const columns: Column[] = [
  { id: "region", name: "Region", dataType: "string", formula: null },
  { id: "when", name: "Booked", dataType: "date", formula: null },
];

const frame = {
  id: "orders",
  name: "Orders",
  kind: "frame",
  columns,
  rows: [],
  derivation: { sourceFrameId: "seed", join: null },
  uniqueKeys: [],
  summaries: [],
} as FrameObject;

afterEach(() => {
  cleanup();
  clearMocks();
});

function mount(
  columnId: string,
  renderedSteps: RenderedFrameStep[] = [],
  distinct = vi.fn(() => ({
    total: 2,
    values: [
      { value: "East", count: 4 },
      { value: "West", count: 1 },
    ],
  }))
) {
  serveInvoke({ get_distinct_values: distinct as (args: unknown) => unknown });
  const onOperation = vi.fn().mockResolvedValue(null);
  render(
    <ActiveFormulaEditorProvider>
      <DerivedFrameCreator
        input={{ label: "Seed", columns }}
        editingFrame={frame}
        renderedSteps={renderedSteps}
        passThroughSteps={0}
        references={[]}
        frames={[]}
        filterColumnRequest={{ token: 1, columnId }}
        onFilterColumnRequestHandled={() => {}}
        onOperation={onOperation}
      />
    </ActiveFormulaEditorProvider>
  );
  return { onOperation, distinct };
}

describe("the column header's filter button", () => {
  it("opens the value checklist, and applying saves a trailing is_in filter", async () => {
    const user = userEvent.setup();
    const { onOperation, distinct } = mount("region");
    await user.click(await screen.findByRole("checkbox", { name: /West/ }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => expect(onOperation).toHaveBeenCalled());
    const [operation] = onOperation.mock.calls[0];
    expect(operation).toMatchObject({
      type: "setFramePipeline",
      frameId: "orders",
      steps: [
        {
          kind: "filter",
          predicates: ['`Region`\n  .is_in(["East"])'],
          matchAll: true,
        },
      ],
    });
    // The first open reads the whole chain: there is no picker filter yet.
    expect(distinct).toHaveBeenCalledWith({
      frameId: "orders",
      columnId: "region",
      steps: [],
      limit: 200,
    });
    // Nothing is saved merely by opening it.
    expect(onOperation).toHaveBeenCalledTimes(1);
  });

  it("re-opens on the values from before its own filter, with only the kept ones checked", async () => {
    const { distinct } = mount("region", [
      { kind: "filter", predicates: ['`Region`.is_in(["East"])'], matchAll: true },
    ]);
    const east = (await screen.findByRole("checkbox", {
      name: /East/,
    })) as HTMLInputElement;
    expect(east.checked).toBe(true);
    expect((screen.getByRole("checkbox", { name: /West/ }) as HTMLInputElement).checked).toBe(
      false
    );
    // The engine is asked about the chain without the picker's predicate, or
    // West would be gone from the list and impossible to tick again.
    expect(distinct).toHaveBeenCalledWith(
      expect.objectContaining({ steps: [] })
    );
  });

  it("keeps the typed filter for a date column", async () => {
    const { distinct } = mount("when");
    // A date has no string spelling in a predicate, so the funnel appends the
    // typed Filter draft (as it always did) rather than a checklist.
    expect(await screen.findByText("Filter rows", { selector: "strong" })).toBeTruthy();
    expect(screen.queryByRole("group", { name: "Filter Booked" })).toBeNull();
    expect(distinct).not.toHaveBeenCalled();
  });

  it("Edit in Wrangle falls back to the typed draft", async () => {
    const user = userEvent.setup();
    mount("region", [
      { kind: "filter", predicates: ['`Region` == "East"'], matchAll: true },
    ]);
    await user.click(await screen.findByRole("button", { name: "Edit in Wrangle" }));
    expect(screen.queryByRole("group", { name: "Filter Region" })).toBeNull();
    expect(screen.getAllByText("Filter rows", { selector: "strong" })).toHaveLength(1);
  });
});
