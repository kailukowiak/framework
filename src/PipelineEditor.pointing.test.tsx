// @vitest-environment jsdom
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useEffect, useState, type ReactNode } from "react";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditorCommands,
} from "./ActiveFormulaEditor";
import { canvasFormulaPointerHandler } from "./CanvasFormulaPicking";
import { DerivedFrameCreator } from "./PipelineEditor";
import { ScratchworkFormulaBar } from "./ScratchworkFormulaBar";
import type { OperationHandler } from "./lib/handlers";
import type {
  DocumentView,
  FrameObject,
  FrameStepInput,
  RenderedFrameStep,
} from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

// Pointing at the grid while a wrangle formula is open, mounted end to end:
// the real pipeline editor writing the column, the real formula bar hosting
// the session, and the app shell's own pointer routing over a grid drawn the
// way the frame card draws one. The claims here are the ones a pure pick test
// cannot make — what the *draft* ends up saying, and what the click that
// follows the press is allowed to do.

afterEach(cleanup);

let commands: ReturnType<typeof useActiveFormulaEditorCommands>;
function Probe() {
  commands = useActiveFormulaEditorCommands();
  return null;
}

let notices: Array<string | null> = [];

function Shell({ view, children }: { view: DocumentView; children: ReactNode }) {
  const local = useActiveFormulaEditorCommands();
  return (
    <div
      onPointerDownCapture={canvasFormulaPointerHandler({
        document: view,
        getActive: local.getActive,
        insertReference: local.insertReference,
        clear: local.clear,
        disengage: local.disengage,
        onNotice: (notice) => notices.push(notice),
        onRecurrence: () => {},
      })}
    >
      {children}
    </div>
  );
}

function press(target: Element): { clickReached: boolean } {
  target.dispatchEvent(
    new MouseEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
  );
  return {
    clickReached: target.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true })
    ),
  };
}

/**
 * The engine's answer to a saved chain, close enough for this seam: the
 * steps come back rendered, and the frame grows the columns they declare.
 * Nothing computes — this only replays what the save was handed, which is
 * exactly the echo `useDocumentChainSync` compares against.
 */
function echoed(
  frame: FrameObject,
  steps: FrameStepInput[]
): { frame: FrameObject; renderedSteps: RenderedFrameStep[] } {
  const columns = [...frame.columns];
  const renderedSteps = steps.map((step): RenderedFrameStep => {
    if (step.kind !== "withColumns") return step as RenderedFrameStep;
    for (const column of step.columns)
      if (!columns.some((existing) => existing.id === column.outputColumnId))
        columns.push({
          ...frame.columns[0],
          id: column.outputColumnId,
          name: column.name,
        });
    return {
      kind: "withColumns",
      columns: step.columns.map((column) => ({
        outputColumnId: column.outputColumnId,
        formula: column.formula,
      })),
    };
  });
  // A frame that had no chain grows one: its declared schema becomes the
  // chain's output, and `baseColumns` starts carrying what it holds itself.
  const baseColumns = frame.baseColumns?.length
    ? frame.baseColumns
    : steps.length
    ? frame.columns
    : [];
  return { frame: { ...frame, columns, baseColumns }, renderedSteps };
}

function Board({
  view,
  frame: initialFrame,
  renderedSteps: initialSteps,
  anchorRowIndex,
  deferRequest = false,
  children,
}: {
  view: DocumentView;
  frame: FrameObject;
  renderedSteps: RenderedFrameStep[];
  anchorRowIndex: number | undefined;
  deferRequest?: boolean;
  children: ReactNode;
}) {
  const [chain, setChain] = useState({
    frame: initialFrame,
    renderedSteps: initialSteps,
  });
  // The Wrangle panel is often already open when "Formula here" is chosen,
  // so the request reaches a mounted editor as a prop change rather than
  // arriving with it.
  const [asked, setAsked] = useState(!deferRequest);
  useEffect(() => {
    if (!asked) setAsked(true);
  }, [asked]);
  const onOperation: OperationHandler = async (operation) => {
    if (operation.type === "setFramePipeline")
      setChain((current) => echoed(current.frame, operation.steps));
    return null;
  };
  return (
    <Shell view={view}>
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
        // Exactly FrameInspector's rule, because it is what moves under a
        // session mid-gesture: a frame with no chain answers completion
        // against itself, and the moment the first calculation is saved it
        // grows `baseColumns` and stops.
        input={{
          label: "Source: this frame’s own data",
          columns: chain.frame.baseColumns?.length
            ? chain.frame.baseColumns
            : chain.frame.columns,
          completionFrameId: chain.frame.baseColumns?.length
            ? undefined
            : chain.frame.id,
        }}
        editingFrame={chain.frame}
        renderedSteps={chain.renderedSteps}
        passThroughSteps={0}
        references={[]}
        frames={[]}
        addCalculatedColumnRequest={asked ? { token: 1, anchorRowIndex } : undefined}
        onOperation={onOperation}
      />
      {children}
    </Shell>
  );
}

function workbook(
  anchorRowIndex: number | undefined,
  {
    sorted = false,
    bare = false,
    deferRequest = false,
  }: { sorted?: boolean; bare?: boolean; deferRequest?: boolean } = {}
) {
  notices = [];
  // `bare` is the tutorial's own shape: a document-owned frame with no chain
  // at all, sorted for display only. The first calculation written on it is
  // therefore also the moment the frame grows a chain.
  const view = bare ? fixtures.salesBeforeFormula : fixtures.salesWithMargin;
  const frame = objectNamed(view, "frame", "Monthly sales");
  const computed = view.computedFrames[frame.id]!;
  const revenue = frame.columns.find((column) => column.name === "Revenue")!;
  const margin = frame.columns.find((column) => column.name === "Margin") ?? revenue;
  const month = frame.columns.find((column) => column.name === "Month")!;
  // A Sort at the end of the chain is what the tutorial workbook actually
  // looks like, and it is the shape that forces the new calculation into a
  // brand-new step rather than into the one already there.
  const renderedSteps: RenderedFrameStep[] = sorted
    ? [
        ...(computed.steps ?? []),
        { kind: "sort", keys: [{ columnId: month.id, descending: false }] },
      ]
    : (computed.steps ?? []);
  const rendered = render(
    <ActiveFormulaEditorProvider>
      <Probe />
      <Board
        view={view}
        frame={frame}
        renderedSteps={renderedSteps}
        anchorRowIndex={anchorRowIndex}
        deferRequest={deferRequest}
      >
        {/* Two rows of the frame, drawn as the card draws them: a plain
            column's value is a div that carries its own click, a calculated
            column's is a button. */}
        <div data-frame-id={frame.id}>
          <table>
            <tbody>
              {[0, 1].map((index) => (
                <tr data-row-index={index} key={index}>
                  <td data-column-id={revenue.id}>
                    <div className="cell-display">{118000 + index * 6000}</div>
                  </td>
                  <td data-column-id={margin.id}>
                    <button className="computed-cell">{index}</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Board>
    </ActiveFormulaEditorProvider>
  );
  return { ...rendered, margin };
}

describe("pointing at the grid from a wrangle formula", () => {
  it("writes the row above the anchor as one period back, name intact", async () => {
    const { container } = workbook(1);
    await waitFor(() => expect(commands.getActive()?.label).toBe("Column 1"));

    const rowAbove = container.querySelectorAll(".cell-display")[0];
    const { clickReached } = press(rowAbove);

    // The reference is the shift the anchor implies, and it landed in the
    // formula rather than over the column's name — a line without its name
    // is not a calculation any more and cannot be saved at all.
    expect(commands.getActive()?.draft).toBe("`Column 1` = `Revenue`.shift(1)");
    await waitFor(() =>
      expect(
        (screen.getByLabelText("Edit Column 1") as HTMLTextAreaElement).value
      ).toBe("`Column 1` = `Revenue`.shift(1)")
    );
    // The press was a pick, so the click it carries is not also a cell click,
    // and the keyboard stays where the formula is being written.
    expect(clickReached).toBe(false);
    await waitFor(() =>
      expect(window.document.activeElement).toBe(
        screen.getByLabelText("Edit Column 1")
      )
    );
  });

  it("keeps the anchor on the very first pick of a frame with no chain yet", async () => {
    const { container } = workbook(1, { bare: true, deferRequest: true });
    await waitFor(() => expect(commands.getActive()?.label).toBe("Column 1"));
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 60));
    });

    press(container.querySelectorAll(".cell-display")[0]);

    expect(commands.getActive()?.draft).toBe("`Column 1` = `Revenue`.shift(1)");
  });

  it("keeps the anchor on the very first pick of a chain that ends in a Sort", async () => {
    const { container } = workbook(1, { sorted: true });
    await waitFor(() => expect(commands.getActive()?.label).toBe("Column 1"));
    // The person looks at what opened before clicking, so the save's echo
    // and the bar's deferred focus have both landed by the time they do.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 60));
    });
    await waitFor(() =>
      expect(window.document.activeElement).toBe(
        screen.getByLabelText("Edit Column 1")
      )
    );

    // The first click, not the second: the anchor the gesture opened with
    // has to be the session's from the moment it opens, or the reference
    // that reads the row above comes out as a plain column reference.
    press(container.querySelectorAll(".cell-display")[0]);

    expect(commands.getActive()?.draft).toBe("`Column 1` = `Revenue`.shift(1)");
  });

  it("says why a calculation beside this one cannot be read yet", async () => {
    const { container } = workbook(1);
    await waitFor(() => expect(commands.getActive()?.label).toBe("Column 1"));

    // Margin is written in the same Add or replace columns step, so it does
    // not exist when Column 1 runs. The click must say so and leave the
    // session alone instead of quietly selecting the column and abandoning
    // the draft.
    press(container.querySelectorAll(".computed-cell")[0]);

    expect(commands.getActive()?.label).toBe("Column 1");
    expect(notices.at(-1)).toBe(
      "Margin is added in this same step, so Column 1 cannot read it yet. Add Column 1 as a new step to read it."
    );
  });
});
