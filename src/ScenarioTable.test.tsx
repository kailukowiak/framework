// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { ScenarioTable } from "./ScenarioTable";
import type { Scenario } from "./lib/types";

afterEach(cleanup);

const scenarios: Scenario[] = [
  { id: "upside", name: "Upside", values: { rate: "0.12" } },
  { id: "downside", name: "Downside", values: {} },
];

function mount(onOperation = vi.fn().mockResolvedValue(null)) {
  render(
    <ScenarioTable
      valueId="rate"
      valueRaw="0.05"
      scenarios={scenarios}
      onOperation={onOperation}
    />
  );
  return onOperation;
}

const field = (label: string) =>
  screen.getByLabelText(label) as HTMLInputElement;

it("shows what each scenario says, and the base as the placeholder where it says nothing", () => {
  mount();
  expect(field("Upside value").value).toBe("0.12");
  expect(field("Downside value").value).toBe("");
  expect(field("Downside value").placeholder).toBe("0.05");
});

it("commits an override on blur and clears it when the cell is emptied", async () => {
  const onOperation = mount();
  await userEvent.clear(screen.getByLabelText("Downside value"));
  await userEvent.type(screen.getByLabelText("Downside value"), "0.01");
  await userEvent.tab();
  expect(onOperation).toHaveBeenCalledWith(
    {
      type: "setScenarioValue",
      scenarioId: "downside",
      valueId: "rate",
      raw: "0.01",
    },
    { inlineError: true }
  );

  onOperation.mockClear();
  await userEvent.clear(screen.getByLabelText("Upside value"));
  await userEvent.tab();
  expect(onOperation).toHaveBeenCalledWith(
    { type: "setScenarioValue", scenarioId: "upside", valueId: "rate", raw: null },
    { inlineError: true }
  );
});

it("renames from the name cell and creates from the empty last row", async () => {
  const onOperation = mount();
  const name = screen.getByLabelText("Scenario name: Upside");
  await userEvent.clear(name);
  await userEvent.type(name, "Bull case{Enter}");
  expect(onOperation).toHaveBeenCalledWith(
    { type: "renameScenario", scenarioId: "upside", name: "Bull case" },
    { inlineError: true }
  );

  onOperation.mockClear();
  await userEvent.type(screen.getByLabelText("New scenario"), "Stress{Enter}");
  expect(onOperation).toHaveBeenCalledWith(
    { type: "addScenario", name: "Stress" },
    { inlineError: true }
  );
  expect(field("New scenario").value).toBe("");
});

it("removes a scenario when its emptied name cell takes a Delete", async () => {
  const onOperation = mount();
  const name = screen.getByLabelText("Scenario name: Downside");
  await userEvent.clear(name);
  await userEvent.type(name, "{Delete}");
  expect(onOperation).toHaveBeenCalledWith(
    { type: "removeScenario", scenarioId: "downside" },
    { inlineError: true }
  );
  // Rule 4: the row is its own delete affordance — no per-row buttons.
  expect(screen.queryByRole("button")).toBeNull();
});

it("reports a refused override as text under the table", async () => {
  const onOperation = vi
    .fn()
    .mockResolvedValue("'soon' is not a valid Number for value 'Growth rate'");
  mount(onOperation);
  await userEvent.type(screen.getByLabelText("Downside value"), "soon");
  await userEvent.tab();
  expect(
    await screen.findByText(
      "'soon' is not a valid Number for value 'Growth rate'"
    )
  ).toBeTruthy();
});
