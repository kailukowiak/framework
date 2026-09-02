import {
  blankVectorStep,
  vectorStepFormulas,
  vectorStepInput,
  vectorStepIsIncomplete,
  type BroadcastStepDraft,
  type ZipVectorStepDraft,
} from "../PipelineVectorSteps";
import { draftName, type NamedDraft } from "../PipelineColumnNames";
import { type PipelineRecurrenceDraft } from "../PipelineRecurrenceStep";
import { recurrenceFormula } from "../RecurrenceDialog";
import { meltedColumnIds } from "./columnList";
import { formulaToken, type FormulaReference } from "./formulaReferences";
import type {
  Column,
  DataType,
  FrameStepInput,
  PivotAggregate,
} from "./types";

/**
 * One step of the chain, as the editor holds it.
 *
 * Order is the whole point: each step is parsed against the columns the
 * steps above it leave behind, so two `withColumns` in a row mean something
 * a single one with both expressions cannot -- the second can read what the
 * first made.
 */
export type StepDraft =
  | {
      id: string;
      kind: "filter";
      predicates: Array<{
        id: string;
        formula: string;
        focusToken?: number;
        focusSelection?: { start: number; end: number };
      }>;
      matchAll: boolean;
    }
  | { id: string; kind: "withColumns"; columns: NamedDraft[] }
  | PipelineRecurrenceDraft
  | {
      id: string;
      kind: "select";
      columnIds: string[];
      /** The human decision this projection records; placement is internal. */
      mode: "delete" | "rearrange" | "placement";
    }
  | {
      id: string;
      kind: "summarize";
      groupKeys: NamedDraft[];
      aggregates: NamedDraft[];
      maintainOrder: boolean;
    }
  | {
      id: string;
      kind: "sort";
      keys: Array<{ id: string; columnId: string; descending: boolean }>;
    }
  | { id: string; kind: "union"; frameId: string }
  | { id: string; kind: "expand"; frameId: string }
  | {
      id: string;
      kind: "pivot";
      namesColumnId: string;
      valuesColumnId: string;
      aggregate: PivotAggregate;
    }
  | {
      id: string;
      kind: "unpivot";
      /** The melt list as written: `` `Jan`, `Feb`, starts_with("Q") ``. */
      columns: string;
      nameColumnId: string;
      nameColumnName: string;
      valueColumnId: string;
      valueColumnName: string;
    }
  | BroadcastStepDraft
  | ZipVectorStepDraft
  /** A remark standing in the chain. Markdown; the engine skips it. */
  | { id: string; kind: "comment"; text: string };

export type StepKind = StepDraft["kind"];

export const STEP_LABELS: Record<StepKind, string> = {
  filter: "Filter rows",
  withColumns: "Add or replace columns",
  recurrence: "Calculate down rows",
  select: "Columns",
  summarize: "Summarize",
  sort: "Sort",
  union: "Stack frame",
  expand: "Expand frame",
  pivot: "Pivot",
  unpivot: "Unpivot",
  broadcast: "Apply vector across columns",
  zipVector: "Pair vector as column",
  comment: "Comment",
};

export type AddStepKind =
  | Exclude<StepKind, "select" | "recurrence">
  | "deleteColumns"
  | "rearrangeColumns";

export function selectStepLabel(step: Extract<StepDraft, { kind: "select" }>): string {
  if (step.mode === "delete") return "Delete columns";
  if (step.mode === "rearrange") return "Rearrange columns";
  return "Columns";
}

export type VisibleColumn = { id: string; name: string; dataType?: DataType };

/**
 * The columns a step can see: the source's, then whatever the steps above it
 * did. Computed here rather than asked of the core so typing a formula costs
 * no round trip -- the core still has the final say when the chain is saved.
 */
export function columnsBeforeStep(
  sourceColumns: Column[],
  steps: StepDraft[],
  index: number
): VisibleColumn[] {
  let visible: VisibleColumn[] = sourceColumns.map((column) => ({
    id: column.id,
    name: column.name,
    dataType: column.dataType,
  }));
  for (const step of steps.slice(0, index)) {
    if (step.kind === "withColumns" || step.kind === "recurrence") {
      const outputs =
        step.kind === "withColumns"
          ? step.columns.map((column) => ({
              outputColumnId: column.outputColumnId,
              name: column.name,
            }))
          : [step];
      for (const column of outputs) {
        const existing = visible.findIndex(
          (candidate) => candidate.id === column.outputColumnId
        );
        const next = { id: column.outputColumnId, name: column.name };
        if (existing >= 0) visible[existing] = next;
        else visible = [...visible, next];
      }
    } else if (step.kind === "select") {
      visible = step.columnIds
        .map((columnId) => visible.find((candidate) => candidate.id === columnId))
        .filter((column): column is VisibleColumn => Boolean(column));
    } else if (step.kind === "summarize") {
      visible = [...step.groupKeys, ...step.aggregates].map((column) => ({
        id: column.outputColumnId,
        name: column.name,
      }));
    } else if (step.kind === "pivot") {
      // The pivoted-out columns are data-dependent -- only the core's own
      // preview knows their names, so the local walk can only say that the
      // two columns feeding the pivot are gone.
      visible = visible.filter(
        (column) =>
          column.id !== step.namesColumnId && column.id !== step.valuesColumnId
      );
    } else if (step.kind === "unpivot") {
      // The written list is read locally so the walk costs no round trip;
      // the core's preview replaces this answer whenever the chain parses.
      const melted = meltedColumnIds(step.columns, visible);
      visible = [
        ...visible.filter((column) => !melted.includes(column.id)),
        { id: step.nameColumnId, name: step.nameColumnName, dataType: "string" },
        { id: step.valueColumnId, name: step.valueColumnName, dataType: "string" },
      ];
    } else if (step.kind === "zipVector") {
      visible = [
        ...visible,
        { id: step.outputColumnId, name: step.name },
      ];
    }
    // A union adds rows. Expand's columns are supplied by the core preview:
    // the local walk has frame names but deliberately does not duplicate
    // another frame's schema.
  }
  return visible;
}

/**
 * A projection that keeps every input is placement bookkeeping, not a
 * transformation somebody authored. Context insertion needs it because the
 * query engine appends a calculated expression, but drawing a whole Choose
 * columns block for that implementation detail makes one gesture look like
 * two decisions. A projection that omits anything remains visible and
 * editable: that one really is choosing columns.
 */
export function isOrderingOnlySelect(
  sourceColumns: Column[],
  steps: StepDraft[],
  index: number
): boolean {
  const step = steps[index];
  if (
    step?.kind !== "select" ||
    index === 0 ||
    !["withColumns", "recurrence"].includes(steps[index - 1].kind)
  ) {
    return false;
  }
  if (step.mode !== "placement") return false;
  const available = columnsBeforeStep(sourceColumns, steps, index).map(
    (column) => column.id
  );
  return (
    available.length === step.columnIds.length &&
    new Set(available).size === available.length &&
    available.every((columnId) => step.columnIds.includes(columnId))
  );
}

/** Column references offered to a formula, scoped to what that step can see. */
export function referencesForStep(
  references: FormulaReference[],
  visible: VisibleColumn[]
): FormulaReference[] {
  const sourceById = new Map(references.map((reference) => [reference.id, reference]));
  const columnReferences = visible.map((column) => {
    const source = sourceById.get(column.id);
    return {
      id: column.id,
      objectId: source?.objectId,
      frameId: source?.frameId,
      label: column.name,
      token: formulaToken(column.name),
      kind: "column" as const,
      detail: "column",
    };
  });
  return [
    ...columnReferences,
    ...references.filter((reference) => reference.kind !== "column"),
  ];
}

/**
 * A new draft, unnamed. `fallbackName` is what it is called until the
 * formula suggests something or the user types a name -- it shows as
 * placeholder text, so the field stays empty and ready to be typed in.
 */
export function namedDraft(fallbackName: string, formula: string): NamedDraft {
  return {
    id: crypto.randomUUID(),
    outputColumnId: mintColumnId(fallbackName),
    name: "",
    formula,
    fallbackName,
  };
}

export function blankStep(
  kind: AddStepKind,
  visible: VisibleColumn[],
  sourceColumns: Column[]
): StepDraft {
  const id = crypto.randomUUID();
  const vectorStep = blankVectorStep(kind, id, mintColumnId);
  if (vectorStep) return vectorStep;
  const numeric = sourceColumns.find((column) =>
    ["integer", "number", "currency", "percentage"].includes(column.dataType)
  );
  switch (kind) {
    case "filter":
      return {
        id,
        kind,
        predicates: [{ id: crypto.randomUUID(), formula: "" }],
        matchAll: true,
      };
    case "withColumns":
      return { id, kind, columns: [namedDraft("Column 1", "")] };
    case "deleteColumns":
      return {
        id,
        kind: "select",
        columnIds: visible.map((column) => column.id),
        mode: "delete",
      };
    case "rearrangeColumns":
      return {
        id,
        kind: "select",
        columnIds: visible.map((column) => column.id),
        mode: "rearrange",
      };
    case "summarize":
      return {
        id,
        kind,
        groupKeys: [],
        aggregates: [
          namedDraft(
            numeric ? `${numeric.name} sum` : "Count",
            numeric ? `${formulaToken(numeric.name)}.sum()` : "len()"
          ),
        ],
        maintainOrder: true,
      };
    case "sort":
      return {
        id,
        kind,
        keys: visible[0]
          ? [{ id: crypto.randomUUID(), columnId: visible[0].id, descending: false }]
          : [],
      };
    case "union":
      return { id, kind, frameId: "" };
    case "expand":
      return { id, kind, frameId: "" };
    case "pivot": {
      const namesColumn = visible.find(
        (column) => column.dataType === "string" || column.dataType === "categorical"
      );
      const valuesColumn = visible.find((column) =>
        column.dataType
          ? ["integer", "number", "currency", "percentage"].includes(column.dataType)
          : false
      );
      return {
        id,
        kind,
        namesColumnId: namesColumn?.id ?? "",
        valuesColumnId: valuesColumn?.id ?? "",
        aggregate: "sum",
      };
    }
    case "unpivot":
      return {
        id,
        kind,
        columns: "",
        nameColumnId: mintColumnId("Name"),
        nameColumnName: "Name",
        valueColumnId: mintColumnId("Value"),
        valueColumnName: "Value",
      };
    case "comment":
      return { id, kind, text: "" };
  }
  throw new Error(`Unsupported transformation: ${kind}`);
}

/**
 * The engine uses column ids as physical Polars names, so their readable
 * prefix pays for itself in every query plan and error. The suffix is the
 * immutable identity: changing the editable name never changes this string.
 */
export function mintColumnId(name: string): string {
  const slug =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "")
      .slice(0, 48) || "column";
  const alphabet = "0123456789abcdefghjkmnpqrstvwxyz";
  let random =
    Number.parseInt(crypto.randomUUID().replaceAll("-", "").slice(0, 8), 16) >>> 2;
  let suffix = "";
  for (let index = 0; index < 6; index += 1) {
    suffix = alphabet[random & 31] + suffix;
    random >>>= 5;
  }
  return `${slug}~${suffix}`;
}

export function stepInput(step: StepDraft): FrameStepInput {
  if (step.kind === "broadcast" || step.kind === "zipVector")
    return vectorStepInput(step);
  switch (step.kind) {
    case "filter":
      return {
        kind: "filter",
        predicates: step.predicates.map((predicate) => predicate.formula),
        matchAll: step.matchAll,
      };
    case "withColumns":
      return {
        kind: "withColumns",
        columns: step.columns.map((column) => ({
          outputColumnId: column.outputColumnId,
          name: draftName(column),
          formula: column.formula,
        })),
      };
    case "recurrence":
      return {
        kind: "withColumns",
        columns: [
          {
            outputColumnId: step.outputColumnId,
            name: step.name,
            formula: recurrenceFormula(step.seed, step.formula, step.partitionName),
          },
        ],
      };
    case "select":
      return { kind: "select", columnIds: step.columnIds };
    case "summarize":
      return {
        kind: "summarize",
        groupKeys: step.groupKeys.map((key) => ({
          outputColumnId: key.outputColumnId,
          name: draftName(key),
          formula: key.formula,
        })),
        aggregates: step.aggregates.map((aggregate) => ({
          outputColumnId: aggregate.outputColumnId,
          name: draftName(aggregate),
          formula: aggregate.formula,
        })),
        maintainOrder: step.maintainOrder,
      };
    case "sort":
      return {
        kind: "sort",
        keys: step.keys.map(({ columnId, descending }) => ({ columnId, descending })),
      };
    case "union":
      return { kind: "union", frameId: step.frameId };
    case "expand":
      return { kind: "expand", frameId: step.frameId };
    case "pivot":
      return {
        kind: "pivot",
        namesColumnId: step.namesColumnId,
        valuesColumnId: step.valuesColumnId,
        aggregate: step.aggregate,
      };
    case "unpivot":
      return {
        kind: "unpivot",
        columns: step.columns,
        nameColumnId: step.nameColumnId,
        nameColumnName: step.nameColumnName,
        valueColumnId: step.valueColumnId,
        valueColumnName: step.valueColumnName,
      };
    case "comment":
      return { kind: "comment", text: step.text };
  }
}

export function stepIsIncomplete(step: StepDraft): boolean {
  if (step.kind === "broadcast" || step.kind === "zipVector")
    return vectorStepIsIncomplete(step);
  const reshapeIncomplete = reshapeStepIsIncomplete(step);
  if (reshapeIncomplete !== null) return reshapeIncomplete;
  switch (step.kind) {
    case "filter":
      return (
        step.predicates.length === 0 ||
        step.predicates.some((predicate) => !predicate.formula.trim())
      );
    case "withColumns":
      return (
        step.columns.length === 0 ||
        step.columns.some((column) => !draftName(column) || !column.formula.trim())
      );
    case "recurrence":
      return !step.name.trim() || !step.seed.trim() || !step.formula.trim();
    case "select":
      return step.columnIds.length === 0;
    case "summarize":
      return (
        step.aggregates.length === 0 ||
        [...step.groupKeys, ...step.aggregates].some(
          (column) => !draftName(column) || !column.formula.trim()
        )
      );
    case "sort":
      return step.keys.length === 0;
    case "comment":
      return !step.text.trim();
  }
  throw new Error("Unsupported transformation");
}

function reshapeStepIsIncomplete(step: StepDraft): boolean | null {
  if (step.kind === "union" || step.kind === "expand") return !step.frameId;
  if (step.kind === "pivot") return !step.namesColumnId || !step.valuesColumnId;
  return step.kind === "unpivot"
    ? !step.columns.trim() ||
        !step.nameColumnName.trim() ||
        !step.valueColumnName.trim()
    : null;
}

export function stepFormulas(step: StepDraft): string[] {
  if (step.kind === "broadcast" || step.kind === "zipVector")
    return vectorStepFormulas(step);
  switch (step.kind) {
    case "filter":
      return step.predicates.map((predicate) => predicate.formula);
    case "withColumns":
      return step.columns.map((column) => column.formula);
    case "recurrence":
      return [step.seed, step.formula];
    case "summarize":
      return [...step.groupKeys, ...step.aggregates].map((column) => column.formula);
    // The melt list is written text the save parses, so an error about it
    // deserves the same context a formula gets.
    case "unpivot":
      return [step.columns];
    case "select":
    case "sort":
    case "union":
    case "expand":
    case "pivot":
    case "comment":
      return [];
  }
}
