// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ActiveFormulaEditorProvider,
  useActiveFormulaEditor,
} from "./ActiveFormulaEditor";
import { SeriesCard, ValueCard, VariableCard } from "./ScalarCards";
import type { OperationHandler } from "./lib/handlers";
import type { ValueObject } from "./lib/types";
import { readVectorDrag, type VectorDrag } from "./lib/vectorDrag";

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

function FormulaBarMirror() {
  const { active, setDraft } = useActiveFormulaEditor();
  return (
    <button
      type="button"
      onClick={() => setDraft("[1, 2, 3]", { start: 9, end: 9 })}
    >
      Top formula: {active?.draft ?? "none"}
    </button>
  );
}

describe("VariableCard", () => {
  it("shares its multiline formula draft with the top formula editor", async () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <ActiveFormulaEditorProvider>
        <VariableCard
          result={{
            kind: "result",
            id: "variable",
            name: "my variable name",
            formula: { expression: { type: "integer", value: 2 } },
            variable: true,
          }}
          formula="`my variable name`"
          computed={{
            formula: "2",
            dataType: "integer",
            valueCount: 1,
            value: 2,
            typedValue: { type: "number", value: 2 },
            display: "2",
            error: null,
            isOverride: false,
          }}
          objects={[]}
          computedFrames={{}}
          formulaFunctions={[]}
          onOperation={onOperation}
          onMovePointerDown={vi.fn()}
        />
        <FormulaBarMirror />
      </ActiveFormulaEditorProvider>
    );

    const formula = screen.getByRole("textbox", { name: "my variable name =" });
    await userEvent.click(formula);
    expect(screen.getByRole("button", { name: "Top formula: 2" })).toBeTruthy();

    await userEvent.click(
      screen.getByRole("button", { name: "Top formula: 2" })
    );
    expect((formula as HTMLTextAreaElement).value).toBe("[1, 2, 3]");
    fireEvent.keyDown(formula, { key: "Enter", metaKey: true });
    expect(onOperation).toHaveBeenCalledWith(
      {
        type: "setResultFormula",
        objectId: "variable",
        formula: "[1, 2, 3]",
      },
      { inlineError: true }
    );
  });

  it("shows and saves formulas in the multiline autoformatted form", async () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <ActiveFormulaEditorProvider>
        <VariableCard
          result={{
            kind: "result",
            id: "variable",
            name: "chosen",
            formula: { expression: { type: "integer", value: 1 } },
            variable: true,
          }}
          formula="`chosen`"
          computed={{
            formula: "[1, 2, 3].at(1)",
            dataType: "integer",
            valueCount: 1,
            value: 1,
            typedValue: { type: "number", value: 1 },
            display: "1",
            error: null,
            isOverride: false,
          }}
          objects={[]}
          computedFrames={{}}
          formulaFunctions={[]}
          onOperation={onOperation}
          onMovePointerDown={vi.fn()}
        />
      </ActiveFormulaEditorProvider>
    );

    const formula = screen.getByRole("textbox", { name: "chosen =" });
    expect((formula as HTMLTextAreaElement).value).toBe("[1, 2, 3]\n  .at(1)");
    fireEvent.focus(formula);
    fireEvent.change(formula, { target: { value: "[3, 4].at(1)" } });
    fireEvent.blur(formula);
    expect(onOperation).toHaveBeenCalledWith(
      {
        type: "setResultFormula",
        objectId: "variable",
        formula: "[3, 4]\n  .at(1)",
      },
      { inlineError: true }
    );
  });

  it("sizes its name naturally and removes formula quoting from renamed names", async () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <ActiveFormulaEditorProvider>
        <VariableCard
          result={{
            kind: "result",
            id: "variable",
            name: "my variable name",
            formula: { expression: { type: "integer", value: 2 } },
            variable: true,
          }}
          formula="`my variable name`"
          computed={undefined}
          objects={[]}
          computedFrames={{}}
          formulaFunctions={[]}
          onOperation={onOperation}
          onMovePointerDown={vi.fn()}
        />
      </ActiveFormulaEditorProvider>
    );

    const name = screen.getByRole("textbox", { name: "Variable name" });
    expect((name as HTMLInputElement).style.width).toBe("17ch");
    await userEvent.clear(name);
    await userEvent.type(name, "`another variable`");
    fireEvent.blur(name);
    expect(onOperation).toHaveBeenCalledWith({
      type: "renameObject",
      objectId: "variable",
      name: "another variable",
    });
  });
});

describe("ValueCard", () => {
  it("uses the native date picker for a date input", () => {
    render(
      <ValueCard
        value={
          {
            id: "date",
            kind: "value",
            name: "Timesheet date",
            raw: "2026-08-17",
            value: "2026-08-17",
            dataType: "date",
          } as ValueObject
        }
        formula="`Timesheet date`"
        onOperation={vi.fn()}
      />
    );

    expect(screen.getByDisplayValue("2026-08-17").getAttribute("type")).toBe(
      "date"
    );
  });

  it("shows the override and names the scenario, and edits that scenario", async () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    render(
      <ValueCard
        value={
          {
            id: "rate",
            kind: "value",
            name: "Growth rate",
            raw: "0.05",
            dataType: "number",
          } as ValueObject
        }
        computed={{ raw: "0.12", scenarioId: "upside", scenarioName: "Upside" }}
        formula="`Growth rate`"
        onOperation={onOperation}
      />
    );

    const field = screen.getByDisplayValue("0.12") as HTMLInputElement;
    expect(screen.getByText("number · 1 value · Upside")).toBeTruthy();

    await userEvent.clear(field);
    await userEvent.type(field, "0.2");
    fireEvent.blur(field);
    expect(onOperation).toHaveBeenCalledWith({
      type: "setScenarioValue",
      scenarioId: "upside",
      valueId: "rate",
      raw: "0.2",
    });
  });

  it("drags a qualified Container value through the shared drop lane", () => {
    let dropped: VectorDrag | null = null;
    render(
      <>
        <ValueCard
          value={
            {
              id: "rate",
              kind: "value",
              name: "Rate",
              raw: "0.08",
              value: 0.08,
              dataType: "number",
            } as ValueObject
          }
          formula="`Variables`.`Rate`"
          onOperation={vi.fn()}
        />
        <div
          className="canvas-viewport"
          aria-label="Canvas drop target"
          onDrop={(event) => {
            dropped = readVectorDrag(event.dataTransfer);
          }}
        />
      </>
    );
    const target = screen.getByLabelText("Canvas drop target");
    Object.defineProperty(document, "elementFromPoint", {
      configurable: true,
      value: vi.fn(() => target),
    });

    fireEvent.pointerDown(screen.getByText("number · 1 value"), {
      button: 0,
      clientX: 10,
      clientY: 10,
    });
    window.dispatchEvent(pointerEvent("pointermove", 20, 20));
    window.dispatchEvent(pointerEvent("pointerup", 20, 20));

    expect(dropped).toEqual({
      objectId: "rate",
      formula: "`Variables`.`Rate`",
      name: "Rate",
      length: 1,
    });
  });
});

describe("SeriesCard", () => {
  it("wraps vector values at rest and edits them in one paste surface", async () => {
    const onOperation = vi.fn<OperationHandler>(async () => null);
    const { container } = render(
      <SeriesCard
        series={{
          kind: "series",
          id: "scenario",
          name: "Scenario",
          dataType: "string",
          values: ["Base", "Upside", "Downside"],
        }}
        formula="`Scenario vectors`.`Scenario`"
        onOperation={onOperation}
      />
    );

    const preview = screen.getByRole("button", { name: "Edit Scenario values" });
    expect(preview.classList.contains("series-values-preview")).toBe(true);
    expect(container.querySelectorAll(".series-value")).toHaveLength(3);
    expect(screen.queryByRole("textbox", { name: "Scenario values" })).toBeNull();

    await userEvent.click(preview);
    const editor = screen.getByRole("textbox", { name: "Scenario values" });
    expect((editor as HTMLTextAreaElement).value).toBe("Base\nUpside\nDownside");

    fireEvent.change(editor, { target: { value: "Base, Upside" } });
    fireEvent.blur(editor);
    expect(onOperation).toHaveBeenCalledWith({
      type: "setSeries",
      objectId: "scenario",
      values: "Base, Upside",
    });
  });
});
