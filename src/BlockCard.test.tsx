// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { BlockCard, BlockCardPreview } from "./BlockCard";
import { NumberDisplayContext } from "./FrameGrid";
import type { OperationHandler } from "./lib/handlers";
import type { Operation } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

// The exemplar for the interaction tier: mount the real component over a
// framework-core-generated fixture, act like a person, and assert the
// operation the interface emits. What the operation *means* is Rust's test
// to write; that the emitted operation actually reaches Rust is the e2e
// suite's. This test owns the middle claim: typing in the Scratchwork
// editor sends setBlockSource with what was typed, live, without any
// commit gesture — because evaluation is live and there is no Execute.
describe("BlockCard", () => {
  it("renders scalar answers with the shared number format preference", () => {
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    const computed = {
      source: "total = 1234.5",
      lines: [
        {
          id: "total",
          name: "total",
          text: "total = 1234.5",
          comment: false,
          blank: false,
          dataType: "number" as const,
          value: 1234.5,
          typedValue: { type: "number" as const, value: 1234.5 },
          display: "1234.5",
          error: null,
          isOverride: false,
        },
      ],
    };

    const { rerender } = render(
      <BlockCardPreview block={checks} computed={computed} />
    );
    expect(screen.getByText("1,234.50")).not.toBeNull();

    rerender(
      <NumberDisplayContext.Provider value={false}>
        <BlockCardPreview block={checks} computed={computed} />
      </NumberDisplayContext.Provider>
    );
    expect(screen.getByText("1234.50")).not.toBeNull();
  });

  it("emits setBlockSource for typed lines, live", async () => {
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    const onOperation = vi.fn<OperationHandler>(async () => null);

    render(
      <ActiveFormulaEditorProvider>
        <BlockCard
          block={checks}
          computed={view.computedBlocks[checks.id]}
          objects={view.objects}
          computedFrames={view.computedFrames}
          formulaFunctions={view.formulaFunctions}
          onOperation={onOperation}
          onFreeze={vi.fn(async () => undefined)}
        />
      </ActiveFormulaEditorProvider>
    );

    const editor = screen.getByLabelText("Checks lines");
    await userEvent.type(editor, "x = 1");

    // Live evaluation debounces behind the keystrokes; the assertion waits
    // for the operation rather than racing it.
    await waitFor(() => {
      const setSource = onOperation.mock.calls
        .map(([operation]) => operation)
        .filter(
          (operation): operation is Extract<Operation, { type: "setBlockSource" }> =>
            operation.type === "setBlockSource"
        );
      expect(setSource.length).toBeGreaterThan(0);
      expect(setSource.at(-1)?.source).toBe("x = 1");
    });
  });

  it("replaces a partial qualified column without repeating its frame", async () => {
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    render(
      <ActiveFormulaEditorProvider>
        <BlockCard
          block={checks}
          computed={view.computedBlocks[checks.id]}
          objects={view.objects}
          computedFrames={view.computedFrames}
          formulaFunctions={view.formulaFunctions}
          onOperation={vi.fn(async () => null)}
          onFreeze={vi.fn(async () => undefined)}
        />
      </ActiveFormulaEditorProvider>
    );

    const editor = screen.getByLabelText("Checks lines") as HTMLTextAreaElement;
    const partial = "`Monthly sales`.Rev";
    fireEvent.focus(editor);
    fireEvent.change(editor, {
      target: {
        value: partial,
        selectionStart: partial.length,
        selectionEnd: partial.length,
      },
    });
    expect(await screen.findByRole("button", { name: /Revenue/ })).not.toBeNull();

    fireEvent.keyDown(editor, { key: "Tab" });
    expect(editor.value).toBe("`Monthly sales`.`Revenue`");
  });
});
