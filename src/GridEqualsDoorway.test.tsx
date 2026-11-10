// @vitest-environment jsdom
import { GRID_RETURN_EVENT } from "./hooks/useGridEdgeGrowth";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditorCommands,
} from "./ActiveFormulaEditor";
import { DerivedFrameCreator } from "./PipelineEditor";
import { ScratchworkFormulaBar } from "./ScratchworkFormulaBar";
import { gridCellFormulaAction } from "./lib/gridCellFormula";
import type { OperationHandler } from "./lib/handlers";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "./test/support";

afterEach(() => {
  cleanup();
  clearMocks();
});

// What typing `=10*2` into a cell of a literal column does.
//
// 1. The `=` keydown in the grid is consumed by handleGridFormulaKey
//    (GridFormulaKeyboard.ts): `gridCellFormulaAction` answers
//    `{ kind: "column" }`, the key is preventDefault'd, and the cell never
//    receives the character.
// 2. The doorway mints a transform request whose formula is the column's own
//    token (`formulaToken(column.name)`), focus true, focusAtEnd false, and
//    `fromCell` naming the cell and whether it can take a literal.
// 3. useColumnTransformRequest appends `Revenue = Revenue` as a local draft
//    -- nothing is saved -- and the editor opens with only the expression
//    after `name = ` selected, so the remaining keystrokes, `10*2`, replace
//    the expression and leave the name.
// 4. Return: reference-free arithmetic in a hand-entered cell folds to that
//    cell's value (one setCell, the draft dropped); anything else is saved
//    as the column's formula.
//
// The tests mount the real editor and the real bar and replay the typing as
// the change event a typed replacement of the selection produces.

let commands: ReturnType<typeof useActiveFormulaEditorCommands>;
function Probe() {
  commands = useActiveFormulaEditorCommands();
  return null;
}

const view = fixtures.salesWithMargin;
const frame = objectNamed(view, "frame", "Monthly sales");
const revenue = frame.columns.find((column) => column.name === "Revenue")!;
const SEEDED = "`Revenue` = `Revenue`";

type Mount = {
  literal?: boolean;
  fromCell?: boolean;
  addCalculatedColumn?: boolean;
};

function mountDoorway(onOperation: OperationHandler, options: Mount = {}) {
  const { literal = false, fromCell = true, addCalculatedColumn = false } = options;
  const computed = view.computedFrames[frame.id]!;
  render(
    <ActiveFormulaEditorProvider>
      <Probe />
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
        renderedSteps={computed.steps ?? []}
        passThroughSteps={computed.passThroughSteps ?? 0}
        references={[]}
        frames={[]}
        addCalculatedColumnRequest={addCalculatedColumn ? { token: 1 } : undefined}
        transformColumnRequest={
          addCalculatedColumn
            ? undefined
            : {
                token: 1,
                columnId: revenue.id,
                // formulaToken(column.name), as handleGridFormulaKey seeds it.
                formula: "`Revenue`",
                focus: true,
                focusAtEnd: false,
                fromCell: fromCell ? { rowId: "row-3", literal } : undefined,
              }
        }
        onOperation={onOperation}
      />
    </ActiveFormulaEditorProvider>
  );
}

async function typeAndReturn(value: string) {
  await waitFor(() => expect(commands.getActive()?.draft).toBe(SEEDED));
  const editing = (await screen.findByLabelText("Edit Revenue")) as HTMLTextAreaElement;
  fireEvent.change(editing, { target: { value } });
  fireEvent.keyDown(editing, { key: "Enter" });
}

const pipelineSaves = (onOperation: ReturnType<typeof vi.fn<OperationHandler>>) =>
  onOperation.mock.calls
    .map(([operation]) => operation)
    .filter((operation) => operation.type === "setFramePipeline");

function lastSavedFormula(onOperation: ReturnType<typeof vi.fn<OperationHandler>>) {
  const saves = pipelineSaves(onOperation);
  const last = saves[saves.length - 1];
  if (last?.type !== "setFramePipeline") return undefined;
  const step = last.steps[last.steps.length - 1];
  return step.kind === "withColumns" ? step.columns[0] : undefined;
}

describe("the = doorway from a literal cell", () => {
  it("consumes the = keystroke and routes to the column, never typing it into the cell", () => {
    expect(
      gridCellFormulaAction({
        key: "=",
        modifier: false,
        printable: true,
        isOverride: false,
        singleCell: true,
      })
    ).toEqual({ kind: "column" });
  });

  it("opens the column's own line with only the expression selected, and saves nothing yet", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation);
    await waitFor(() => expect(commands.getActive()?.draft).toBe(SEEDED));
    expect(commands.getActive()?.selection).toEqual({
      start: SEEDED.length - "`Revenue`".length,
      end: SEEDED.length,
    });
    // The column restating itself means nothing yet: no step and no undo
    // entry exist until Return says what the formula is.
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("keeps the name when a reference expression is typed over the selection", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { literal: true });
    // What typing `Cost * 2` over the selected expression leaves.
    await typeAndReturn("`Revenue` = `Cost` * 2");
    await waitFor(() => expect(pipelineSaves(onOperation)).toHaveLength(1));
    expect(lastSavedFormula(onOperation)).toMatchObject({
      outputColumnId: revenue.id,
      name: "Revenue",
      formula: "`Cost` * 2",
    });
    expect(onOperation.mock.calls.some(([operation]) => operation.type === "setCell")).toBe(
      false
    );
  });

  it("commits a line whose name was deleted as the target column's expression", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    // A column that cannot take a literal: the arithmetic is the column's.
    mountDoorway(onOperation, { literal: false });
    await typeAndReturn("=10*2");
    await waitFor(() => expect(pipelineSaves(onOperation)).toHaveLength(1));
    expect(lastSavedFormula(onOperation)).toMatchObject({
      outputColumnId: revenue.id,
      name: "Revenue",
      formula: "10*2",
    });
  });

  it("folds reference-free arithmetic into the one cell, leaving the chain as it was", async () => {
    const sample = vi.fn(() => ({
      frameId: frame.id,
      stepIndex: 0,
      columns: [revenue],
      rows: [["20"]],
      truncated: true,
    }));
    serveInvoke({
      sample_frame_step: sample,
      preview_frame_pipeline: () => ({ steps: [], failedStep: null, error: null }),
    });
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { literal: true });
    await waitFor(() => expect(commands.getActive()?.draft).toBe(SEEDED));
    const editing = (await screen.findByLabelText("Edit Revenue")) as HTMLTextAreaElement;
    fireEvent.change(editing, { target: { value: "`Revenue` = 10*2" } });
    // Nothing claims "all rows" when one cell is what will be written.
    await waitFor(() =>
      expect(commands.getActive()?.completion.appliesToAllRows).toBe(false)
    );
    const returned = vi.fn();
    window.addEventListener(GRID_RETURN_EVENT, returned);
    fireEvent.keyDown(editing, { key: "Enter" });

    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    expect(onOperation.mock.calls[0][0]).toEqual({
      type: "setCell",
      frameId: frame.id,
      rowId: "row-3",
      columnId: revenue.id,
      raw: "20",
    });
    // The keyboard goes back to the grid, one row down, once the value is in.
    await waitFor(() => expect(returned).toHaveBeenCalledTimes(1));
    window.removeEventListener(GRID_RETURN_EVENT, returned);
    // The engine was asked, with the expression and nothing of the chain.
    expect(sample.mock.calls[0]).toEqual([
      {
        frameId: frame.id,
        steps: [
          {
            kind: "withColumns",
            columns: [{ outputColumnId: revenue.id, name: "Revenue", formula: "10*2" }],
          },
        ],
        stepIndex: 0,
        limit: 1,
      },
    ]);
    expect(pipelineSaves(onOperation)).toHaveLength(0);
    await waitFor(() => expect(commands.getActive()).toBeNull());
    expect(screen.queryByText(/`Revenue` = /)).toBeNull();
  });

  it("says all rows again once the draft reads a column", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { literal: true });
    await waitFor(() => expect(commands.getActive()?.draft).toBe(SEEDED));
    const editing = (await screen.findByLabelText("Edit Revenue")) as HTMLTextAreaElement;
    fireEvent.change(editing, { target: { value: "`Revenue` = 10*2" } });
    await waitFor(() =>
      expect(commands.getActive()?.completion.appliesToAllRows).toBe(false)
    );
    fireEvent.change(editing, { target: { value: "`Revenue` = `Cost` * 2" } });
    await waitFor(() =>
      expect(commands.getActive()?.completion.appliesToAllRows).toBe(true)
    );
  });

  it("drops the unsaved line when Escape abandons the doorway", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { literal: true });
    await waitFor(() => expect(commands.getActive()?.draft).toBe(SEEDED));
    expect(screen.getAllByText(SEEDED).length).toBeGreaterThan(0);
    commands.cancel();
    await waitFor(() => expect(screen.queryAllByText(SEEDED)).toHaveLength(0));
    expect(onOperation).not.toHaveBeenCalled();
  });

  it("still saves a transform that did not start in a cell as it opens", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { fromCell: false });
    await waitFor(() => expect(pipelineSaves(onOperation)).toHaveLength(1));
  });

  it("opens a fresh Column N placeholder with the whole line selected", async () => {
    const onOperation = vi.fn<OperationHandler>().mockResolvedValue(null);
    mountDoorway(onOperation, { addCalculatedColumn: true });
    await waitFor(() => expect(commands.getActive()?.label).toMatch(/^Column \d+$/));
    const active = commands.getActive()!;
    expect(active.draft).toMatch(/^`Column \d+` = None\s*\.cast\("number"\)$/);
    expect(active.selection).toEqual({ start: 0, end: active.draft.length });
  });
});
