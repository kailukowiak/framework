import { exactName } from "../PipelineColumnNames";
import { formulaToken } from "./formulaReferences";
import type { StepDraft, VisibleColumn } from "./pipelineSteps";
import type { PivotAggregate } from "./types";

/** Text on the left of a transformation assignment is an identifier too. */
export function namedCommand(name: string, formula: string): string {
  return `${formulaToken(name)} = ${formula}`;
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

/** Intentional name reuse means overwrite; a new name keeps its minted id. */
export function outputColumnIdForName(
  visible: Array<{ id: string; name: string }>,
  currentId: string,
  name: string
): string {
  return visible.find((candidate) => candidate.name === name)?.id ?? currentId;
}

export function sortCommand(
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

export function columnListCommand(ids: string[], visible: VisibleColumn[]): string {
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

export function pivotCommand(
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

export function unpivotCommand(step: Extract<StepDraft, { kind: "unpivot" }>): string {
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
