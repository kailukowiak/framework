import type { StepDraft } from "./PipelineEditor";
import { parseRecurrenceFormula } from "./RecurrenceDialog";
import { formatFormulaChains } from "./lib/formulaFormatting";
import type { RenderedFrameStep } from "./lib/types";

export const formattedFormula = (formula: string) =>
  formatFormulaChains(formula).source;

export function recurrenceDraft(
  step: Extract<RenderedFrameStep, { kind: "withColumns" }>,
  id: string,
  nameOf: (columnId: string, fallback: string) => string
): StepDraft | null {
  if (step.columns.length !== 1) return null;
  const recurrent = parseRecurrenceFormula(step.columns[0].formula);
  if (!recurrent) return null;
  const column = step.columns[0];
  return {
    id,
    kind: "recurrence",
    outputColumnId: column.outputColumnId,
    name: nameOf(column.outputColumnId, "Column"),
    seed: formattedFormula(recurrent.seed),
    formula: formattedFormula(recurrent.next),
    partitionName: recurrent.partitionName,
  };
}

/** Every saved chain returns to one deterministic, multiline authoring form. */
export function formatPipelineFormulas(step: StepDraft): StepDraft {
  switch (step.kind) {
    case "filter":
      return {
        ...step,
        predicates: step.predicates.map((item) => ({
          ...item,
          formula: formattedFormula(item.formula),
        })),
      };
    case "withColumns":
      return {
        ...step,
        columns: step.columns.map((item) => ({
          ...item,
          formula: formattedFormula(item.formula),
        })),
      };
    case "recurrence":
      return {
        ...step,
        seed: formattedFormula(step.seed),
        formula: formattedFormula(step.formula),
      };
    case "summarize":
      return {
        ...step,
        groupKeys: step.groupKeys.map((item) => ({
          ...item,
          formula: formattedFormula(item.formula),
        })),
        aggregates: step.aggregates.map((item) => ({
          ...item,
          formula: formattedFormula(item.formula),
        })),
      };
    default:
      return step;
  }
}
