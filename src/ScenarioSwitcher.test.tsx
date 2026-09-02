// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { ScenarioSwitcher } from "./ScenarioSwitcher";
import type { Scenario } from "./lib/types";

afterEach(cleanup);

const scenarios: Scenario[] = [
  { id: "upside", name: "Upside", values: {} },
  { id: "downside", name: "Downside", values: {} },
];

it("draws nothing until the document has a scenario to switch to", () => {
  const { container } = render(
    <ScenarioSwitcher scenarios={[]} activeScenario={null} onOperation={vi.fn()} />
  );
  expect(container.innerHTML).toBe("");
});

it("lists Base plus every scenario and shows which is in force", () => {
  render(
    <ScenarioSwitcher
      scenarios={scenarios}
      activeScenario="downside"
      onOperation={vi.fn()}
    />
  );
  const select = screen.getByLabelText("Scenario") as HTMLSelectElement;
  expect([...select.options].map((option) => option.textContent)).toEqual([
    "Base",
    "Upside",
    "Downside",
  ]);
  expect(select.value).toBe("downside");
});

it("activates the chosen scenario, and the base as the empty option", async () => {
  const onOperation = vi.fn().mockResolvedValue(null);
  render(
    <ScenarioSwitcher
      scenarios={scenarios}
      activeScenario={null}
      onOperation={onOperation}
    />
  );
  await userEvent.selectOptions(screen.getByLabelText("Scenario"), "upside");
  expect(onOperation).toHaveBeenCalledWith({
    type: "activateScenario",
    scenarioId: "upside",
  });

  cleanup();
  onOperation.mockClear();
  render(
    <ScenarioSwitcher
      scenarios={scenarios}
      activeScenario="upside"
      onOperation={onOperation}
    />
  );
  await userEvent.selectOptions(screen.getByLabelText("Scenario"), "");
  expect(onOperation).toHaveBeenCalledWith({
    type: "activateScenario",
    scenarioId: null,
  });
});
