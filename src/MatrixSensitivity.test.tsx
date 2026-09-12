// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { CalculationMatrixCanvasCard } from "./CalculationMatrixCard";
import { ParameterInputsProvider } from "./ParameterInputs";
import { ActiveFormulaEditorProvider } from "./ActiveFormulaEditor";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it("edits only the existing matrix axis binding through the canonical operation", async () => {
  const document = fixtures.matrixSensitivity;
  const matrix = objectNamed(document, "calculationMatrix", "Sensitivity");
  const onOperation = vi.fn(async () => null);
  render(<ActiveFormulaEditorProvider><ParameterInputsProvider document={document} onOperation={onOperation}>
    <CalculationMatrixCanvasCard matrix={matrix} computed={document.computedCalculationMatrices[matrix.id]}
      objects={document.objects} computedFrames={document.computedFrames} formulaFunctions={document.formulaFunctions} onOperation={onOperation} />
  </ParameterInputsProvider></ActiveFormulaEditorProvider>);
  const input = screen.getByRole("combobox", { name: "Rows · Rate input" }) as HTMLSelectElement;
  expect(input.value).toBe(matrix.rows[0].targetId);
  expect(screen.getByRole("option", { name: "Vary growth" })).toBeTruthy();
  fireEvent.change(input, { target: { value: "" } });
  await waitFor(() => expect(onOperation).toHaveBeenCalledWith({ type: "setCalculationMatrix", objectId: matrix.id,
    rows: [{ id: matrix.rows[0].id, name: "Rate", formula: "[0.1,0.3]", targetId: undefined }], columns: [], body: matrix.body.source,
  }, { inlineError: true }));
});
