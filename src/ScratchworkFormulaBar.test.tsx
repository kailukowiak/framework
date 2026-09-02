// @vitest-environment jsdom
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditorCommands,
} from "./ActiveFormulaEditor";
import { canvasFormulaPointerHandler } from "./CanvasFormulaPicking";
import { PipelineCommand } from "./PipelineCommand";
import { ScratchworkFormulaBar } from "./ScratchworkFormulaBar";
import type { FormulaBarCell } from "./lib/formulaBarCell";
import type { DocumentView } from "./lib/types";

const selectedCell: FormulaBarCell = {
  id: "ledger:r1:amount",
  label: "Amount · row 1",
  kind: "literal",
  value: "42",
  frameId: "ledger",
  rowId: "r1",
  columnId: "amount",
  rowIndex: 0,
};

function bar(cell: FormulaBarCell | null) {
  return (
    <ScratchworkFormulaBar
      onCommit={vi.fn(async () => ({ saved: true }))}
      references={[]}
      cell={cell}
      onCommitCell={vi.fn(async () => null)}
      onEditCalculatedCell={vi.fn()}
      onEditOverrideCell={vi.fn()}
      onRequestReadOnlyCell={vi.fn()}
      expanded={false}
      onToggle={vi.fn()}
    />
  );
}

/**
 * The app shell's pointer routing, mounted around real components so a test
 * can click "the canvas" the way App.tsx wires it. Nothing here computes: the
 * handler is the production one and the registry is the real provider's.
 */
function CanvasHarness({ children }: { children: ReactNode }) {
  const commands = useActiveFormulaEditorCommands();
  return (
    <div
      onPointerDownCapture={canvasFormulaPointerHandler({
        document: { objects: [], computedFrames: {} } as unknown as DocumentView,
        getActive: commands.getActive,
        insertReference: commands.insertReference,
        clear: commands.clear,
        disengage: commands.disengage,
        onNotice: () => {},
        onRecurrence: () => {},
      })}
    >
      {children}
    </div>
  );
}

afterEach(cleanup);

describe("ScratchworkFormulaBar", () => {
  it("commits a formatted active formula through its owning editor", async () => {
    const onCommit = vi.fn(async () => undefined);
    render(
      <ActiveFormulaEditorProvider>
        <PipelineCommand
          editorId="formula:date"
          label="Date"
          initialDraft="value.dt.month_start()"
          references={[]}
          onChange={vi.fn()}
          onCommit={onCommit}
        />
        {bar(null)}
      </ActiveFormulaEditorProvider>
    );

    await userEvent.click(screen.getByRole("button", { name: /value\.dt/ }));
    await userEvent.click(screen.getByRole("button", { name: "Format" }));

    expect(onCommit).toHaveBeenCalledWith("value\n  .dt\n  .month_start()");
  });

  it("formatting keeps the session; it is not a finishing gesture", async () => {
    render(
      <ActiveFormulaEditorProvider>
        <PipelineCommand
          editorId="formula:date"
          label="Date"
          initialDraft="value.dt.month_start()"
          references={[]}
          onChange={vi.fn()}
          onCommit={vi.fn(async () => undefined)}
        />
        {bar(selectedCell)}
      </ActiveFormulaEditorProvider>
    );

    await userEvent.click(screen.getByRole("button", { name: /value\.dt/ }));
    await userEvent.click(screen.getByRole("button", { name: "Format" }));

    expect(screen.getByLabelText("Edit Date")).toBeTruthy();
  });

  it("Return commits the formula, ends the session, and returns the bar to the cell", async () => {
    const onCommit = vi.fn(async () => undefined);
    const { container } = render(
      <ActiveFormulaEditorProvider>
        <CanvasHarness>
          <PipelineCommand
            editorId="formula:amount"
            label="Amount"
            initialDraft="`Debit` - `Credit`"
            references={[]}
            onChange={vi.fn()}
            onCommit={onCommit}
          />
          {bar(selectedCell)}
          <div data-frame-id="ledger">
            <table>
              <tbody>
                <tr data-row-index="0">
                  <td data-column-id="je">JE-1</td>
                </tr>
              </tbody>
            </table>
          </div>
        </CanvasHarness>
      </ActiveFormulaEditorProvider>
    );

    await userEvent.click(screen.getByRole("button", { name: /`Debit`/ }));
    const editing = screen.getByLabelText("Edit Amount");
    expect(container.querySelector("form")?.className).toContain("session");

    fireEvent.keyDown(editing, { key: "Enter" });

    expect(onCommit).toHaveBeenCalledWith("`Debit` - `Credit`");
    // The session is over: the bar answers for the selected cell again
    // instead of replaying the dead session's draft.
    const cellField = await screen.findByLabelText("Edit Amount · row 1");
    expect((cellField as HTMLTextAreaElement).value).toBe("42");
    expect(container.querySelector("form")?.className).not.toContain("session");

    // And a grid cell click after the commit is an ordinary click: nothing
    // intercepts it, and the bar's draft does not change underneath it.
    const gridCell = container.querySelector("td[data-column-id]")!;
    const passedThrough = gridCell.dispatchEvent(
      new MouseEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
    );
    expect(passedThrough).toBe(true);
    expect((cellField as HTMLTextAreaElement).value).toBe("42");
  });

  it("Escape restores the formula the session opened with and releases the bar", async () => {
    const onChange = vi.fn();
    render(
      <ActiveFormulaEditorProvider>
        <PipelineCommand
          editorId="formula:amount"
          label="Amount"
          initialDraft="`Debit` - `Credit`"
          references={[]}
          onChange={onChange}
          onCommit={vi.fn(async () => undefined)}
        />
        {bar(null)}
      </ActiveFormulaEditorProvider>
    );

    await userEvent.click(screen.getByRole("button", { name: /`Debit`/ }));
    const editing = screen.getByLabelText("Edit Amount");
    fireEvent.change(editing, { target: { value: "`Debit` - `Credit` - 1" } });
    fireEvent.keyDown(editing, { key: "Escape" });

    expect(onChange).toHaveBeenLastCalledWith("`Debit` - `Credit`");
    await waitFor(() =>
      expect(screen.getByLabelText("Add formula to Scratchwork")).toBeTruthy()
    );
  });
});
