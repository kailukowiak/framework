import type { OperationHandler } from "./lib/handlers";
import type { Scenario } from "./lib/types";

/**
 * Which set of assumptions the whole document is being read through.
 *
 * One select, in the canvas corner beside the other readouts, and absent
 * entirely until the document has a scenario — a workbook with one set of
 * numbers has nothing to switch between, and a permanently visible control
 * that says "Base" and does nothing is chrome by the definition in
 * AGENTS.md rule 4.
 *
 * "Base" is the empty option rather than a scenario object: the base is what
 * the value cards themselves hold, so it cannot be renamed, deleted, or
 * disagreed with.
 */
export function ScenarioSwitcher({
  scenarios,
  activeScenario,
  onOperation,
}: {
  scenarios: Scenario[];
  activeScenario: string | null;
  onOperation: OperationHandler;
}) {
  if (scenarios.length === 0) return null;
  return (
    <label className="scenario-switcher">
      Scenario
      <select
        aria-label="Scenario"
        value={activeScenario ?? ""}
        onChange={(event) =>
          void onOperation({
            type: "activateScenario",
            scenarioId: event.target.value === "" ? null : event.target.value,
          })
        }
      >
        <option value="">Base</option>
        {scenarios.map((scenario) => (
          <option key={scenario.id} value={scenario.id}>
            {scenario.name}
          </option>
        ))}
      </select>
    </label>
  );
}
