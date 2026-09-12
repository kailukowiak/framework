// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ParameterAnswer, ParameterInputsProvider } from "./ParameterInputs";
import { fixtures } from "./test/support";

afterEach(cleanup);
function mount() {
  const onOperation = vi.fn().mockResolvedValue(null);
  render(<ParameterInputsProvider document={fixtures.parameters} onOperation={onOperation}>
    {fixtures.parameters.parameterInputs!.map((input) => <ParameterAnswer key={input.id} id={input.id}>fallback</ParameterAnswer>)}
    <ParameterAnswer id="not-a-control">ordinary answer</ParameterAnswer>
  </ParameterInputsProvider>);
  return onOperation;
}

describe("constructor-backed variable inputs", () => {
  it("keeps focus and follows stored selection changes and undo", () => {
    const onOperation = vi.fn().mockResolvedValue(null);
    const content = (document: typeof fixtures.parameters) => <ParameterInputsProvider document={document} onOperation={onOperation}>
      {document.parameterInputs!.map((input) => <ParameterAnswer key={input.id} id={input.id}>fallback</ParameterAnswer>)}
    </ParameterInputsProvider>;
    const mounted = render(content(fixtures.parameters));
    const slider = screen.getByLabelText("growth slider");
    slider.focus();
    mounted.rerender(content(fixtures.parametersSelected));
    expect(document.activeElement).toBe(slider);
    expect(screen.getByLabelText("growth value")).toHaveProperty("value", "0.4");
    expect(screen.getByLabelText("cutoff date")).toHaveProperty("value", "2027-01-01");
    mounted.rerender(content(fixtures.parameters));
    expect(document.activeElement).toBe(slider);
    expect(screen.getByLabelText("growth value")).toHaveProperty("value", "0.2");
    expect(screen.getByLabelText("cutoff date")).toHaveProperty("value", "2026-12-31");
  });

  it("commits one edit per slider gesture, not each intermediate position", async () => {
    const onOperation = mount();
    const slider = screen.getByRole("slider", { name: "growth slider" });
    fireEvent.change(slider, { target: { value: "0.3" } });
    fireEvent.change(slider, { target: { value: "0.4" } });
    expect(onOperation).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider);
    fireEvent.blur(slider);
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(1));
    expect(onOperation).toHaveBeenCalledWith({ type: "setParameterValue", objectId: fixtures.parameters.parameterInputs![0].id, value: { type: "number", value: 0.4 } }, { inlineError: true });
  });

  it("emits typed choices, dates and exact numeric values without replacing source", async () => {
    const onOperation = mount();
    fireEvent.change(screen.getByLabelText("region choice"), { target: { value: "1" } });
    const date = screen.getByLabelText("cutoff date");
    fireEvent.change(date, { target: { value: "2027-01-01" } });
    fireEvent.blur(date);
    const number = screen.getByLabelText("growth value");
    fireEvent.change(number, { target: { value: "0.25" } });
    fireEvent.blur(number);
    await waitFor(() => expect(onOperation).toHaveBeenCalledTimes(3));
    expect(onOperation.mock.calls.map(([operation]) => operation.value)).toEqual([
      { type: "string", value: "South" }, { type: "date", value: "2027-01-01" }, { type: "number", value: 0.25 },
    ]);
    expect(screen.getByText("ordinary answer")).toBeTruthy();
  });

  it("shows operation errors without inventing an accepted value", async () => {
    const onOperation = vi.fn().mockResolvedValue("Outside range");
    const input = fixtures.parameters.parameterInputs![0];
    render(<ParameterInputsProvider document={fixtures.parameters} onOperation={onOperation}><ParameterAnswer id={input.id}>fallback</ParameterAnswer></ParameterInputsProvider>);
    const number = screen.getByLabelText("growth value");
    fireEvent.change(number, { target: { value: "2" } });
    fireEvent.blur(number);
    expect(await screen.findByRole("status")).toHaveProperty("textContent", "Outside range");
  });
});
