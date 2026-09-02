import { describe, expect, it, vi } from "vitest";
import type { DocumentView, FrameObject, Operation } from "./types";
import { combineVectorWithFrame } from "./vectorCombine";

function document(overrides: Partial<DocumentView> = {}): DocumentView {
  return {
    id: "document-1",
    name: "Test",
    revision: 1,
    canUndo: false,
    canRedo: false,
    safeMode: false,
    formulaFunctions: [],
    computedResults: {},
    computedBlocks: {},
    computedTexts: {},
    computedValues: {},
    scenarios: [],
    objects: [
      frame("frame-1", "Rows", "Row"),
    ],
    computedFrames: { "frame-1": {} as DocumentView["computedFrames"][string] },
    views: [
      {
        id: "view-1",
        objectId: "frame-1",
        x: 20,
        y: 30,
        width: 240,
        height: 180,
        collapsed: false,
      },
    ],
    ...overrides,
    computedCalculationMatrices: overrides.computedCalculationMatrices ?? {},
  };
}

function frame(id: string, name: string, columnName: string): FrameObject & {
  kind: "frame";
} {
  return {
    kind: "frame",
    id,
    name,
    columns: [
      {
        id: `${id}-column`,
        name: columnName,
        dataType: "integer",
        formula: null,
      },
    ],
    rows: [],
    derivation: null,
    uniqueKeys: [],
    summaries: [],
    display: { orientation: "recordsAsRows" },
  };
}

const state = {
  frameId: "frame-1",
  viewId: "view-1",
  vector: { name: "Column", formula: "[1, 2, 3]", length: 3 },
};

describe("combineVectorWithFrame", () => {
  it("builds VStack as a visible row source followed by Union", async () => {
    const initial = document();
    const withHelper = document({
      objects: [
        ...initial.objects,
        frame("helper-1", "Column rows", "Row"),
      ],
    });
    const apply = vi
      .fn<(operation: Operation) => Promise<DocumentView>>()
      .mockResolvedValueOnce(withHelper)
      .mockResolvedValueOnce(withHelper);

    await combineVectorWithFrame(initial, state, apply);

    expect(apply.mock.calls[0][0]).toMatchObject({
      type: "addGeneratorFrame",
      formula: "[1, 2, 3]",
      columnName: "Row",
      x: 296,
      y: 30,
    });
    expect(apply.mock.calls[1][0]).toEqual({
      type: "setFramePipeline",
      frameId: "frame-1",
      steps: [{ kind: "union", frameId: "helper-1" }],
    });
  });

});
