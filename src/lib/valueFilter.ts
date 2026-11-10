import { formulaToken } from "./formulaReferences";
import type { StepDraft } from "./pipelineSteps";
import type { Column } from "./types";

/**
 * The value-checklist filter, as the one kind of Filter step the picker is
 * willing to read back.
 *
 * There is still exactly one home for a filter: a predicate in a Filter step
 * of the Wrangle chain. The picker does not add a second representation, it
 * only writes one *spelling* of that predicate and recognises that spelling
 * when it comes back. That is what lets re-opening the picker show the
 * current checks without a side table that could disagree with the chain,
 * and it is also why anything else -- `Region == "East"`, a range, an
 * expression over two columns -- is left strictly alone: guessing which
 * boxes an arbitrary predicate "means" would be inventing a filter.
 *
 * The spelling is the engine's own canonical echo (a backticked column,
 * `.is_in([...])`, a JSON-escaped string, `|` between the pieces, `None`
 * never `null`), because a client-authored chain formula that differs from
 * what the engine echoes makes the save re-seed the chain and sever any
 * sync session. Written once here so no caller formats by hand.
 */

/** One distinct value of a column, as a checklist row. */
export type DistinctValue = {
  value: string | number | boolean | null;
  /** Rows holding it, when the source can say cheaply. */
  count?: number;
};

export type DistinctValues = {
  /** Commonest first, already capped by the source. */
  values: DistinctValue[];
  /**
   * Distinct values in the whole column -- or, for a query, the ones that
   * matched it -- so truncation can be reported.
   */
  total: number;
};

/**
 * Where the distinct values must come from: the engine, over the whole
 * column on the data layer (the chain without its own display filter, so a
 * value this picker has unchecked stays offered and can be checked again).
 */
export type LoadDistinctValues = (
  frameId: string,
  columnId: string,
  /** Values whose text contains this, ignoring case, matched before the cap. */
  query?: string
) => Promise<DistinctValues>;

/** A stable identity for a value across string/number lookalikes. */
export function valueKey(value: DistinctValue["value"]): string {
  return value === null ? "null" : `${typeof value}:${String(value)}`;
}

function literal(value: Exclude<DistinctValue["value"], null>): string {
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "boolean") return value ? "True" : "False";
  return numberText(value);
}

/**
 * A number the way the engine writes it back: a whole number without `.0`,
 * a fraction to four places with trailing zeros dropped. `String(n)` would
 * differ (`1e-7`, `0.12345`), and the engine's echo of a client-authored
 * formula must be identical to what was sent or the save re-seeds the chain.
 */
export function numberText(value: number): string {
  if (Math.abs(value % 1) < Number.EPSILON) return value.toFixed(0);
  return value.toFixed(4).replace(/0+$/, "").replace(/\.$/, "");
}

/**
 * Whether a predicate can carry every one of these values exactly. A number
 * with more than four decimals reads back from the engine rounded, so a
 * filter written from it would keep a different value than the one checked.
 */
export function writable(values: DistinctValue["value"][]): boolean {
  return values.every(
    (value) => typeof value !== "number" || Number(numberText(value)) === value
  );
}

/** The predicate keeping rows whose `column` is one of `values`. */
export function valueFilterPredicate(
  column: Pick<Column, "name">,
  values: DistinctValue["value"][]
): string {
  const token = formulaToken(column.name);
  const present = values.filter(
    (value): value is Exclude<DistinctValue["value"], null> => value !== null
  );
  const hasNull = present.length !== values.length;
  const members = `${token}.is_in([${present.map(literal).join(", ")}])`;
  if (!hasNull) return members;
  const blank = `${token}.is_null()`;
  return present.length === 0 ? blank : `${members} | ${blank}`;
}

function parseLiteral(text: string): DistinctValue["value"] | undefined {
  const trimmed = text.trim();
  if (trimmed === "True") return true;
  if (trimmed === "False") return false;
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed);
  if (trimmed.startsWith('"')) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      return typeof parsed === "string" ? parsed : undefined;
    } catch {
      return undefined;
    }
  }
  return undefined;
}

/** Splits a list body on commas that are outside string literals. */
function splitList(body: string): string[] | undefined {
  const items: string[] = [];
  let current = "";
  let inString = false;
  for (let index = 0; index < body.length; index += 1) {
    const char = body[index];
    if (inString) {
      current += char;
      if (char === "\\") {
        index += 1;
        current += body[index] ?? "";
      } else if (char === '"') inString = false;
    } else if (char === '"') {
      inString = true;
      current += char;
    } else if (char === ",") {
      items.push(current);
      current = "";
    } else current += char;
  }
  if (inString) return undefined;
  if (current.trim() !== "" || items.length > 0) items.push(current);
  return items;
}

/**
 * The values a predicate keeps, when it is exactly the picker's spelling for
 * this column; `undefined` for anything else.
 */
export function readValueFilterPredicate(
  written: string,
  column: Pick<Column, "name">
): DistinctValue["value"][] | undefined {
  // A saved chain comes back through the chain formatter, which puts each
  // member on its own indented line. That is whitespace only, and a string
  // literal cannot hold a raw newline (it is JSON-escaped), so flattening
  // every line break is safe -- and necessary, or the picker would stop
  // recognising its own filter the moment the chain was saved.
  const predicate = written.replace(/\s*\n\s*/g, "");
  const token = formulaToken(column.name);
  const blank = `${token}.is_null()`;
  const head = `${token}.is_in([`;
  if (predicate === blank) return [null];
  let body = predicate;
  let hasNull = false;
  if (body.endsWith(` | ${blank}`)) {
    hasNull = true;
    body = body.slice(0, -` | ${blank}`.length);
  }
  if (!body.startsWith(head) || !body.endsWith("])")) return undefined;
  const items = splitList(body.slice(head.length, -2));
  if (!items) return undefined;
  const values: DistinctValue["value"][] = [];
  for (const item of items) {
    const value = parseLiteral(item);
    if (value === undefined) return undefined;
    values.push(value);
  }
  // Round trip or nothing: a hand-edited list the writer would spell
  // differently is somebody's formula, not the picker's.
  const rewritten = valueFilterPredicate(column, hasNull ? [...values, null] : values);
  return rewritten === predicate ? (hasNull ? [...values, null] : values) : undefined;
}

export type ValueFilterState =
  /** No predicate on this column in the trailing filter. */
  | { kind: "none" }
  /** Written by the picker; these values are kept. */
  | { kind: "picker"; values: DistinctValue["value"][] }
  /** Something else mentions the column; the picker must not touch it. */
  | { kind: "foreign" };

function trailingFilter(steps: StepDraft[]) {
  const last = steps.at(-1);
  return last?.kind === "filter" ? last : undefined;
}

/** What the trailing filter currently says about `column`. */
export function readValueFilter(
  steps: StepDraft[],
  column: Pick<Column, "name">
): ValueFilterState {
  const step = trailingFilter(steps);
  if (!step) return { kind: "none" };
  const token = formulaToken(column.name);
  const mentioning = step.predicates.filter((predicate) =>
    predicate.formula.includes(token)
  );
  if (mentioning.length === 0) return { kind: "none" };
  // An OR chain or several predicates on the column cannot be summarised by
  // one checklist without changing what they mean.
  if (!step.matchAll || mentioning.length > 1) return { kind: "foreign" };
  const values = readValueFilterPredicate(mentioning[0].formula, column);
  return values ? { kind: "picker", values } : { kind: "foreign" };
}

/**
 * The chain after committing a checklist: `kept` is every value to keep, or
 * `null` for "everything", which removes the picker's predicate rather than
 * writing a filter that keeps all.
 */
export function writeValueFilter(
  steps: StepDraft[],
  column: Pick<Column, "name">,
  kept: DistinctValue["value"][] | null
): StepDraft[] {
  const state = readValueFilter(steps, column);
  if (state.kind === "foreign") return steps;
  const step = trailingFilter(steps);
  const token = formulaToken(column.name);
  const formula = kept === null ? null : valueFilterPredicate(column, kept);
  if (!step) {
    if (formula === null) return steps;
    return [
      ...steps,
      {
        id: crypto.randomUUID(),
        kind: "filter",
        predicates: [{ id: crypto.randomUUID(), formula }],
        matchAll: true,
      },
    ];
  }
  const existing = step.predicates.findIndex((predicate) =>
    predicate.formula.includes(token)
  );
  let predicates = step.predicates;
  if (existing >= 0 && formula !== null)
    predicates = predicates.map((predicate, index) =>
      index === existing ? { ...predicate, formula } : predicate
    );
  else if (existing >= 0)
    predicates = predicates.filter((_, index) => index !== existing);
  else if (formula !== null) {
    // An OR-combined trailing filter would change meaning if joined by AND.
    if (!step.matchAll)
      return [
        ...steps,
        {
          id: crypto.randomUUID(),
          kind: "filter",
          predicates: [{ id: crypto.randomUUID(), formula }],
          matchAll: true,
        },
      ];
    predicates = [...predicates, { id: crypto.randomUUID(), formula }];
  }
  if (predicates === step.predicates) return steps;
  // Removing the last predicate removes the step: an empty Filter is an
  // incomplete one, and the picker must never leave the chain unsaveable.
  return predicates.length === 0
    ? steps.slice(0, -1)
    : [...steps.slice(0, -1), { ...step, predicates }];
}
