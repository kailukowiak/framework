// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { DerivedFrameCreator } from "./PipelineEditor";
import type { DocumentView, FrameObject } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

// The wrangle editor drafts over the document's saved chain. The claim
// here is the reconciliation one: when the saved chain changes underneath
// the mounted editor — an undo is the everyday case — the step list must
// re-derive from the document, not keep showing the draft it was seeded
// with. The stale list was not just cosmetic: the next saved gesture wrote
// it back over the document, silently reversing the undo.
function editorFor(view: DocumentView, frame: FrameObject) {
  const computed = view.computedFrames[frame.id]!;
  return (
    <ActiveFormulaEditorProvider>
      <DerivedFrameCreator
        input={{
          label: "Source: this frame’s own data",
          columns: frame.baseColumns?.length ? frame.baseColumns : frame.columns,
        }}
        editingFrame={frame}
        renderedSteps={computed.steps ?? []}
        passThroughSteps={computed.passThroughSteps ?? 0}
        references={[]}
        frames={[]}
        onOperation={vi.fn().mockResolvedValue(null)}
      />
    </ActiveFormulaEditorProvider>
  );
}

describe("PipelineEditor document sync", () => {
  it("drops a step the document no longer holds, as an undo delivers", () => {
    const withDelete = fixtures.salesMarginDeleteRegion;
    const withoutDelete = fixtures.salesWithMargin;
    const frameWithDelete = objectNamed(withDelete, "frame", "Monthly sales");
    const frameWithout = objectNamed(withoutDelete, "frame", "Monthly sales");

    // "Delete columns" is also an entry in the add-a-step picker; the claim
    // is about the drawn step rows, so the picker's option is not counted.
    const deleteStepRows = () =>
      screen
        .queryAllByText("Delete columns")
        .filter((element) => element.tagName !== "OPTION");

    const { rerender } = render(editorFor(withDelete, frameWithDelete));
    expect(deleteStepRows()).toHaveLength(1);

    // The undo lands: a new DocumentView whose chain no longer carries the
    // Select step. Same frame id — the fixtures come from one store.
    rerender(editorFor(withoutDelete, frameWithout));
    expect(deleteStepRows()).toHaveLength(0);
    // The surviving calculation is still drawn from the fresh chain.
    expect(screen.getAllByText(/Margin/).length).toBeGreaterThan(0);
  });
});
