import { Braces } from "lucide-react";
import { Field } from "./Field";
import { ScenarioTable } from "./ScenarioTable";
import type { OperationHandler } from "./lib/handlers";
import type { ValueObject, Scenario } from "./lib/types";

export function ValueInspector({
  value,
  scenarios,
  onOperation,
}: {
  value: ValueObject;
  scenarios: Scenario[];
  onOperation: OperationHandler;
}) {
  return (
    <div className="inspector-content">
      <Field
        label="Name"
        initial={value.name}
        onCommit={(name) =>
          onOperation({ type: "renameObject", objectId: value.id, name })
        }
      />
      <Field
        label="Value"
        initial={value.raw}
        onCommit={(raw) => onOperation({ type: "setValue", objectId: value.id, raw })}
      />
      {/* The base above, and what each scenario says instead below it — one
          grid, so the assumption and its alternatives are read together. */}
      <label className="inspector-field">
        Scenarios
        <ScenarioTable
          valueId={value.id}
          valueRaw={value.raw}
          scenarios={scenarios}
          onOperation={onOperation}
        />
      </label>
      <div className="info-panel">
        <Braces size={16} />
        <p>
          This is a standalone scalar object. Frame formulas reference its stable ID, so
          renaming it is safe.
        </p>
      </div>
    </div>
  );
}
