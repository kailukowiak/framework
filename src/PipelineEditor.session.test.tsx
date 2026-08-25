// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditorCommands,
} from "./ActiveFormulaEditor";
import { DerivedFrameCreator } from "./PipelineEditor";
import { ScratchworkFormulaBar } from "./ScratchworkFormulaBar";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  DocumentView,
  FrameObject,
  FrameStepInput,
  Operation,
  RenderedFrameStep,
} from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

// The wrangle rows are addresses for the one shared formula editor: clicking
// one moves that editor to the formula bar, and the bar's Return commits it.
// The claims here are about that session's lifetime against the document
// sync. The bug these tests pin down: "Add calculated column" saved an
// unformatted draft, the save's own round trip therefore reseeded the step
// list with fresh row identities, and the session the request had just
// focused was left addressing an editor that no longer existed — the bar
// could edit the draft but Return silently saved nothing.

let commands: ReturnType<typeof useActiveFormulaEditorCommands>;
function Probe() {
  commands = useActiveFormulaEditorCommands();
  return null;
}

function savedPipeline(operation: Operation): FrameStepInput[] {
  if (operation.type !== "setFramePipeline")
    throw new Error(`expected setFramePipeline, got ${operation.type}`);
  return operation.steps;
}

function editorFor(
  view: DocumentView,
  frame: FrameObject,
  onOperation: OperationHandler,
  options: {
    renderedSteps?: RenderedFrameStep[];
    addCalculatedColumnRequest?: { token: number };
  } = {}
) {
  const computed = view.computedFrames[frame.id]!;
  return (
    <ActiveFormulaEditorProvider>
      <Probe />
      {/* The real top bar: the session the wrangle rows open lives in it,
          and its Return is the commit gesture under test. A synthetic
          binding here would prove nothing — the bug lived between the real
          pipeline editor's save and the real bar's commit. */}
      <ScratchworkFormulaBar
        onCommit={vi.fn(async () => ({ saved: true }))}
        references={[]}
        cell={null}
        onCommitCell={vi.fn(async () => null)}
        onEditCalculatedCell={vi.fn()}
        onEditOverrideCell={vi.fn()}
        onRequestReadOnlyCell={vi.fn()}
        expanded={false}
        onToggle={vi.fn()}
      />
      <DerivedFrameCreator
        input={{
          label: "Source: this frame’s own data",
          columns: frame.baseColumns?.length ? frame.baseColumns : frame.columns,
        }}
        editingFrame={frame}
        renderedSteps={options.renderedSteps ?? computed.steps ?? []}
        passThroughSteps={computed.passThroughSteps ?? 0}
        references={[]}
        frames={[]}
        addCalculatedColumnRequest={options.addCalculatedColumnRequest}
        onOperation={onOperation}
      />
    </ActiveFormulaEditorProvider>
  );
}

/**
 * The engine echoes a saved chain back verbatim, with names resolved through
 * the frame's columns. Deriving the echo from what the editor actually saved
 * — rather than a hand-written chain — is the point: the claim under test is
 * that the editor's own save comes back reconcilable.
 */
function echoOf(saved: FrameStepInput[]): RenderedFrameStep[] {
  return saved.map((step) => {
    if (step.kind !== "withColumns") throw new Error(`unexpected ${step.kind} step`);
    return {
      kind: "withColumns",
      columns: step.columns.map(({ outputColumnId, formula }) => ({
        outputColumnId,
        formula,
      })),
    };
  });
}

/** Open the Margin column's session the way a person does: click its row. */
function clickMarginAddress() {
  const address = screen
    .getByText(
      (content, element) =>
        element?.tagName === "CODE" && content.includes("Margin")
    )
    .closest("button")!;
  fireEvent.click(address);
}

describe("PipelineEditor formula sessions", () => {
  it("keeps the created column's session through its save's round trip, so the bar commits it", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);

    const { rerender } = render(
      editorFor(view, frame, onOperation, {
        addCalculatedColumnRequest: { token: 1 },
      })
    );

    // The request appended and saved a blank typed column, and moved the
    // shared editor onto it.
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    const saved = savedPipeline(onOperation.mock.calls[0][0]);
    const savedColumns =
      saved[0].kind === "withColumns" ? saved[0].columns : [];
    expect(savedColumns.map((column) => column.name)).toEqual([
      "Margin",
      "Column 1",
    ]);
    await waitFor(() => expect(commands.getActive()?.label).toBe("Column 1"));
    const sessionId = commands.getActive()!.id;

    // The bar is hosting the session — "Edit Column 1" — and the person is
    // already replacing the placeholder while the creation save is still
    // round-tripping. Typing ahead of the echo is the everyday timing.
    const editing = screen.getByLabelText("Edit Column 1") as HTMLTextAreaElement;
    fireEvent.change(editing, {
      target: { value: "`Column 1` = `Revenue` * 2" },
    });

    // Now the save's echo lands: the document renders the saved chain and
    // the frame carries the new column. The editor must recognize its own
    // save rather than treat it as the document moving underneath — the
    // session and the typing both ride through.
    const returned: FrameObject = {
      ...frame,
      columns: [
        ...frame.columns,
        {
          id: savedColumns[1].outputColumnId,
          name: "Column 1",
          dataType: "number",
        } as Column,
      ],
    };
    rerender(
      editorFor(view, returned, onOperation, {
        renderedSteps: echoOf(saved),
        addCalculatedColumnRequest: { token: 1 },
      })
    );
    expect(commands.getActive()?.id).toBe(sessionId);
    expect(commands.getActive()?.draft).toBe("`Column 1` = `Revenue` * 2");

    // Return is the commit gesture.
    fireEvent.keyDown(editing, { key: "Enter" });

    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(2));
    const recommitted = savedPipeline(onOperation.mock.calls[1][0]);
    const formulas =
      recommitted[0].kind === "withColumns"
        ? recommitted[0].columns.map((column) => column.formula)
        : [];
    expect(formulas).toContain("`Revenue` * 2");
    // Enter means "apply and be done": the session ended with the commit and
    // the bar returned to its resting mode.
    expect(commands.getActive()).toBeNull();
    expect(screen.getByLabelText("Add formula to Scratchwork")).toBeTruthy();
  });

  it("ends a session whose chain the document replaced from outside", async () => {
    const withMargin = fixtures.salesWithMargin;
    const afterUndo = fixtures.salesMarginDeleteRegion;
    const frame = objectNamed(withMargin, "frame", "Monthly sales");
    const frameAfter = objectNamed(afterUndo, "frame", "Monthly sales");
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);

    const { rerender } = render(editorFor(withMargin, frame, onOperation));
    clickMarginAddress();
    expect(commands.getActive()?.label).toBe("Margin");

    // The saved chain moves under the editor — an undo, a peer, another
    // surface. The drafts the session addressed are re-derived, so the
    // session ends with them instead of staying armed over callbacks that
    // would write the superseded chain back.
    rerender(editorFor(afterUndo, frameAfter, onOperation));
    await waitFor(() => expect(commands.getActive()).toBeNull());
  });

  // A commit can be refused with the inspector closed: the session lives in
  // the bar through the registry's retained binding, and the refusal must
  // read there — the panel that renders `formulaError` may not exist. The
  // two refusals arrive differently (the engine answers the save; a parse
  // rejection never reaches the engine), so each gets its own claim.

  it("keeps a session the engine refused, and reads the refusal in the bar", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    const onOperation = vi
      .fn<OperationHandler>()
      .mockResolvedValue("Unknown column `Revenu`");

    render(editorFor(view, frame, onOperation));
    clickMarginAddress();
    expect(commands.getActive()?.label).toBe("Margin");

    const editing = screen.getByLabelText("Edit Margin") as HTMLTextAreaElement;
    fireEvent.change(editing, {
      target: { value: "`Margin` = `Revenu` - `Cost`" },
    });
    fireEvent.keyDown(editing, { key: "Enter" });

    // The bar's feedback area carries the engine's sentence, as plain text.
    const notice = await screen.findByRole("status");
    expect(notice.textContent).toContain("Unknown column `Revenu`");
    // A refused commit is not "apply and be done": the session and the draft
    // both stay, so the correction happens in place instead of via Wrangle.
    expect(commands.getActive()?.label).toBe("Margin");
    expect(commands.getActive()?.draft).toBe("`Margin` = `Revenu` - `Cost`");
    expect(screen.getByLabelText("Edit Margin")).toBeTruthy();
  });

  it("keeps a session whose draft does not parse, and says what is missing", async () => {
    const view = fixtures.salesWithMargin;
    const frame = objectNamed(view, "frame", "Monthly sales");
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);

    render(editorFor(view, frame, onOperation));
    clickMarginAddress();
    const editing = screen.getByLabelText("Edit Margin") as HTMLTextAreaElement;
    // The backticked name was deleted along the way: nothing here can be
    // saved as a named column, and nothing is sent to the engine.
    fireEvent.change(editing, { target: { value: "Revenue - Cost" } });
    fireEvent.keyDown(editing, { key: "Enter" });

    const notice = await screen.findByRole("status");
    expect(notice.textContent).toContain(
      "Write a backticked column name, =, and a formula"
    );
    expect(onOperation).not.toHaveBeenCalled();
    expect(commands.getActive()?.label).toBe("Margin");
  });
});
