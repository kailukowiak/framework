/**
 * What the three chain relationships in Combine have to work out before they
 * can save: the chain a frame already carries, how many rows each side has,
 * whether a paired list fits, and — for a stack — which columns will line up
 * with which.
 *
 * It lives beside the pipeline helpers rather than inside the dialog because
 * every one of these answers is a fact about the document, not about a form.
 * The dialog draws them; the tests read them here without mounting anything.
 */
import { frameColumnVector } from "../useFrameCardInteraction";
import { uniqueColumnName } from "../PipelineColumnNames";
import { formattedFormula } from "../PipelineFormulaFormatting";
import { stepsFromRendered } from "./pipelineChainEdits";
import { mintColumnId, stepInput } from "./pipelineSteps";
import type {
  Column,
  ComputedFrame,
  DataObject,
  FrameObject,
  FrameStepInput,
  ZipFill,
} from "./types";

/**
 * The schema this frame's chain starts from — the same question
 * `FrameInspector` answers for the Wrangle editor, asked again here because
 * appending a step means rebuilding the steps above it, and those are parsed
 * against this schema. A derived frame reads the frame it derives from; a
 * frame with its own data reads `baseColumns` once a chain exists, since by
 * then `columns` is the chain's *output*.
 */
export function chainInputColumns(
  frame: FrameObject,
  objects: DataObject[]
): Column[] {
  const derivation = frame.derivation;
  if (derivation && !derivation.join) {
    const source = objects.find(
      (candidate): candidate is FrameObject =>
        candidate.kind === "frame" && candidate.id === derivation.sourceFrameId
    );
    if (source) return source.columns;
  }
  return frame.baseColumns?.length ? frame.baseColumns : frame.columns;
}

/**
 * The chain as it stands, rebuilt from what the engine rendered back.
 *
 * `setFramePipeline` replaces the whole chain, so appending a step means
 * resending every step above it. Rebuilding through the editor's own
 * `stepsFromRendered` / `stepInput` pair is what keeps that resend faithful:
 * the formulas go back in the formatted spelling the engine echoes, so the
 * round trip reconciles instead of reseeding.
 */
export function existingChainSteps(
  frame: FrameObject,
  computed: ComputedFrame | undefined,
  objects: DataObject[]
): FrameStepInput[] {
  return stepsFromRendered(
    computed?.steps ?? [],
    frame,
    chainInputColumns(frame, objects)
  ).map(stepInput);
}

/**
 * How many rows a frame has, when that is knowable without running a query.
 * `undefined` means paged data whose count would cost a full pass — the
 * previews say so rather than guessing, and no refusal is raised on a number
 * nobody has.
 */
export function frameRowCount(
  frame: FrameObject,
  computed: ComputedFrame | undefined
): number | undefined {
  if (typeof computed?.totalRows === "number") return computed.totalRows;
  return computed?.paged ? undefined : frame.rows.length;
}

/**
 * Why these two row counts cannot meet under the chosen fill, in one line,
 * or null when they can. `exact` wants one value per row; `repeat` tiles the
 * list, so the frame's rows must be a whole multiple of the list's length.
 *
 * The core refuses the same two cases with a message naming both counts.
 * Saying it here first is not duplication of the rule — it is the difference
 * between a disabled button that explains itself and a dialog that accepts a
 * click and then reports failure.
 */
export function pairFillRefusal(
  frameRows: number | undefined,
  otherRows: number | undefined,
  fill: ZipFill
): string | null {
  if (frameRows === undefined || otherRows === undefined) return null;
  if (otherRows === 0) return "The other table has no rows to pair.";
  if (fill === "exact")
    return frameRows === otherRows
      ? null
      : `Exact needs equal counts: ${frameRows.toLocaleString()} rows here, ${otherRows.toLocaleString()} there.`;
  return frameRows % otherRows === 0
    ? null
    : `Repeat needs ${frameRows.toLocaleString()} to be a whole multiple of ${otherRows.toLocaleString()}.`;
}

/**
 * One `zipVector` step per column brought across, spelled the way the
 * header-onto-plus-edge drag spells it: the qualified live column
 * `` `Other`.`Column` ``, formatted, so the chain's saved text is the text
 * the engine echoes back.
 */
export function pairColumnSteps(
  other: FrameObject,
  columnIds: string[],
  fill: ZipFill,
  usedNames: string[],
  otherRows: number
): FrameStepInput[] {
  const used = [...usedNames];
  const steps: FrameStepInput[] = [];
  for (const columnId of columnIds) {
    const column = other.columns.find((candidate) => candidate.id === columnId);
    const vector = frameColumnVector(other, columnId, Math.max(otherRows, 1));
    if (!column || !vector) continue;
    const name = uniqueColumnName(column.name, used);
    used.push(name);
    steps.push({
      kind: "zipVector",
      outputColumnId: mintColumnId(name),
      name,
      vector: formattedFormula(vector.formula),
      fill,
    });
  }
  return steps;
}

/** One row of the stack preview: this frame's column and what fills it. */
export type StackPairing = {
  columnId: string;
  name: string;
  /** The name-matched column on the other side, or null for nulls. */
  sourceName: string | null;
  /** Matched by name but declaring a different type. */
  mismatch: boolean;
};

/**
 * What a union will do, worked out the way the core works it out: by name,
 * once, when the step is prepared. Columns only this frame has take nulls;
 * columns only the other frame has are dropped, and the preview names them
 * rather than letting them disappear quietly.
 */
export function stackMapping(
  targetColumns: Column[],
  sourceColumns: Column[]
): { pairs: StackPairing[]; notCarried: string[] } {
  const byName = new Map(sourceColumns.map((column) => [column.name, column]));
  const pairs = targetColumns.map((column) => {
    const match = byName.get(column.name);
    return {
      columnId: column.id,
      name: column.name,
      sourceName: match?.name ?? null,
      mismatch: Boolean(match && match.dataType !== column.dataType),
    };
  });
  const targetNames = new Set(targetColumns.map((column) => column.name));
  return {
    pairs,
    notCarried: sourceColumns
      .filter((column) => !targetNames.has(column.name))
      .map((column) => column.name),
  };
}
