import { formattedFormula } from "../PipelineFormulaFormatting";
import { formulaToken } from "./formulaReferences";
import { stepsFromRendered } from "./pipelineChainEdits";
import { columnsBeforeStep, stepInput } from "./pipelineSteps";
import type { Column, ComputedFrame, FrameObject, FrameStepInput } from "./types";

export const READ_TYPES = ["string", "integer", "number", "boolean", "date"] as const;
export type ReadType = typeof READ_TYPES[number];

/** Type choices are ordinary Polars casts immediately after the input. Keep
 * them visible in Wrangle, editable by any formula surface, and undoable via
 * the existing pipeline operation. Only a step made entirely of same-column
 * casts can be edited here: folding into arbitrary calculations would change
 * which schema those calculations read. Leading identity projections belong
 * to linked frames and must remain ahead of these choices. */
export function readColumnTypes(frame: FrameObject, computed: ComputedFrame, input: Column[]) {
  const drafts = stepsFromRendered(computed.steps ?? [], frame, input);
  const offset = computed.passThroughSteps ?? 0;
  const columns = columnsBeforeStep(input, drafts, offset);
  const steps = drafts.map(stepInput);
  const first = steps[offset];
  const overrides = first?.kind === "withColumns" && first.columns.length > 0 &&
    first.columns.every((expression) => {
      const column = columns.find((candidate) => candidate.id === expression.outputColumnId);
      return column && READ_TYPES.some((type) => expression.formula === cast(column.name, type));
    }) ? first.columns : [];
  const types = new Map(overrides.map((expression) => [expression.outputColumnId,
    READ_TYPES.find((type) => expression.formula === cast(
      columns.find((column) => column.id === expression.outputColumnId)!.name, type))!]));
  return {
    columns, types,
    change(columnId: string, type: ReadType | ""): FrameStepInput[] {
      const column = columns.find((candidate) => candidate.id === columnId);
      if (!column) throw new Error("Select a source column first.");
      const next = overrides.filter((expression) => expression.outputColumnId !== columnId);
      if (type) next.push({ outputColumnId: column.id, name: column.name, formula: cast(column.name, type) });
      return [...steps.slice(0, offset),
        ...(next.length ? [{ kind: "withColumns" as const, columns: next }] : []),
        ...steps.slice(offset + (overrides.length ? 1 : 0))];
    },
  };
}

/** Chain formatting is the textual identity the engine echoes back, so author
 * the cast in that form: an unformatted spelling reseeds the chain drafts on
 * the next save and would not match its own round trip here. */
function cast(name: string, type: ReadType) {
  return formattedFormula(`${formulaToken(name)}.cast("${type}")`);
}
