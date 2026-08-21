// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { DerivedFrameCreator } from "./PipelineEditor";
import type { Column, FrameObject, RenderedFrameStep } from "./lib/types";

const columns: Column[] = [
  { id: "project", name: "Project", dataType: "string", formula: null },
  { id: "date", name: "Date label", dataType: "string", formula: null },
  { id: "hours", name: "Hours", dataType: "number", formula: null },
];

const frame = {
  id: "timesheet",
  name: "Timesheet POC",
  kind: "frame",
  columns,
  rows: [],
  derivation: { sourceFrameId: "seed", join: null },
  uniqueKeys: [],
  summaries: [],
} as FrameObject;

const steps: RenderedFrameStep[] = [
  {
    kind: "expand",
    frameId: "offsets",
    outputs: [{ outputColumnId: "day-offset", sourceColumnId: "offset-source" }],
  },
  {
    kind: "pivot",
    namesColumnId: "date",
    valuesColumnId: "hours",
    aggregate: "first",
    outputs: [{ outputColumnId: "aug-16", value: "2026-08-16" }],
  },
];

describe("pivot refresh", () => {
  it("re-saves the pipeline so data-dependent columns are discovered again", async () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    render(
      <ActiveFormulaEditorProvider>
        <DerivedFrameCreator
          input={{ label: "Timesheet Seed", columns }}
          editingFrame={frame}
          renderedSteps={steps}
          passThroughSteps={0}
          references={[]}
          frames={[{ id: "offsets", name: "Day offsets" }]}
          onOperation={onOperation}
        />
      </ActiveFormulaEditorProvider>
    );

    fireEvent.click(screen.getByRole("button", { name: "Refresh generated columns" }));

    await waitFor(() =>
      expect(onOperation).toHaveBeenCalledWith(
        {
          type: "setFramePipeline",
          frameId: "timesheet",
          steps: [
            {
              kind: "expand",
              frameId: "offsets",
            },
            {
              kind: "pivot",
              namesColumnId: "date",
              valuesColumnId: "hours",
              aggregate: "first",
            },
          ],
        },
        { inlineError: true }
      )
    );
  });
});
