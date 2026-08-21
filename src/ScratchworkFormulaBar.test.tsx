// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { PipelineCommand } from "./PipelineCommand";
import { ScratchworkFormulaBar } from "./ScratchworkFormulaBar";

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
      </ActiveFormulaEditorProvider>
    );

    await userEvent.click(screen.getByRole("button", { name: /value\.dt/ }));
    await userEvent.click(screen.getByRole("button", { name: "Format" }));

    expect(onCommit).toHaveBeenCalledWith("value\n  .dt\n  .month_start()");
  });
});
