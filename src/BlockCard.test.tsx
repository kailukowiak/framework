// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActiveFormulaEditorProvider, useActiveFormulaEditor } from "./ActiveFormulaEditor";
import { BlockCard, BlockCardPreview } from "./BlockCard";
import { NumberDisplayContext } from "./FrameGrid";
import type { OperationHandler } from "./lib/handlers";
import type { ComputedBlock, Operation } from "./lib/types";
import { readVectorDrag, type VectorDrag } from "./lib/vectorDrag";
import { fixtures, objectNamed } from "./test/support";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(document, "elementFromPoint");
});

function pointerEvent(type: string, x: number, y: number) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperties(event, {
    clientX: { value: x },
    clientY: { value: y },
  });
  return event;
}

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
          valueCount: 1,
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

  it("drags a named vector answer by its Scratchwork formula address", () => {
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    const computed: ComputedBlock = {
      source: "months = sequence(1, 4)",
      lines: [
        {
          id: "months",
          name: "months",
          text: "months = sequence(1, 4)",
          comment: false,
          blank: false,
          dataType: "integer",
          valueCount: 3,
          value: null,
          typedValue: { type: "null" },
          display: "[1, 2, 3]",
          error: null,
          isOverride: false,
        },
      ],
    };
    let dropped: VectorDrag | null = null;
    render(
      <ActiveFormulaEditorProvider>
        <BlockCard
          block={checks}
          computed={computed}
          objects={view.objects}
          computedFrames={view.computedFrames}
          formulaFunctions={view.formulaFunctions}
          onOperation={vi.fn(async () => null)}
          onFreeze={vi.fn(async () => undefined)}
        />
        <div
          className="canvas-viewport"
          aria-label="Canvas drop target"
          onDrop={(event) => {
            dropped = readVectorDrag(event.dataTransfer);
          }}
        />
      </ActiveFormulaEditorProvider>
    );
    const target = screen.getByLabelText("Canvas drop target");
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => target),
    });

    const handle = document.querySelector(".formula-drag-handle");
    expect(handle).not.toBeNull();
    fireEvent.pointerDown(handle!, { button: 0, clientX: 10, clientY: 10 });
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));
    window.dispatchEvent(pointerEvent("pointerup", 20, 20));

    expect(dropped).toEqual({
      objectId: "months",
      formula: "`Checks`.`months`",
      name: "months",
      length: 3,
    });
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

  it("survives a parent that re-renders on every editor snapshot", async () => {
    // The Scratchwork window subscribes to the whole editor snapshot. Each
    // render of this card rebinds it with its completion list, and a fresh
    // list per render made rebind publish, the parent render, the card
    // rebind -- React #185. The list is memoized now; this pins that.
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    function Watching() {
      const { active } = useActiveFormulaEditor();
      return (
        <>
          <output data-testid="draft">{active?.draft ?? ""}</output>
          <BlockCard
            block={checks}
            computed={view.computedBlocks[checks.id]}
            objects={view.objects}
            computedFrames={view.computedFrames}
            formulaFunctions={view.formulaFunctions}
            onOperation={vi.fn(async () => null)}
            onFreeze={vi.fn(async () => undefined)}
          />
        </>
      );
    }
    render(
      <ActiveFormulaEditorProvider>
        <Watching />
      </ActiveFormulaEditorProvider>
    );
    const editor = screen.getByLabelText("Checks lines");
    // Inside an unclosed backtick the offered list is a filtered copy, which
    // is where the identity churn lived.
    await userEvent.type(editor, "total = `Mo");
    expect(screen.getByTestId("draft").textContent).toBe("total = `Mo");
  });

  it("reopens a session when ⌘J lands on a textarea that never lost focus", async () => {
    // The workbook can end the pop-out's session from across the window
    // boundary while the textarea stays the pop-out's active element. The
    // next ⌘J must start a session again, or typing publishes nothing.
    const view = fixtures.salesBeforeFormula;
    const checks = objectNamed(view, "block", "Checks");
    let clearSession: (() => void) | null = null;
    function Watching({ focusToken }: { focusToken: number }) {
      const { active, clear } = useActiveFormulaEditor();
      clearSession = clear;
      return (
        <>
          <output data-testid="session">{active?.id ?? "none"}</output>
          <BlockCard
            block={checks}
            computed={view.computedBlocks[checks.id]}
            focusToken={focusToken}
            objects={view.objects}
            computedFrames={view.computedFrames}
            formulaFunctions={view.formulaFunctions}
            onOperation={vi.fn(async () => null)}
            onFreeze={vi.fn(async () => undefined)}
          />
        </>
      );
    }
    const { rerender } = render(
      <ActiveFormulaEditorProvider>
        <Watching focusToken={1} />
      </ActiveFormulaEditorProvider>
    );
    const editor = screen.getByLabelText("Checks lines");
    await waitFor(() => expect(document.activeElement).toBe(editor));
    expect(screen.getByTestId("session").textContent).toBe(`scratchwork:${checks.id}`);

    act(() => clearSession?.());
    expect(screen.getByTestId("session").textContent).toBe("none");
    expect(document.activeElement).toBe(editor);

    rerender(
      <ActiveFormulaEditorProvider>
        <Watching focusToken={2} />
      </ActiveFormulaEditorProvider>
    );
    await waitFor(() =>
      expect(screen.getByTestId("session").textContent).toBe(`scratchwork:${checks.id}`)
    );
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
