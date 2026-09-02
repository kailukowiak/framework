import type { StepDraft } from "./lib/pipelineSteps";
import { formulaToken } from "./lib/formulaReferences";
import type { Column } from "./lib/types";

/** A valid-looking header filter whose example value is ready to replace. */
export function columnFilterDraft(column: Pick<Column, "name" | "dataType">) {
  const left = `${formulaToken(column.name)} == `;
  const value =
    column.dataType === "string" || column.dataType === "categorical"
      ? '"value"'
      : column.dataType === "boolean"
        ? "True"
        : column.dataType === "date"
          ? "date(2026, 1, 1)"
          : "0";
  return {
    formula: `${left}${value}`,
    focusSelection: {
      start: left.length + (value.startsWith('"') ? 1 : 0),
      end: left.length + value.length - (value.endsWith('"') ? 1 : 0),
    },
  };
}

/** Append to a trailing filter when doing so preserves the chain's meaning. */
export function appendColumnFilter(
  steps: StepDraft[],
  column: Pick<Column, "name" | "dataType">,
  focusToken: number
): StepDraft[] {
  const draft = {
    id: crypto.randomUUID(),
    ...columnFilterDraft(column),
    focusToken,
  };
  const last = steps.at(-1);
  if (last?.kind === "filter")
    return [...steps.slice(0, -1), { ...last, predicates: [...last.predicates, draft] }];
  return [
    ...steps,
    {
      id: crypto.randomUUID(),
      kind: "filter",
      predicates: [draft],
      matchAll: true,
    },
  ];
}
