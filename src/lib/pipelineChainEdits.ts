import {
  draftName,
  nextBlankColumnName,
  uniqueColumnName,
  type NamedDraft,
} from "../PipelineColumnNames";
import { formattedFormula, recurrenceDraft } from "../PipelineFormulaFormatting";
import { vectorDraftFromRendered } from "../PipelineVectorSteps";
import { parseRecurrenceFormula } from "../RecurrenceDialog";
import { siblingReferenceInFormula } from "./formulaPicking";
import { formulaToken } from "./formulaReferences";
import {
  columnsBeforeStep,
  mintColumnId,
  namedDraft,
  type StepDraft,
} from "./pipelineSteps";
import type { Column, FrameObject, RenderedFrameStep } from "./types";

/**
 * The formula a column starts life with, wherever it is added from — the
 * header's *Add calculated column* and Wrangle's *Add or replace column*
 * open the identical line, `` `Column 1` = None.cast("number") ``, selected
 * whole so the first keystroke replaces all of it.
 *
 * A typed null is blank in every row but still gives the query plan a
 * stable dtype, so the column can render immediately and the formula can be
 * replaced in place without a second creation path.
 *
 * `None`, not `null`: the parser takes either, but the engine renders the
 * saved expression back canonically — `Expr::Null` prints as `None` — and
 * the document sync compares that echo against this draft textually. A
 * placeholder spelled `null` can never reconcile with its own save, so the
 * round trip reseeded the step list and severed the formula session the
 * gesture had just opened.
 */
export const BLANK_CALCULATION = 'None.cast("number")';

/**
 * The context-menu gesture creates something real before asking for its
 * expression: see `BLANK_CALCULATION`.
 */
export function appendBlankCalculatedColumn(
  steps: StepDraft[],
  focusToken: number,
  afterColumnId?: string,
  visibleColumns: Array<{ id: string; name: string }> = [],
  protectedStepCount = 0,
  anchorRowIndex?: number
): StepDraft[] {
  const fallbackName = nextBlankColumnName(visibleColumns.map((column) => column.name));
  const outputColumnId = mintColumnId(fallbackName);
  const column = {
    ...namedDraft(fallbackName, BLANK_CALCULATION),
    outputColumnId,
    focusToken,
    anchorRowIndex,
  };
  const visibleColumnIds = visibleColumns.map((column) => column.id);
  const afterIndex = afterColumnId ? visibleColumnIds.indexOf(afterColumnId) : -1;
  const ordered = [...visibleColumnIds];
  ordered.splice(afterIndex < 0 ? ordered.length : afterIndex + 1, 0, outputColumnId);

  // A Choose columns step only changes visibility and placement. It does
  // not make a later calculation depend on an intermediate result, so it
  // should not force every context-menu insertion into another Add columns
  // block. Reuse the last authored calculation while only projections sit
  // below it, and teach the final projection about the new output. A filter,
  // sort, summarize, or reshape remains a real boundary: moving a formula
  // above one would change what it reads or which rows it runs over.
  const finalIsProjection = steps.at(-1)?.kind === "select";
  const reusableIndex = steps.length - (finalIsProjection ? 2 : 1);
  if (
    reusableIndex >= protectedStepCount &&
    steps[reusableIndex].kind === "withColumns"
  ) {
    const merged = steps.map((step, index) => {
      if (index === reusableIndex && step.kind === "withColumns") {
        return { ...step, columns: [...step.columns, column] };
      }
      if (index === steps.length - 1 && step.kind === "select") {
        return { ...step, columnIds: ordered };
      }
      return step;
    });
    if (
      !finalIsProjection &&
      afterIndex >= 0 &&
      afterIndex < visibleColumnIds.length - 1
    ) {
      return [
        ...merged,
        {
          id: crypto.randomUUID(),
          kind: "select",
          columnIds: ordered,
          mode: "placement",
        },
      ];
    }
    return merged;
  }

  const withColumn: StepDraft = {
    id: crypto.randomUUID(),
    kind: "withColumns",
    columns: [column],
  };
  if (afterIndex < 0 || afterIndex === visibleColumnIds.length - 1) {
    return [...steps, withColumn];
  }
  return [
    ...steps,
    withColumn,
    {
      id: crypto.randomUUID(),
      kind: "select",
      columnIds: ordered,
      mode: "placement",
    },
  ];
}

/**
 * Replace one visible column without changing its identity or position.
 * Polars' `with_columns` overwrites an existing physical name, and physical
 * names are our column ids, so reusing the id is the important part. The
 * editable name is repeated only to make the pipeline read like a formula:
 * `` `Amount` = `Amount`.cast("integer") ``.
 *
 * This deliberately gets its own step. Combining it with the preceding
 * calculation would make both expressions read the same input schema, which
 * changes the meaning of successive in-place edits.
 */
export function appendInPlaceColumnTransformation(
  steps: StepDraft[],
  column: { id: string; name: string },
  formula: string,
  opening: ColumnOpening = {}
): StepDraft[] {
  const { focusToken, focusAtEnd = true, anchorRowIndex } = opening;
  return [
    ...steps,
    {
      id: crypto.randomUUID(),
      kind: "withColumns",
      columns: [
        {
          ...namedDraft(column.name, formula),
          outputColumnId: column.id,
          focusToken,
          focusAtEnd: focusToken === undefined ? undefined : focusAtEnd,
          anchorRowIndex,
        },
      ],
    },
  ];
}

/**
 * How the formula this appends opens for editing: which session focuses it,
 * where the caret lands, and the row the gesture started on. The anchor is
 * the part that is easy to lose — without it, every reference pointed at
 * while the formula is open comes out as the whole column, silently dropping
 * the `.shift(n)` the clicked row was asking for.
 */
export type ColumnOpening = {
  focusToken?: number;
  focusAtEnd?: boolean;
  anchorRowIndex?: number;
};

/** Add a declared order immediately before a row-position formula needs it. */
export function appendOrderedColumnTransformation(
  steps: StepDraft[],
  column: { id: string; name: string },
  formula: string,
  orderByColumnId?: string,
  opening: ColumnOpening = {}
): StepDraft[] {
  const ordered =
    !orderByColumnId || steps.some((step) => step.kind === "sort")
      ? steps
      : [
          ...steps,
          {
            id: crypto.randomUUID(),
            kind: "sort" as const,
            keys: [
              {
                id: crypto.randomUUID(),
                columnId: orderByColumnId,
                descending: false,
              },
            ],
          },
        ];
  const { focusToken, focusAtEnd = true, anchorRowIndex } = opening;
  const recurrence = parseRecurrenceFormula(formula);
  if (!recurrence)
    return appendInPlaceColumnTransformation(ordered, column, formula, opening);
  return [
    ...ordered,
    {
      id: crypto.randomUUID(),
      kind: "recurrence",
      outputColumnId: column.id,
      name: column.name,
      seed: recurrence.seed,
      formula: recurrence.next,
      partitionName: recurrence.partitionName,
      focusToken,
      focusAtEnd: focusToken === undefined ? undefined : focusAtEnd,
      anchorRowIndex,
    },
  ];
}

/**
 * Give a calculation that reads its own step's sibling a step of its own.
 *
 * `with_columns` evaluates every formula in the block against the frame as
 * it stood before the step, so a name written beside this one is not there
 * yet. The person's sentence is right and the arrangement is wrong, so the
 * commit fixes the arrangement: the committing column moves into a new
 * "Add or replace columns" step inserted directly after this one, where the
 * sibling it reads has already been produced. Wrangle then shows the two
 * numbered steps, which is the explanation.
 *
 * Anything else in the step that reads the moved column travels with it, in
 * order. Leaving such a reader behind would be worse than the situation it
 * came from: its reference would point at a column produced *below* it, an
 * unknown name rather than a sibling. A reader carried along may still have
 * a sibling problem of its own inside the new step — that is the same
 * refusal it already had, and the same Return fixes it.
 *
 * `visibleNames` are the columns arriving from above, and a name among them
 * is deliberately not a sibling reference at all: a step that replaces
 * `Amount` while another of its calculations reads `Amount` is reading the
 * upstream value, which is ordinary and must not be split apart.
 *
 * Returns null when nothing needs to move, so the caller can save the
 * chain it already has.
 */
export function splitSiblingReferenceStep(
  steps: StepDraft[],
  stepId: string,
  columnId: string,
  visibleNames: string[]
): StepDraft[] | null {
  const index = steps.findIndex((step) => step.id === stepId);
  const step = steps[index];
  if (!step || step.kind !== "withColumns") return null;
  const committing = step.columns.find((column) => column.id === columnId);
  if (!committing) return null;
  const moving = new Set([columnId]);
  const namesOf = (predicate: (columnId: string) => boolean) =>
    step.columns
      .filter((column) => predicate(column.id))
      .map((column) => ({ name: draftName(column) }));
  if (
    !siblingReferenceInFormula(
      committing.formula,
      namesOf((candidate) => candidate !== columnId),
      visibleNames
    )
  )
    return null;
  for (let grew = true; grew; ) {
    grew = false;
    for (const candidate of step.columns) {
      if (moving.has(candidate.id)) continue;
      if (
        !siblingReferenceInFormula(
          candidate.formula,
          namesOf((id) => moving.has(id)),
          visibleNames
        )
      )
        continue;
      moving.add(candidate.id);
      grew = true;
    }
  }
  const staying = step.columns.filter((column) => !moving.has(column.id));
  if (!staying.length) return null;
  return [
    ...steps.slice(0, index),
    { ...step, columns: staying },
    {
      id: crypto.randomUUID(),
      kind: "withColumns",
      columns: step.columns.filter((column) => moving.has(column.id)),
    },
    ...steps.slice(index + 1),
  ];
}

/** Move the shared editor to the final declaration that produces a column. */
export function focusExistingCalculatedColumn(
  steps: StepDraft[],
  columnId: string,
  focusToken: number,
  anchorRowIndex?: number
): StepDraft[] | null {
  let stepIndex = -1;
  for (let index = steps.length - 1; index >= 0; index -= 1) {
    const step = steps[index];
    if (
      (step.kind === "withColumns" &&
        step.columns.some((column) => column.outputColumnId === columnId)) ||
      (step.kind === "recurrence" && step.outputColumnId === columnId)
    ) {
      stepIndex = index;
      break;
    }
  }
  if (stepIndex < 0) return null;
  return steps.map((step, index) => {
    if (index !== stepIndex) return step;
    if (step.kind === "recurrence") {
      return {
        ...step,
        focusToken,
        focusAtEnd: false,
        anchorRowIndex,
      };
    }
    return step.kind === "withColumns"
      ? {
          ...step,
          columns: step.columns.map((column) =>
            column.outputColumnId === columnId
              ? {
                  ...column,
                  focusToken,
                  focusAtEnd: false,
                  anchorRowIndex,
                }
              : column
          ),
        }
      : step;
  });
}

/**
 * A grid-level delete on computed data is the compact spelling of unchecking
 * the column in a final Choose columns step. The source and every calculation
 * stay intact, so the choice is recoverable and nothing upstream has to be
 * rewritten merely because its output is no longer shown.
 */
export function hidePipelineColumn(
  steps: StepDraft[],
  columnId: string,
  visibleColumnIds: string[]
): StepDraft[] | null {
  const remaining = visibleColumnIds.filter((visible) => visible !== columnId);
  if (remaining.length === visibleColumnIds.length || remaining.length === 0)
    return null;
  const last = steps.at(-1);
  if (last?.kind === "select" && last.mode !== "rearrange") {
    return [
      ...steps.slice(0, -1),
      {
        ...last,
        mode: "delete",
        columnIds: last.columnIds.filter((visible) => visible !== columnId),
      },
    ];
  }
  return [
    ...steps,
    {
      id: crypto.randomUUID(),
      kind: "select",
      columnIds: remaining,
      mode: "delete",
    },
  ];
}

export function rearrangePipelineColumns(
  steps: StepDraft[],
  columnIds: string[]
): StepDraft[] {
  const last = steps.at(-1);
  if (last?.kind === "select" && last.mode !== "delete") {
    return [...steps.slice(0, -1), { ...last, columnIds, mode: "rearrange" }];
  }
  return [
    ...steps,
    {
      id: crypto.randomUUID(),
      kind: "select",
      columnIds,
      mode: "rearrange",
    },
  ];
}

/**
 * Resolve names in the same order the chain exposes them. Formula aliases
 * can collide just as typed names can, so normalization happens at save
 * time after both have had their say.
 */
export function normalizeCalculatedColumnNames(
  steps: StepDraft[],
  sourceColumns: Column[],
  passThroughSteps: number
): StepDraft[] {
  const normalized = [...steps];
  for (let index = passThroughSteps; index < normalized.length; index += 1) {
    const step = normalized[index];
    if (step.kind === "withColumns") {
      const used = columnsBeforeStep(sourceColumns, normalized, index).map(
        (column) => column.name
      );
      const columns = step.columns.map((column) => {
        const requested = draftName(column);
        const replaces = columnsBeforeStep(sourceColumns, normalized, index).some(
          (visible) =>
            visible.id === column.outputColumnId && visible.name === requested
        );
        const name = replaces ? requested : uniqueColumnName(requested, used);
        used.push(name);
        return name === requested ? column : { ...column, name };
      });
      normalized[index] = { ...step, columns };
    } else if (step.kind === "summarize") {
      const used: string[] = [];
      const normalize = (column: NamedDraft) => {
        const requested = draftName(column);
        const name = uniqueColumnName(requested, used);
        used.push(name);
        return name === requested ? column : { ...column, name };
      };
      normalized[index] = {
        ...step,
        groupKeys: step.groupKeys.map(normalize),
        aggregates: step.aggregates.map(normalize),
      };
    }
  }
  return normalized;
}

/**
 * Reads the saved chain back into drafts, taking names from the frame's
 * columns. Built as a fold rather than a map because the unpivot draft
 * writes its melt list as text, and the names in that text are the ones
 * the columns carry *now* — which only the steps already rebuilt can say.
 */
export function stepsFromRendered(
  rendered: RenderedFrameStep[],
  editingFrame: FrameObject,
  sourceColumns: Column[]
): StepDraft[] {
  const nameOf = (id: string, fallback: string) =>
    renderedOutputName(editingFrame, id, fallback);
  const drafts: StepDraft[] = [];
  for (const step of rendered) {
    const id = crypto.randomUUID();
    if (appendVectorDraft(step, id, sourceColumns, drafts)) continue;
    switch (step.kind) {
      case "filter":
        drafts.push({
          id,
          kind: "filter",
          predicates: step.predicates.map((formula) => ({
            id: crypto.randomUUID(),
            formula: formattedFormula(formula),
          })),
          matchAll: step.matchAll,
        });
        break;
      case "withColumns": {
        // A linked frame's hidden pass-through projection owns intermediate
        // output ids. Once a later summarize replaces the frame's declared
        // schema, those ids are no longer in `editingFrame.columns`; falling
        // back to "Column" here renames every hidden output on the next save
        // and makes the authored step unable to resolve the source names it
        // plainly shows. A bare projected reference carries its own exact
        // source name, so inherit that name while rebuilding the draft.
        const visible = columnsBeforeStep(sourceColumns, drafts, drafts.length);
        const recurrent = recurrenceDraft(step, id, nameOf);
        if (recurrent) {
          drafts.push(recurrent);
          break;
        }
        drafts.push({
          id,
          kind: "withColumns",
          columns: step.columns.map((column) => {
            const projected = column.formula
              .trim()
              .match(/^`((?:[^`]|``)*)`$/)?.[1]
              ?.replaceAll("``", "`");
            const inherited = visible.find(
              (candidate) => candidate.name === projected
            )?.name;
            return {
              id: crypto.randomUUID(),
              outputColumnId: column.outputColumnId,
              name: nameOf(column.outputColumnId, inherited ?? "Column"),
              formula: formattedFormula(column.formula),
              fallbackName: inherited ?? "Column",
            };
          }),
        });
        break;
      }
      case "select": {
        const available = columnsBeforeStep(sourceColumns, drafts, drafts.length).map(
          (column) => column.id
        );
        const keepsEveryColumn =
          available.length === step.columnIds.length &&
          available.every((columnId) => step.columnIds.includes(columnId));
        const followsCalculation = ["withColumns", "recurrence"].includes(
          drafts.at(-1)?.kind ?? ""
        );
        drafts.push({
          id,
          kind: "select",
          columnIds: step.columnIds,
          mode: !keepsEveryColumn
            ? "delete"
            : followsCalculation
            ? "placement"
            : "rearrange",
        });
        break;
      }
      case "summarize":
        drafts.push({
          id,
          kind: "summarize",
          groupKeys: step.groupKeys.map((column) => ({
            id: crypto.randomUUID(),
            outputColumnId: column.outputColumnId,
            name: nameOf(column.outputColumnId, "Group"),
            formula: formattedFormula(column.formula),
            fallbackName: "Group",
          })),
          aggregates: step.aggregates.map((column) => ({
            id: crypto.randomUUID(),
            outputColumnId: column.outputColumnId,
            name: nameOf(column.outputColumnId, "Aggregate"),
            formula: formattedFormula(column.formula),
            fallbackName: "Aggregate",
          })),
          maintainOrder: step.maintainOrder,
        });
        break;
      case "sort":
        drafts.push({
          id,
          kind: "sort",
          keys: step.keys.map((key) => ({ id: crypto.randomUUID(), ...key })),
        });
        break;
      case "union":
        drafts.push({ id, kind: "union", frameId: step.frameId });
        break;
      case "expand":
        drafts.push({ id, kind: "expand", frameId: step.frameId });
        break;
      case "pivot":
        drafts.push({
          id,
          kind: "pivot",
          namesColumnId: step.namesColumnId,
          valuesColumnId: step.valuesColumnId,
          aggregate: step.aggregate,
        });
        break;
      case "unpivot": {
        // The saved step holds ids, with the labels its rows carry frozen
        // at save time. The text offered for editing writes each column's
        // current name — the name the save will resolve against — falling
        // back to the frozen label for a column the walk cannot find.
        const visible = columnsBeforeStep(sourceColumns, drafts, drafts.length);
        drafts.push({
          id,
          kind: "unpivot",
          columns: step.columns
            .map((column) =>
              formulaToken(
                visible.find((candidate) => candidate.id === column.columnId)?.name ??
                  column.label
              )
            )
            .join(", "),
          nameColumnId: step.nameColumnId,
          nameColumnName: step.nameColumnName,
          valueColumnId: step.valueColumnId,
          valueColumnName: step.valueColumnName,
        });
        break;
      }
      case "comment":
        drafts.push({ id, kind: "comment", text: step.text });
        break;
      // A join builds its own columns and is edited from its join settings.
      case "join":
        break;
    }
  }
  return drafts;
}

function appendVectorDraft(
  step: RenderedFrameStep,
  id: string,
  sourceColumns: Column[],
  drafts: StepDraft[]
): boolean {
  const draft = vectorDraftFromRendered(
    step,
    id,
    columnsBeforeStep(sourceColumns, drafts, drafts.length)
  );
  if (!draft) return false;
  drafts.push(draft);
  return true;
}

function renderedOutputName(
  frame: FrameObject,
  outputColumnId: string,
  fallback: string
) {
  return frame.columns.find((column) => column.id === outputColumnId)?.name ?? fallback;
}
