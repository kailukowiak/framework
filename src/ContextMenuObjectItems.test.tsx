// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ContextMenuScenarioItems } from "./ContextMenuObjectItems";
import type {
  BlockLine,
  BlockObject,
  DocumentView,
  Operation,
  Scenario,
} from "./lib/types";

afterEach(cleanup);

// No generated fixture carries scenarios, so the document is written out
// here. It is a literal, not a double that computes: `apply` replays the two
// answers it is handed and nothing works anything out.

function line(name: string, source: string): BlockLine {
  return {
    id: `${name}-line`,
    name,
    named: true,
    nameQuoted: false,
    source,
    formula: null,
    error: null,
  };
}

const block: BlockObject = {
  kind: "block",
  id: "block-1",
  name: "Model",
  lines: [
    line("revenue", "`Sales`.`Amount`.sum()"),
    line("gross", "revenue * 0.4"),
    // A line nobody named: addressable, but not a row somebody would
    // recognise, so the comparison leaves it out.
    {
      ...line("line_3", "revenue - gross"),
      named: false,
    },
  ],
};

function document(scenarios: Scenario[]): DocumentView {
  return {
    id: "document-1",
    name: "Test",
    revision: 1,
    canUndo: false,
    canRedo: false,
    safeMode: false,
    formulaFunctions: [],
    computedResults: {},
    computedModels: {},
    computedBlocks: {},
    computedTexts: {},
    computedValues: {},
    computedCalculationMatrices: {},
    computedFrames: {},
    calendars: [],
    scenarios,
    objects: [block],
    views: [
      {
        id: "view-1",
        objectId: "block-1",
        x: 40,
        y: 60,
        width: 600,
        height: 220,
        collapsed: false,
      },
    ],
  };
}

const upside: Scenario = { id: "upside", name: "Upside", values: {} };

/** The comparison frame as the first operation's answer describes it. */
function withComparisonFrame(initial: DocumentView): DocumentView {
  return {
    ...initial,
    objects: [
      ...initial.objects,
      {
        kind: "frame",
        id: "frame-1",
        name: "Model by scenario",
        columns: [
          { id: "output", name: "Output", dataType: "string", formula: null },
        ],
        rows: [],
        derivation: null,
        uniqueKeys: [],
        summaries: [],
        display: { orientation: "recordsAsRows" },
      },
    ],
  };
}

function mount(view: DocumentView) {
  const apply = vi
    .fn<(operation: Operation) => Promise<DocumentView>>()
    .mockResolvedValue(withComparisonFrame(view));
  render(
    <ContextMenuScenarioItems
      document={view}
      contextObject={block}
      apply={apply}
      setContextMenu={vi.fn()}
      setError={vi.fn()}
    />
  );
  return apply;
}

/** The chain formatter's line breaks, undone, so the text can be read. */
const oneLine = (formula: string) => formula.replace(/\s*\n\s*/g, "");

it("builds the outputs table and one calculated column per scenario", async () => {
  const apply = mount(document([upside]));
  fireEvent.click(screen.getByText("Compare scenarios"));
  await waitFor(() => expect(apply).toHaveBeenCalledTimes(2));

  expect(apply.mock.calls[0][0]).toMatchObject({
    type: "addFrame",
    name: "Model by scenario",
    grid: [["Output"], ["revenue"], ["gross"]],
  });

  const pipeline = apply.mock.calls[1][0];
  if (pipeline.type !== "setFramePipeline")
    throw new Error("the second operation should set the chain");
  expect(pipeline.frameId).toBe("frame-1");
  expect(pipeline.steps).toHaveLength(1);
  const step = pipeline.steps[0];
  if (step.kind !== "withColumns")
    throw new Error("the chain should be one withColumns step");
  expect(step.columns.map((column) => column.name)).toEqual([
    "Base",
    "Upside",
  ]);
  expect(oneLine(step.columns[0].formula)).toBe(
    'when(`Output` == "revenue").then(`Model`.`revenue`).otherwise(`Model`.`gross`)'
  );
  expect(oneLine(step.columns[1].formula)).toBe(
    'when(`Output` == "revenue").then(under(`Upside`, `Model`.`revenue`))' +
      ".otherwise(under(`Upside`, `Model`.`gross`))"
  );
});

it("is not offered while the document has no scenario to compare against", () => {
  mount(document([]));
  expect(screen.queryByText("Compare scenarios")).toBeNull();
});
