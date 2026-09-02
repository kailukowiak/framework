import { useState } from "react";
import type { OperationHandler } from "./lib/handlers";
import type { Scenario } from "./lib/types";

/**
 * What one value is worth in each scenario, as one editable grid.
 *
 * Deliberately a table and not a stack of labelled rows with buttons: the
 * whole point of scenarios is holding Base, Upside and Downside in view at
 * once, and a card that spends a delete button and a heading per scenario
 * would hold three of them and be full (AGENTS.md rules 1 and 4). So every
 * cell here is the editable surface for the thing it displays — the name
 * cell renames, the value cell overrides, the empty last row creates — and
 * there are no controls beside them.
 *
 * A blank value cell is not an override of "nothing": it means this
 * scenario agrees with the base, which is why clearing the field clears the
 * override rather than writing an empty literal.
 */
export function ScenarioTable({
  valueId,
  valueRaw,
  scenarios,
  onOperation,
}: {
  valueId: string;
  /** The number on the card, shown as the placeholder each scenario falls
      back to when it says nothing. */
  valueRaw: string;
  scenarios: Scenario[];
  onOperation: OperationHandler;
}) {
  const [error, setError] = useState<string | null>(null);
  const run = (operation: Parameters<OperationHandler>[0]) =>
    void onOperation(operation, { inlineError: true }).then(setError);

  return (
    <div className="scenario-table">
      <table>
        <thead>
          <tr>
            <th scope="col">Scenario</th>
            <th scope="col">Value</th>
          </tr>
        </thead>
        <tbody>
          {scenarios.map((scenario) => (
            <tr key={scenario.id}>
              <td>
                <input
                  aria-label={`Scenario name: ${scenario.name}`}
                  defaultValue={scenario.name}
                  key={scenario.name}
                  onBlur={(event) => {
                    if (event.target.value !== scenario.name)
                      run({
                        type: "renameScenario",
                        scenarioId: scenario.id,
                        name: event.target.value,
                      });
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") event.currentTarget.blur();
                    // A row emptied and then deleted is the same gesture a
                    // spreadsheet uses to remove a row: clear it, press
                    // Delete. No per-row × — see AGENTS.md rule 4.
                    if (
                      (event.key === "Delete" || event.key === "Backspace") &&
                      event.currentTarget.value === ""
                    ) {
                      event.preventDefault();
                      run({ type: "removeScenario", scenarioId: scenario.id });
                    }
                  }}
                />
              </td>
              <td>
                <input
                  aria-label={`${scenario.name} value`}
                  className="scenario-value"
                  defaultValue={scenario.values[valueId] ?? ""}
                  key={scenario.values[valueId] ?? ""}
                  placeholder={valueRaw}
                  onBlur={(event) => {
                    const raw = event.target.value;
                    if (raw !== (scenario.values[valueId] ?? ""))
                      run({
                        type: "setScenarioValue",
                        scenarioId: scenario.id,
                        valueId,
                        raw: raw === "" ? null : raw,
                      });
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") event.currentTarget.blur();
                  }}
                />
              </td>
            </tr>
          ))}
          <tr>
            <td>
              <input
                aria-label="New scenario"
                placeholder="New scenario"
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  const name = event.currentTarget.value;
                  if (name.trim() === "") return;
                  event.currentTarget.value = "";
                  run({ type: "addScenario", name });
                }}
              />
            </td>
            <td />
          </tr>
        </tbody>
      </table>
      {error && <span className="formula-error-line">{error}</span>}
    </div>
  );
}
