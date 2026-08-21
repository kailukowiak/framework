import { CircleAlert, GitBranch, Plus, RefreshCw, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Markdown } from "./Markdown";
import { FormulaErrorDetails } from "./FormulaEditor";
import { PipelineCommand } from "./PipelineCommand";
import { PipelineFrameStepCommand } from "./PipelineFrameStepCommand";
import {
  PipelineRecurrenceStep,
  type PipelineRecurrenceDraft,
} from "./PipelineRecurrenceStep";
import { parseRecurrenceFormula, recurrenceFormula } from "./RecurrenceDialog";
import { previewFramePipeline, type PipelineSchema } from "./lib/api";
import { meltedColumnIds } from "./lib/columnList";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";
import { aliasFromFormula } from "./lib/formulaAlias";
import {
  formattedFormula,
  formatPipelineFormulas,
  recurrenceDraft,
} from "./PipelineFormulaFormatting";
import type { OperationHandler } from "./lib/handlers";
import type {
  Column,
  DataType,
  FrameObject,
  RenderedFrameStep,
  FrameStepInput,
  PivotAggregate,
} from "./lib/types";

/**
 * A remark in the chain: rendered markdown until clicked, a plain textarea
 * while being written. No formula machinery — the text is never parsed —
 * and no Save button: blur commits, and committing nothing removes the
 * row, so "no comment" stays the absence of a step.
 */
function CommentStepRow({
  text,
  startEditing,
  onCommit,
}: {
  text: string;
  startEditing: boolean;
  onCommit: (text: string) => void;
}) {
  const [editing, setEditing] = useState(startEditing);
  const [draft, setDraft] = useState(text);
  const cancelled = useRef(false);
  if (!editing)
    return (
      <button
        type="button"
        className="pipeline-comment"
        title="Edit comment"
        onClick={() => {
          setDraft(text);
          setEditing(true);
        }}
      >
        <Markdown source={text} />
      </button>
    );
  return (
    <textarea
      className="pipeline-comment-editor"
      value={draft}
      autoFocus
      rows={Math.max(2, draft.split("\n").length)}
      placeholder="Why the chain does what it does — markdown allowed"
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => {
        setEditing(false);
        onCommit(cancelled.current ? text : draft);
        cancelled.current = false;
      }}
      onKeyDown={(event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === "Enter")
          event.currentTarget.blur();
        if (event.key === "Escape") {
          cancelled.current = true;
          event.currentTarget.blur();
        }
      }}
    />
  );
}

/**
 * A named expression inside a step: what to compute, and what to call the
 * column it produces.
 */
type NamedDraft = {
  id: string;
  outputColumnId: string;
  /**
   * What the user typed, and only that. Empty means they have not named it,
   * not that it has no name -- see `draftName`.
   */
  name: string;
  formula: string;
  /** What to call it when the formula suggests nothing either. */
  fallbackName: string;
  /** A request from outside the chain to put the cursor in this formula. */
  focusToken?: number;
  /** Put that cursor after the seeded expression instead of selecting it. */
  focusAtEnd?: boolean;
  /** Ephemeral row anchor for a formula begun from a grid cell. */
  anchorRowIndex?: number;
};

/**
 * The name a draft would take if it were saved right now.
 *
 * A name the user typed wins. Otherwise `` `debit`.sum() `` calls itself
 * "Debit Sum", which is what anyone describing it out loud would call it,
 * and a formula that suggests nothing leaves the seeded fallback.
 */
function draftName(draft: NamedDraft): string {
  return draft.name.trim() || suggestedName(draft);
}

/** The half of `draftName` the field shows as placeholder text. */
function suggestedName(draft: NamedDraft): string {
  return aliasFromFormula(draft.formula) || draft.fallbackName;
}

/** Keep a requested label recognizable while making it referenceable. */
export function uniqueColumnName(name: string, existing: string[]): string {
  if (!existing.includes(name)) return name;
  const blank = name.match(/^Column (\d+)$/);
  if (blank) {
    let number = Number(blank[1]) + 1;
    while (existing.includes(`Column ${number}`)) number += 1;
    return `Column ${number}`;
  }
  const numbered = name.match(/^(.*)_(\d+)$/);
  const root = numbered?.[1] || name;
  let suffix = numbered ? Number(numbered[2]) + 1 : 2;
  while (existing.includes(`${root}_${suffix}`)) suffix += 1;
  return `${root}_${suffix}`;
}

/** Spreadsheet-style names count upward instead of minting another Column 1. */
export function nextBlankColumnName(existing: string[]): string {
  let largest = 0;
  for (const name of existing) {
    const match = name.match(/^Column (\d+)$/);
    if (match) largest = Math.max(largest, Number(match[1]));
  }
  return `Column ${largest + 1}`;
}

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
  /** A remark standing in the chain. Markdown; the engine skips it. */
  | { id: string; kind: "comment"; text: string };

type StepKind = StepDraft["kind"];

const STEP_LABELS: Record<StepKind, string> = {
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
  comment: "Comment",
};

type AddStepKind =
  | Exclude<StepKind, "select" | "recurrence">
  | "deleteColumns"
  | "rearrangeColumns";

function selectStepLabel(step: Extract<StepDraft, { kind: "select" }>): string {
  if (step.mode === "delete") return "Delete columns";
  if (step.mode === "rearrange") return "Rearrange columns";
  return "Columns";
}

type VisibleColumn = { id: string; name: string; dataType?: DataType };

/**
 * The columns a step can see: the source's, then whatever the steps above it
 * did. Computed here rather than asked of the core so typing a formula costs
 * no round trip -- the core still has the final say when the chain is saved.
 */
function columnsBeforeStep(
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
function referencesForStep(
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

/** Text on the left of a transformation assignment is an identifier too. */
function namedCommand(name: string, formula: string): string {
  return `${formulaToken(name)} = ${formula}`;
}

function exactName(token: string): string | null {
  const trimmed = token.trim();
  if (!trimmed.startsWith("`") || !trimmed.endsWith("`")) return null;
  return trimmed.slice(1, -1).replaceAll("``", "`");
}

/** Split only at commas that are not inside a call, string, or identifier. */
function commandPieces(source: string): string[] {
  const pieces: string[] = [];
  let current = "";
  let depth = 0;
  let quote: string | null = null;
  let backticked = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      current += character;
      if (character === "\\") current += source[++index] ?? "";
      else if (character === quote) quote = null;
      continue;
    }
    if (backticked) {
      current += character;
      if (character === "`" && source[index + 1] === "`") current += source[++index];
      else if (character === "`") backticked = false;
      continue;
    }
    if (character === "`") backticked = true;
    else if (character === "'" || character === '"') quote = character;
    else if (character === "(") depth += 1;
    else if (character === ")") depth = Math.max(0, depth - 1);
    else if (character === "," && depth === 0) {
      if (current.trim()) pieces.push(current.trim());
      current = "";
      continue;
    }
    current += character;
  }
  if (current.trim()) pieces.push(current.trim());
  return pieces;
}

export function parseNamedTransformation(
  source: string
): { name: string; formula: string } | null {
  let backticked = false;
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] === "`") {
      if (backticked && source[index + 1] === "`") index += 1;
      else backticked = !backticked;
      continue;
    }
    if (backticked || source[index] !== "=") continue;
    if (source[index - 1] === "=") continue;
    const name = exactName(source.slice(0, index));
    // The command already prints its assignment separator. Spreadsheet
    // muscle memory can still add another `=` before the expression, either
    // adjacent (`name == expression`) or after the separator's spaces
    // (`name = = expression`). In this named-command surface neither spelling
    // can mean a comparison: the left side is the output's name, not an input
    // expression. Forgive the redundant mark here instead of saving an
    // unusable formula or letting it masquerade as a Filter step.
    const afterSeparator = source.slice(
      index + (source[index + 1] === "=" ? 2 : 1)
    );
    const formula = afterSeparator.trim().replace(/^=\s*/, "");
    return name && formula ? { name, formula } : null;
  }
  return null;
}

/** Intentional name reuse means overwrite; a new name keeps its minted id. */
export function outputColumnIdForName(
  visible: Array<{ id: string; name: string }>,
  currentId: string,
  name: string
): string {
  return visible.find((candidate) => candidate.name === name)?.id ?? currentId;
}

/**
 * A header filter starts as a valid-looking comparison with only its value
 * selected. The column click has already supplied the hard part; typing can
 * immediately replace the example value, while the formula remains an
 * ordinary Wrangle condition rather than a second filter model.
 */
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
  if (last?.kind === "filter") {
    return [...steps.slice(0, -1), { ...last, predicates: [...last.predicates, draft] }];
  }
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

function sortCommand(
  step: Extract<StepDraft, { kind: "sort" }>,
  visible: VisibleColumn[]
): string {
  return step.keys
    .map((key) => {
      const name = visible.find((column) => column.id === key.columnId)?.name;
      return name ? `${formulaToken(name)} ${key.descending ? "desc" : "asc"}` : "";
    })
    .filter(Boolean)
    .join(", ");
}

export function parseSortCommand(source: string, visible: VisibleColumn[]) {
  const keys: Array<{ id: string; columnId: string; descending: boolean }> = [];
  for (const piece of commandPieces(source)) {
    const match = /^(.*?)(?:\s+(asc|desc))?$/i.exec(piece);
    const name = exactName(match?.[1] ?? "");
    const column = visible.find((candidate) => candidate.name === name);
    if (!column) return null;
    keys.push({
      id: crypto.randomUUID(),
      columnId: column.id,
      descending: match?.[2]?.toLowerCase() === "desc",
    });
  }
  return keys.length ? keys : null;
}

function columnListCommand(ids: string[], visible: VisibleColumn[]): string {
  return ids
    .map((id) => visible.find((column) => column.id === id)?.name)
    .filter((name): name is string => Boolean(name))
    .map(formulaToken)
    .join(", ");
}

export function reorderColumnIds(
  ids: string[],
  draggedId: string,
  targetId: string,
  afterTarget: boolean
): string[] {
  if (draggedId === targetId || !ids.includes(draggedId) || !ids.includes(targetId))
    return ids;
  const reordered = ids.filter((id) => id !== draggedId);
  const targetIndex = reordered.indexOf(targetId);
  reordered.splice(targetIndex + (afterTarget ? 1 : 0), 0, draggedId);
  return reordered.every((id, index) => id === ids[index]) ? ids : reordered;
}

function pivotCommand(
  step: Extract<StepDraft, { kind: "pivot" }>,
  visible: VisibleColumn[]
): string {
  const names = visible.find((column) => column.id === step.namesColumnId)?.name;
  const values = visible.find((column) => column.id === step.valuesColumnId)?.name;
  return `columns=${names ? formulaToken(names) : ""}, values=${
    values ? formulaToken(values) : ""
  }, aggregate=${step.aggregate}`;
}

export function parsePivotCommand(source: string, visible: VisibleColumn[]) {
  const fields = Object.fromEntries(
    commandPieces(source).map((piece) => {
      const equals = piece.indexOf("=");
      return equals < 0
        ? [piece.trim().toLowerCase(), ""]
        : [piece.slice(0, equals).trim().toLowerCase(), piece.slice(equals + 1).trim()];
    })
  );
  const names = exactName(fields.columns ?? "");
  const values = exactName(fields.values ?? "");
  const aggregate = fields.aggregate?.toLowerCase() as PivotAggregate | undefined;
  const namesColumn = visible.find((column) => column.name === names);
  const valuesColumn = visible.find((column) => column.name === values);
  const aggregates: PivotAggregate[] = [
    "sum",
    "count",
    "mean",
    "min",
    "max",
    "first",
    "none",
  ];
  if (!namesColumn || !valuesColumn || !aggregate || !aggregates.includes(aggregate))
    return null;
  return {
    namesColumnId: namesColumn.id,
    valuesColumnId: valuesColumn.id,
    aggregate,
  };
}

function unpivotCommand(step: Extract<StepDraft, { kind: "unpivot" }>): string {
  return `columns=${step.columns}, names=${formulaToken(
    step.nameColumnName
  )}, values=${formulaToken(step.valueColumnName)}`;
}

export function parseUnpivotCommand(source: string) {
  const match =
    /^\s*columns\s*=\s*(.*?)\s*,\s*names\s*=\s*(`(?:``|[^`])+`)\s*,\s*values\s*=\s*(`(?:``|[^`])+`)\s*$/is.exec(
      source
    );
  if (!match) return null;
  const nameColumnName = exactName(match[2]);
  const valueColumnName = exactName(match[3]);
  return match[1].trim() && nameColumnName && valueColumnName
    ? { columns: match[1].trim(), nameColumnName, valueColumnName }
    : null;
}

/**
 * A new draft, unnamed. `fallbackName` is what it is called until the
 * formula suggests something or the user types a name -- it shows as
 * placeholder text, so the field stays empty and ready to be typed in.
 */
function namedDraft(fallbackName: string, formula: string): NamedDraft {
  return {
    id: crypto.randomUUID(),
    outputColumnId: mintColumnId(fallbackName),
    name: "",
    formula,
    fallbackName,
  };
}

/**
 * The context-menu gesture creates something real before asking for its
 * expression. A typed null is blank in every row but still gives the query
 * plan a stable dtype, so the column can render immediately and the formula
 * can be replaced in place without a second creation path.
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
    ...namedDraft(fallbackName, 'null.cast("number")'),
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
  focusToken?: number,
  focusAtEnd = true
): StepDraft[] {
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
        },
      ],
    },
  ];
}

/** Add a declared order immediately before a row-position formula needs it. */
export function appendOrderedColumnTransformation(
  steps: StepDraft[],
  column: { id: string; name: string },
  formula: string,
  orderByColumnId?: string,
  focusToken?: number,
  focusAtEnd = true
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
  const recurrence = parseRecurrenceFormula(formula);
  if (!recurrence)
    return appendInPlaceColumnTransformation(
      ordered, column, formula, focusToken, focusAtEnd
    );
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
    },
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

function blankStep(
  kind: AddStepKind,
  visible: VisibleColumn[],
  sourceColumns: Column[]
): StepDraft {
  const id = crypto.randomUUID();
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
  const nameOf = (outputColumnId: string, fallback: string) =>
    editingFrame.columns.find((column) => column.id === outputColumnId)?.name ??
    fallback;
  const drafts: StepDraft[] = [];
  for (const step of rendered) {
    const id = crypto.randomUUID();
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

function stepInput(step: StepDraft): FrameStepInput {
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

function stepIsIncomplete(step: StepDraft): boolean {
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
    case "union":
      return step.frameId === "";
    case "expand":
      return step.frameId === "";
    case "pivot":
      return !step.namesColumnId || !step.valuesColumnId;
    case "unpivot":
      return (
        !step.columns.trim() ||
        !step.nameColumnName.trim() ||
        !step.valueColumnName.trim()
      );
    case "comment":
      return !step.text.trim();
  }
}

function stepFormulas(step: StepDraft): string[] {
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

/**
 * The transformation chain: an ordered list of steps, each editable, all
 * reorderable by dragging. Saving replaces the whole chain, which is also
 * how the core validates it -- a step naming a column no earlier step
 * produces is refused by name rather than failing later at read time.
 */
export function DerivedFrameCreator({
  input,
  editingFrame,
  renderedSteps,
  passThroughSteps,
  references,
  frames,
  addCalculatedColumnRequest,
  onAddCalculatedColumnRequestHandled,
  transformColumnRequest,
  onTransformColumnRequestHandled,
  filterColumnRequest,
  onFilterColumnRequestHandled,
  hidePipelineColumnRequest,
  onHidePipelineColumnRequestHandled,
  rearrangeColumnsRequest,
  onRearrangeColumnsRequestHandled,
  onOperation,
}: {
  /** Where the chain starts: the frame it derives from, or its own data. */
  input: { label: string; columns: Column[]; completionFrameId?: string };
  editingFrame: FrameObject;
  renderedSteps: RenderedFrameStep[];
  /**
   * Leading steps that exist only so a linked frame owns its column ids.
   * They are held in state and saved back untouched, but never drawn —
   * nobody wrote them, and dropping them would strand every column id this
   * frame has published.
   */
  passThroughSteps: number;
  references: FormulaReference[];
  /** Every other frame in the document, for two-input wrangle steps. */
  frames: Array<{ id: string; name: string }>;
  /** Appends and saves one blank Number column at the bottom of the chain. */
  addCalculatedColumnRequest?: {
    token: number;
    afterColumnId?: string;
    anchorRowIndex?: number;
  };
  onAddCalculatedColumnRequestHandled?: () => void;
  /** Appends a same-id expression, optionally focusing it for another method call. */
  transformColumnRequest?: {
    token: number;
    columnId: string;
    formula: string;
    focus?: boolean;
    editExisting?: boolean;
    anchorRowIndex?: number;
    orderByColumnId?: string;
    focusAtEnd?: boolean;
  };
  onTransformColumnRequestHandled?: () => void;
  /** Opens a single-column condition in the canonical Wrangle filter step. */
  filterColumnRequest?: { token: number; columnId: string };
  onFilterColumnRequestHandled?: () => void;
  hidePipelineColumnRequest?: { token: number; columnId: string };
  onHidePipelineColumnRequestHandled?: () => void;
  rearrangeColumnsRequest?: { token: number; columnIds: string[] };
  onRearrangeColumnsRequestHandled?: () => void;
  onOperation: OperationHandler;
}) {
  const [steps, setSteps] = useState<StepDraft[]>(() =>
    stepsFromRendered(renderedSteps, editingFrame, input.columns)
  );
  const visibleAuthored = steps
    .map((step, index) => ({ step, index }))
    .slice(passThroughSteps)
    .filter(({ index }) => !isOrderingOnlySelect(input.columns, steps, index));
  const [formulaError, setFormulaError] = useState<string | null>(null);
  const [refreshingGeneratedColumns, setRefreshingGeneratedColumns] = useState(false);
  const [dragging, setDragging] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);
  const [draggingColumn, setDraggingColumn] = useState<string | null>(null);
  const [columnDrop, setColumnDrop] = useState<{
    columnId: string;
    after: boolean;
  } | null>(null);
  const [pendingEditor, setPendingEditor] = useState<string | null>(null);
  const handledAddRequest = useRef<number | null>(null);
  const handledTransformRequest = useRef<number | null>(null);
  const handledFilterRequest = useRef<number | null>(null);
  const handledHideRequest = useRef<number | null>(null);
  const handledRearrangeRequest = useRef<number | null>(null);

  // Declared ahead of the request-handling effects below: their dependency
  // arrays reference it directly (not just from inside the effect body), so
  // it has to exist by the time those arrays are built, not just by the
  // time the effects run.
  const persist = useCallback(
    async (next: StepDraft[]) => {
      const normalized = normalizeCalculatedColumnNames(
        next,
        input.columns,
        passThroughSteps
      ).map(formatPipelineFormulas);
      setSteps(normalized);
      const failure = await onOperation(
        {
          type: "setFramePipeline",
          frameId: editingFrame.id,
          steps: normalized.map(stepInput),
        },
        { inlineError: true }
      );
      setFormulaError(failure);
    },
    [input.columns, passThroughSteps, editingFrame.id, onOperation]
  );

  useEffect(() => {
    if (
      addCalculatedColumnRequest === undefined ||
      handledAddRequest.current === addCalculatedColumnRequest.token
    )
      return;
    handledAddRequest.current = addCalculatedColumnRequest.token;
    onAddCalculatedColumnRequestHandled?.();
    const next = normalizeCalculatedColumnNames(
      appendBlankCalculatedColumn(
        steps,
        addCalculatedColumnRequest.token,
        addCalculatedColumnRequest.afterColumnId,
        editingFrame.columns,
        passThroughSteps,
        addCalculatedColumnRequest.anchorRowIndex
      ),
      input.columns,
      passThroughSteps
    );
    setSteps(next);
    void onOperation(
      {
        type: "setFramePipeline",
        frameId: editingFrame.id,
        steps: next.map(stepInput),
      },
      { inlineError: true }
    ).then(setFormulaError);
  }, [
    addCalculatedColumnRequest,
    editingFrame.columns,
    editingFrame.id,
    input.columns,
    onAddCalculatedColumnRequestHandled,
    onOperation,
    passThroughSteps,
    steps,
  ]);

  useEffect(() => {
    if (
      transformColumnRequest === undefined ||
      handledTransformRequest.current === transformColumnRequest.token
    )
      return;
    handledTransformRequest.current = transformColumnRequest.token;
    onTransformColumnRequestHandled?.();
    const column = editingFrame.columns.find(
      (candidate) => candidate.id === transformColumnRequest.columnId
    );
    if (!column) return;
    if (transformColumnRequest.editExisting) {
      const focused = focusExistingCalculatedColumn(
        steps,
        column.id,
        transformColumnRequest.token,
        transformColumnRequest.anchorRowIndex
      );
      if (focused) setSteps(focused);
      else setFormulaError(`The calculation for ${column.name} is not in this chain.`);
      return;
    }
    const next = appendOrderedColumnTransformation(
      steps,
      column,
      transformColumnRequest.formula,
      transformColumnRequest.orderByColumnId,
      transformColumnRequest.focus ? transformColumnRequest.token : undefined,
      transformColumnRequest.focusAtEnd
    );
    setSteps(next);
    void onOperation(
      {
        type: "setFramePipeline",
        frameId: editingFrame.id,
        steps: next.map(stepInput),
      },
      { inlineError: true }
    ).then(setFormulaError);
  }, [
    transformColumnRequest,
    editingFrame.columns,
    editingFrame.id,
    onOperation,
    onTransformColumnRequestHandled,
    steps,
  ]);

  useEffect(() => {
    if (
      filterColumnRequest === undefined ||
      handledFilterRequest.current === filterColumnRequest.token
    )
      return;
    handledFilterRequest.current = filterColumnRequest.token;
    onFilterColumnRequestHandled?.();
    const column = editingFrame.columns.find(
      (candidate) => candidate.id === filterColumnRequest.columnId
    );
    if (!column) return;
    // This remains a local draft until Enter. Merely opening a header filter
    // must not silently remove rows from the frame.
    setSteps((current) => appendColumnFilter(current, column, filterColumnRequest.token));
  }, [
    editingFrame.columns,
    filterColumnRequest,
    onFilterColumnRequestHandled,
  ]);

  useEffect(() => {
    if (
      hidePipelineColumnRequest === undefined ||
      handledHideRequest.current === hidePipelineColumnRequest.token
    )
      return;
    handledHideRequest.current = hidePipelineColumnRequest.token;
    onHidePipelineColumnRequestHandled?.();
    const next = hidePipelineColumn(
      steps,
      hidePipelineColumnRequest.columnId,
      editingFrame.columns.map((column) => column.id)
    );
    if (!next) return;
    void onOperation(
      {
        type: "setFramePipeline",
        frameId: editingFrame.id,
        steps: next.map(stepInput),
      },
      { inlineError: true }
    ).then((failure) => {
      setFormulaError(failure);
      if (!failure) setSteps(next);
    });
  }, [
    hidePipelineColumnRequest,
    editingFrame.columns,
    editingFrame.id,
    onHidePipelineColumnRequestHandled,
    onOperation,
    steps,
  ]);

  useEffect(() => {
    if (
      rearrangeColumnsRequest === undefined ||
      handledRearrangeRequest.current === rearrangeColumnsRequest.token
    )
      return;
    handledRearrangeRequest.current = rearrangeColumnsRequest.token;
    onRearrangeColumnsRequestHandled?.();
    const next = rearrangePipelineColumns(steps, rearrangeColumnsRequest.columnIds);
    void persist(next);
  }, [rearrangeColumnsRequest, onRearrangeColumnsRequestHandled, persist, steps]);

  // A save error describes the exact draft that was submitted. Once any
  // step changes it is historical, and leaving it visible makes a corrected
  // formula look as though it is still being rejected before Save is tried.
  useEffect(() => setFormulaError(null), [steps]);

  // The draft as the core reads it. Memoized so a scope object handed to a
  // formula editor is stable between keystrokes in *other* steps, which is
  // what keeps its completion effect from refiring on every edit anywhere.
  const stepScopeInputs = useMemo(() => steps.map(stepInput), [steps]);

  // What the draft would actually produce, asked of the core rather than
  // worked out here. It answers from the query plan, so this costs no scan
  // — but it is a round trip, so it waits for typing to finish.
  const [preview, setPreview] = useState<PipelineSchema | null>(null);
  // The draft the preview above describes. Held by identity, which is all
  // that is needed: `stepScopeInputs` is rebuilt whenever the chain changes,
  // so anything but the very array that was sent means the answer is about
  // a draft that no longer exists.
  const [previewOf, setPreviewOf] = useState<FrameStepInput[] | null>(null);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      void previewFramePipeline(editingFrame.id, stepScopeInputs)
        .then((next) => {
          if (disposed) return;
          setPreview(next);
          setPreviewOf(stepScopeInputs);
        })
        // A preview that cannot be taken is not worth reporting: the
        // fallback below still describes the chain, and saving reports the
        // real error against the real chain.
        .catch(() => {});
    }, 250);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [editingFrame.id, stepScopeInputs]);

  /**
   * The columns a step can read.
   *
   * The core's answer when it reaches that far, because it carries real
   * types and agrees with what will actually run. The local walk is the
   * fallback for a draft the core could not get through — typically the
   * step being typed into right now, which is exactly when a stale answer
   * would be worse than a guessed one.
   */
  const visibleBeforeStep = (index: number): VisibleColumn[] => {
    const columns =
      index === 0 ? preview?.inputColumns : preview?.steps[index - 1]?.columns;
    return (
      columns?.map((column) => ({
        id: column.id,
        name: column.name,
        dataType: column.dataType,
      })) ?? columnsBeforeStep(input.columns, steps, index)
    );
  };

  const patch = (stepId: string, update: (step: StepDraft) => StepDraft) =>
    setSteps((current) =>
      current.map((step) => (step.id === stepId ? update(step) : step))
    );
  const removeStep = (stepId: string) => {
    const index = steps.findIndex((step) => step.id === stepId);
    if (index < 0) return;
    const count = isOrderingOnlySelect(input.columns, steps, index + 1) ? 2 : 1;
    void persist(
      steps.filter(
        (_, candidateIndex) => candidateIndex < index || candidateIndex >= index + count
      )
    );
  };

  const moveStep = (fromId: string, toId: string) => {
    const from = steps.findIndex((step) => step.id === fromId);
    const to = steps.findIndex((step) => step.id === toId);
    if (from < 0 || to < 0 || from === to) return;
    const next = [...steps];
    const count = isOrderingOnlySelect(input.columns, steps, from + 1) ? 2 : 1;
    const moved = next.splice(from, count);
    const target = next.findIndex((step) => step.id === toId);
    if (target < 0) return;
    const targetCount = isOrderingOnlySelect(input.columns, next, target + 1) ? 2 : 1;
    const insertion = from < to ? target + targetCount : target;
    next.splice(insertion, 0, ...moved);
    void persist(next);
  };

  const savePatch = async (stepId: string, update: (step: StepDraft) => StepDraft) =>
    persist(steps.map((step) => (step.id === stepId ? update(step) : step)));

  const commandId = (step: StepDraft, itemId?: string) =>
    `pipeline:${editingFrame.id}:${step.id}:${itemId ?? step.kind}`;
  const commandFocus = (id: string, requested?: number) =>
    pendingEditor === id ? requested ?? 1 : requested;
  const rejectCommand = (message: string) => setFormulaError(message);

  return (
    <div className="derived-creator pipeline-outline">
      <div className="section-heading">
        <GitBranch size={16} />
        <strong>Transformations</strong>
        {visibleAuthored.some(({ step }) => step.kind === "pivot") && (
          <button
            type="button"
            className="pipeline-refresh-generated"
            disabled={refreshingGeneratedColumns}
            onClick={() => {
              setRefreshingGeneratedColumns(true);
              void persist(steps).finally(() => setRefreshingGeneratedColumns(false));
            }}
          >
            <RefreshCw
              className={refreshingGeneratedColumns ? "spinning" : ""}
              size={12}
            />
            {refreshingGeneratedColumns ? "Refreshing…" : "Refresh generated columns"}
          </button>
        )}
      </div>
      {visibleAuthored.map(({ step, index }, displayIndex) => {
        const visible = visibleBeforeStep(index);
        const stepReferences = referencesForStep(references, visible);
        const columnReferences = stepReferences.filter(
          (reference) => reference.kind === "column"
        );
        const columnListReferences: FormulaReference[] = [
          ...columnReferences,
          ...[
            ["starts_with", 'starts_with("', "Columns whose names start with text"],
            ["ends_with", 'ends_with("', "Columns whose names end with text"],
            ["contains", 'contains("', "Columns whose names contain text"],
            ["except", "except(", "Every column except those named"],
          ].map(([label, token, detail]) => ({
            id: `selector.${label}`,
            label,
            token,
            kind: "function" as const,
            detail,
          })),
        ];
        const scope = { steps: stepScopeInputs, stepIndex: index };
        const stepFailure =
          previewOf === stepScopeInputs && preview?.failedStep === index
            ? preview.error
            : null;
        return (
          <section
            key={step.id}
            className={`pipeline-step ${dragging === step.id ? "dragging" : ""} ${
              dragOver === step.id ? "drag-target" : ""
            }`}
            onDragOver={(event) => {
              event.preventDefault();
              setDragOver(step.id);
            }}
            onDragLeave={() =>
              setDragOver((current) => (current === step.id ? null : current))
            }
            onDrop={(event) => {
              event.preventDefault();
              if (dragging) moveStep(dragging, step.id);
              setDragging(null);
              setDragOver(null);
            }}
          >
            <div className="pipeline-step-heading">
              <span
                className="pipeline-step-index"
                draggable
                title="Drag to reorder"
                onDragStart={() => setDragging(step.id)}
                onDragEnd={() => {
                  setDragging(null);
                  setDragOver(null);
                }}
              >
                {displayIndex + 1}
              </span>
              <strong>
                {step.kind === "select"
                  ? selectStepLabel(step)
                  : STEP_LABELS[step.kind]}
              </strong>
              <button
                className="remove-derived-row"
                title="Remove step"
                onClick={() => removeStep(step.id)}
              >
                <X size={12} />
              </button>
            </div>

            {step.kind === "comment" && (
              <CommentStepRow
                text={step.text}
                startEditing={pendingEditor === commandId(step) || !step.text}
                onCommit={(draft) => {
                  const trimmed = draft.trim();
                  if (!trimmed) return removeStep(step.id);
                  if (trimmed === step.text) return;
                  void savePatch(step.id, (current) =>
                    current.kind === "comment" ? { ...current, text: trimmed } : current
                  );
                }}
              />
            )}

            {step.kind === "filter" && (
              <div className="pipeline-filter-list">
                {step.predicates.map((predicate, predicateIndex) => {
                  const id = commandId(step, predicate.id);
                  const update = (draft: string, saveNow: boolean) => {
                    if (!draft.trim()) {
                      if (saveNow)
                        rejectCommand("Write a condition before applying this filter");
                      return;
                    }
                    const change = (current: StepDraft): StepDraft =>
                      current.kind === "filter"
                        ? {
                            ...current,
                            predicates: current.predicates.map((item) =>
                              item.id === predicate.id
                                ? { ...item, formula: draft }
                                : item
                            ),
                          }
                        : current;
                    if (saveNow) return savePatch(step.id, change);
                    patch(step.id, change);
                  };
                  return (
                    <div className="pipeline-filter-row" key={predicate.id}>
                      {predicateIndex > 0 && (
                        <button
                          type="button"
                          className="pipeline-filter-join"
                          title={`Match ${step.matchAll ? "any" : "all"} conditions instead`}
                          onClick={() =>
                            void savePatch(step.id, (current) =>
                              current.kind === "filter"
                                ? { ...current, matchAll: !current.matchAll }
                                : current
                            )
                          }
                        >
                          {step.matchAll ? "AND" : "OR"}
                        </button>
                      )}
                      <div className="pipeline-filter-condition">
                        <PipelineCommand
                          editorId={id}
                          label={`Filter condition ${predicateIndex + 1}`}
                          initialDraft={predicate.formula}
                          references={stepReferences}
                          frameId={input.completionFrameId}
                          scope={scope}
                          focusToken={commandFocus(id, predicate.focusToken)}
                          focusSelection={predicate.focusSelection}
                          appliesToAllRows
                          onChange={(draft) => update(draft, false)}
                          onCommit={(draft) => update(draft, true)}
                        />
                        {step.predicates.length > 1 && (
                          <button
                            type="button"
                            className="remove-derived-row"
                            title="Remove condition"
                            onClick={() =>
                              void savePatch(step.id, (current) =>
                                current.kind === "filter"
                                  ? {
                                      ...current,
                                      predicates: current.predicates.filter(
                                        (item) => item.id !== predicate.id
                                      ),
                                    }
                                  : current
                              )
                            }
                          >
                            <X size={11} />
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
                <button
                  type="button"
                  className="pipeline-add-item"
                  onClick={() => {
                    const predicate = { id: crypto.randomUUID(), formula: "" };
                    patch(step.id, (current) =>
                      current.kind === "filter"
                        ? {
                            ...current,
                            predicates: [...current.predicates, predicate],
                          }
                        : current
                    );
                    setPendingEditor(commandId(step, predicate.id));
                  }}
                >
                  <Plus size={11} /> condition
                </button>
              </div>
            )}

            {step.kind === "withColumns" && (
              <div className="pipeline-command-list">
                {step.columns.map((column) => {
                  const id = commandId(step, column.id);
                  const update = (draft: string, saveNow: boolean) => {
                    const parsed = parseNamedTransformation(draft);
                    if (!parsed) {
                      if (saveNow)
                        rejectCommand(
                          "Write a backticked column name, =, and a formula"
                        );
                      return;
                    }
                    const change = (current: StepDraft): StepDraft =>
                      current.kind === "withColumns"
                        ? {
                            ...current,
                            columns: current.columns.map((item) =>
                              item.id === column.id
                                ? {
                                    ...item,
                                    // Naming an output exactly like a column
                                    // visible above this step is an overwrite,
                                    // not a request for "Name_2". Identity is
                                    // what makes Polars replace it in place.
                                    outputColumnId: outputColumnIdForName(
                                      visible,
                                      item.outputColumnId,
                                      parsed.name
                                    ),
                                    name: parsed.name,
                                    formula: parsed.formula,
                                  }
                                : item
                            ),
                          }
                        : current;
                    if (saveNow) return savePatch(step.id, change);
                    patch(step.id, change);
                  };
                  return (
                    <div className="pipeline-command-row" key={column.id}>
                      <PipelineCommand
                        editorId={id}
                        label={draftName(column)}
                        initialDraft={namedCommand(draftName(column), column.formula)}
                        references={stepReferences}
                        frameId={input.completionFrameId}
                        scope={scope}
                        focusToken={commandFocus(id, column.focusToken)}
                        targetColumnId={column.outputColumnId}
                        anchorRowIndex={column.anchorRowIndex}
                        anchorFrameId={editingFrame.id}
                        focusSelection={
                          column.focusToken === undefined
                            ? undefined
                            : {
                                // A fresh formula selects from the very
                                // start, name included: the placeholder
                                // `Column N` is nobody's chosen name, so
                                // typing should be able to replace the whole
                                // line — name, =, and expression — in one
                                // stroke. focusAtEnd flows keep the caret at
                                // the end for appending instead.
                                start: column.focusAtEnd
                                  ? namedCommand(draftName(column), column.formula)
                                      .length
                                  : 0,
                                end: namedCommand(draftName(column), column.formula)
                                  .length,
                              }
                        }
                        onChange={(draft) => update(draft, false)}
                        onCommit={(draft) => update(draft, true)}
                      />
                      <button
                        className="remove-derived-row"
                        title="Remove column"
                        onClick={() =>
                          void savePatch(step.id, (current) =>
                            current.kind === "withColumns" && current.columns.length > 1
                              ? {
                                  ...current,
                                  columns: current.columns.filter(
                                    (item) => item.id !== column.id
                                  ),
                                }
                              : current
                          )
                        }
                      >
                        <X size={11} />
                      </button>
                    </div>
                  );
                })}
                <button
                  className="pipeline-add-item"
                  onClick={() => {
                    const column = namedDraft(
                      nextBlankColumnName(visible.map((item) => item.name)),
                      ""
                    );
                    patch(step.id, (current) =>
                      current.kind === "withColumns"
                        ? { ...current, columns: [...current.columns, column] }
                        : current
                    );
                    setPendingEditor(commandId(step, column.id));
                  }}
                >
                  <Plus size={11} /> Add or replace column
                </button>
              </div>
            )}

            {step.kind === "recurrence" && (
              <PipelineRecurrenceStep
                step={step}
                references={stepReferences}
                columnNames={visible.map((column) => column.name)}
                frameId={input.completionFrameId}
                editingFrameId={editingFrame.id}
                scope={scope}
                seedEditorId={commandId(step, "seed")}
                nextEditorId={commandId(step, "next")}
                focusToken={commandFocus(
                  commandId(step, "next"),
                  step.focusToken
                )}
                onChange={(update) =>
                  patch(step.id, (current) =>
                    current.kind === "recurrence"
                      ? { ...current, ...update }
                      : current
                  )
                }
                onCommit={(update) =>
                  savePatch(step.id, (current) =>
                    current.kind === "recurrence"
                      ? { ...current, ...update }
                      : current
                  )
                }
                onReject={rejectCommand}
              />
            )}

            {step.kind === "select" && step.mode === "rearrange" && (
              <div className="pipeline-column-order" role="list">
                {step.columnIds.map((columnId) => {
                  const column = visible.find((candidate) => candidate.id === columnId);
                  if (!column) return null;
                  const drop = columnDrop?.columnId === columnId ? columnDrop : null;
                  return (
                    <div
                      role="listitem"
                      tabIndex={0}
                      aria-label={`${column.name}, position ${
                        step.columnIds.indexOf(columnId) + 1
                      } of ${step.columnIds.length}`}
                      key={columnId}
                      data-pipeline-column-id={columnId}
                      className={`${draggingColumn === columnId ? "dragging" : ""} ${
                        drop ? (drop.after ? "drop-after" : "drop-before") : ""
                      }`}
                      title="Drag to rearrange"
                      onPointerDown={(event) => {
                        if (event.button !== 0) return;
                        event.preventDefault();
                        event.currentTarget.focus();
                        const start = { x: event.clientX, y: event.clientY };
                        const list = event.currentTarget.closest(
                          ".pipeline-column-order"
                        );
                        let moved = false;
                        let latestDrop: { columnId: string; after: boolean } | null =
                          null;
                        const move = (moveEvent: PointerEvent) => {
                          if (
                            !moved &&
                            Math.hypot(
                              moveEvent.clientX - start.x,
                              moveEvent.clientY - start.y
                            ) < 3
                          )
                            return;
                          moved = true;
                          setDraggingColumn(columnId);
                          const target = document
                            .elementFromPoint(moveEvent.clientX, moveEvent.clientY)
                            ?.closest<HTMLElement>("[data-pipeline-column-id]");
                          if (
                            !target ||
                            target.closest(".pipeline-column-order") !== list
                          ) {
                            latestDrop = null;
                            setColumnDrop(null);
                            return;
                          }
                          const bounds = target.getBoundingClientRect();
                          latestDrop = {
                            columnId: target.dataset.pipelineColumnId!,
                            after: moveEvent.clientY >= bounds.top + bounds.height / 2,
                          };
                          setColumnDrop(latestDrop);
                        };
                        const end = () => {
                          window.removeEventListener("pointermove", move);
                          window.removeEventListener("pointerup", end);
                          window.removeEventListener("pointercancel", end);
                          if (moved && latestDrop) {
                            const ordered = reorderColumnIds(
                              step.columnIds,
                              columnId,
                              latestDrop.columnId,
                              latestDrop.after
                            );
                            if (ordered !== step.columnIds)
                              void savePatch(step.id, (current) =>
                                current.kind === "select"
                                  ? { ...current, columnIds: ordered }
                                  : current
                              );
                          }
                          setDraggingColumn(null);
                          setColumnDrop(null);
                        };
                        window.addEventListener("pointermove", move);
                        window.addEventListener("pointerup", end);
                        window.addEventListener("pointercancel", end);
                      }}
                      onKeyDown={(event) => {
                        const offset =
                          event.key === "ArrowUp"
                            ? -1
                            : event.key === "ArrowDown"
                            ? 1
                            : 0;
                        if (!offset) return;
                        const from = step.columnIds.indexOf(columnId);
                        const to = from + offset;
                        if (to < 0 || to >= step.columnIds.length) return;
                        event.preventDefault();
                        const ordered = [...step.columnIds];
                        [ordered[from], ordered[to]] = [ordered[to], ordered[from]];
                        void savePatch(step.id, (current) =>
                          current.kind === "select"
                            ? { ...current, columnIds: ordered }
                            : current
                        );
                      }}
                    >
                      {column.name}
                    </div>
                  );
                })}
              </div>
            )}

            {step.kind === "select" && step.mode !== "rearrange" && (
              <PipelineCommand
                editorId={commandId(step)}
                label={selectStepLabel(step)}
                initialDraft={columnListCommand(
                  visible
                    .filter((column) => !step.columnIds.includes(column.id))
                    .map((column) => column.id),
                  visible
                )}
                references={columnListReferences}
                focusToken={commandFocus(commandId(step))}
                onChange={(draft) => {
                  const ids = meltedColumnIds(draft, visible);
                  if (ids.length > 0 && ids.length < visible.length)
                    patch(step.id, (current) =>
                      current.kind === "select"
                        ? {
                            ...current,
                            columnIds: visible
                              .filter((column) => !ids.includes(column.id))
                              .map((column) => column.id),
                          }
                        : current
                    );
                }}
                onCommit={(draft) => {
                  const ids = meltedColumnIds(draft, visible);
                  if (ids.length === 0)
                    return rejectCommand("Name at least one column to delete");
                  if (ids.length === visible.length)
                    return rejectCommand("A frame needs at least one column");
                  const kept = visible
                    .filter((column) => !ids.includes(column.id))
                    .map((column) => column.id);
                  return savePatch(step.id, (current) =>
                    current.kind === "select"
                      ? { ...current, columnIds: kept }
                      : current
                  );
                }}
              />
            )}

            {step.kind === "summarize" && (
              <div className="pipeline-command-list">
                {[
                  ...step.groupKeys.map((item) => ({ item, detail: "group" })),
                  ...step.aggregates.map((item) => ({ item, detail: "value" })),
                ].map(({ item, detail }) => {
                  const id = commandId(step, item.id);
                  const update = (draft: string, saveNow: boolean) => {
                    const parsed = parseNamedTransformation(draft);
                    if (!parsed) {
                      if (saveNow)
                        rejectCommand(
                          "Write a backticked output name, =, and a formula"
                        );
                      return;
                    }
                    const change = (current: StepDraft): StepDraft => {
                      if (current.kind !== "summarize") return current;
                      const revise = (candidate: NamedDraft) =>
                        candidate.id === item.id
                          ? { ...candidate, name: parsed.name, formula: parsed.formula }
                          : candidate;
                      return {
                        ...current,
                        groupKeys: current.groupKeys.map(revise),
                        aggregates: current.aggregates.map(revise),
                      };
                    };
                    if (saveNow) return savePatch(step.id, change);
                    patch(step.id, change);
                  };
                  return (
                    <PipelineCommand
                      key={item.id}
                      editorId={id}
                      label={`${
                        detail === "group" ? "Group" : "Aggregate"
                      }: ${draftName(item)}`}
                      detail={detail}
                      initialDraft={namedCommand(draftName(item), item.formula)}
                      references={stepReferences}
                      frameId={input.completionFrameId}
                      scope={scope}
                      focusToken={commandFocus(id)}
                      onChange={(draft) => update(draft, false)}
                      onCommit={(draft) => update(draft, true)}
                    />
                  );
                })}
                <div className="pipeline-item-actions">
                  <button
                    onClick={() => {
                      const item = namedDraft(
                        visible[0]?.name ?? "Group",
                        visible[0] ? formulaToken(visible[0].name) : ""
                      );
                      patch(step.id, (current) =>
                        current.kind === "summarize"
                          ? { ...current, groupKeys: [...current.groupKeys, item] }
                          : current
                      );
                      setPendingEditor(commandId(step, item.id));
                    }}
                  >
                    <Plus size={11} /> Group
                  </button>
                  <button
                    onClick={() => {
                      const item = namedDraft("Aggregate", "");
                      patch(step.id, (current) =>
                        current.kind === "summarize"
                          ? { ...current, aggregates: [...current.aggregates, item] }
                          : current
                      );
                      setPendingEditor(commandId(step, item.id));
                    }}
                  >
                    <Plus size={11} /> Value
                  </button>
                  <label>
                    <input
                      type="checkbox"
                      checked={step.maintainOrder}
                      onChange={(event) =>
                        void savePatch(step.id, (current) =>
                          current.kind === "summarize"
                            ? { ...current, maintainOrder: event.target.checked }
                            : current
                        )
                      }
                    />
                    ordered
                  </label>
                </div>
              </div>
            )}

            {step.kind === "sort" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const keys = parseSortCommand(draft, visible);
                  if (!keys) {
                    if (saveNow)
                      rejectCommand(
                        "Write one or more backticked columns with asc or desc"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "sort" ? { ...current, keys } : current;
                  if (saveNow) return savePatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Sort"
                    initialDraft={sortCommand(step, visible)}
                    references={columnReferences}
                    focusToken={commandFocus(commandId(step))}
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {step.kind === "union" &&
              <PipelineFrameStepCommand
                editorId={commandId(step)}
                label="Stack frame"
                frameId={step.frameId}
                frames={frames}
                focusToken={commandFocus(commandId(step))}
                resolveName={exactName}
                onInvalid={() => rejectCommand("Choose a frame to stack")}
                onSelect={(frameId, saveNow) => {
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "union" ? { ...current, frameId } : current;
                  if (saveNow) return savePatch(step.id, change);
                  patch(step.id, change);
                }}
              />}

            {step.kind === "expand" &&
              <PipelineFrameStepCommand
                editorId={commandId(step)}
                label="Expand frame"
                frameId={step.frameId}
                frames={frames}
                focusToken={commandFocus(commandId(step))}
                resolveName={exactName}
                onInvalid={() => rejectCommand("Choose a frame to expand with")}
                onSelect={(frameId, saveNow) => {
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "expand" ? { ...current, frameId } : current;
                  if (saveNow) return savePatch(step.id, change);
                  patch(step.id, change);
                }}
              />}

            {step.kind === "pivot" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const parsed = parsePivotCommand(draft, visible);
                  if (!parsed) {
                    if (saveNow)
                      rejectCommand(
                        "Write columns=, values=, and a supported aggregate"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "pivot" ? { ...current, ...parsed } : current;
                  if (saveNow) return savePatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Pivot"
                    initialDraft={pivotCommand(step, visible)}
                    references={columnReferences}
                    focusToken={commandFocus(commandId(step))}
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {step.kind === "unpivot" &&
              (() => {
                const update = (draft: string, saveNow: boolean) => {
                  const parsed = parseUnpivotCommand(draft);
                  if (!parsed) {
                    if (saveNow)
                      rejectCommand(
                        "Write columns=, names=, and values= with backticked output names"
                      );
                    return;
                  }
                  const change = (current: StepDraft): StepDraft =>
                    current.kind === "unpivot" ? { ...current, ...parsed } : current;
                  if (saveNow) return savePatch(step.id, change);
                  patch(step.id, change);
                };
                return (
                  <PipelineCommand
                    editorId={commandId(step)}
                    label="Unpivot"
                    initialDraft={unpivotCommand(step)}
                    references={columnListReferences}
                    focusToken={commandFocus(commandId(step))}
                    focusSelection={
                      step.columns.trim()
                        ? undefined
                        : { start: "columns=".length, end: "columns=".length }
                    }
                    onChange={(draft) => update(draft, false)}
                    onCommit={(draft) => update(draft, true)}
                  />
                );
              })()}

            {stepFailure && !stepIsIncomplete(step) && (
              <p className="pipeline-step-shape broken">
                <CircleAlert size={11} /> {stepFailure}
              </p>
            )}
          </section>
        );
      })}

      <label className="add-step compact-add-step">
        <select
          value=""
          aria-label="Add transformation"
          onChange={(event) => {
            const kind = event.target.value as AddStepKind;
            if (!kind) return;
            const step = blankStep(
              kind,
              columnsBeforeStep(input.columns, steps, steps.length),
              input.columns
            );
            setSteps((current) => [...current, step]);
            const itemId =
              step.kind === "filter"
                ? step.predicates[0]?.id
                : step.kind === "withColumns"
                ? step.columns[0]?.id
                : step.kind === "summarize"
                ? step.aggregates[0]?.id
                : undefined;
            setPendingEditor(commandId(step, itemId));
          }}
        >
          <option value="">+ Add transformation</option>
          <option value="filter">Filter rows</option>
          <option value="withColumns">Add or replace columns</option>
          <option value="deleteColumns">Delete columns</option>
          <option value="rearrangeColumns">Rearrange columns</option>
          <option value="summarize">Summarize</option>
          <option value="sort">Sort</option>
          <option value="union">Stack frame</option>
          <option value="expand">Expand frame</option>
          <option value="pivot">Pivot</option>
          <option value="unpivot">Unpivot</option>
          <option value="comment">Comment</option>
        </select>
      </label>

      {formulaError && (
        <FormulaErrorDetails
          error={formulaError ?? ""}
          formulas={steps.flatMap(stepFormulas)}
        />
      )}
    </div>
  );
}
