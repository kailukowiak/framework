import type { FormulaArgument } from "./types";

export interface FormulaReference {
  id: string;
  label: string;
  token: string;
  kind: "frame" | "column" | "value" | "function";
  detail: string;
  /** Canvas object that can be outlined while this reference is edited. */
  objectId?: string;
  /** Frame that owns a column, needed to make row-relative highlighting honest. */
  frameId?: string;
  searchTerms?: string[];
  /** Catalog documentation, present only for function references. */
  signature?: string;
  description?: string;
  /** Canonical argument guidance, supplied by the Rust formula catalog. */
  arguments?: FormulaArgument[];
}

export function formulaToken(name: string): string {
  return `\`${name.replaceAll("`", "``")}\``;
}

export function getFormulaReferenceQuery(source: string, cursor: number): string {
  const beforeCursor = source.slice(0, cursor);
  let quotedStart = -1;
  let inQuotedReference = false;
  for (let index = 0; index < beforeCursor.length; index += 1) {
    if (beforeCursor[index] !== "`") continue;
    if (inQuotedReference && beforeCursor[index + 1] === "`") {
      index += 1;
      continue;
    }
    inQuotedReference = !inQuotedReference;
    if (inQuotedReference) quotedStart = index;
  }
  if (inQuotedReference) return beforeCursor.slice(quotedStart);
  return beforeCursor.match(/\.[\p{L}\p{N}_.]*$|[\p{L}\p{N}_]*$/u)?.[0] ?? "";
}

export function filterFormulaReferences(
  references: FormulaReference[],
  query: string
): FormulaReference[] {
  const normalized = normalize(query.split(".").at(-1) ?? query);
  if (!normalized) return references;

  return references
    .filter((reference) => {
      const token = normalize(reference.token);
      const label = normalize(reference.label);
      return (
        token.includes(normalized) ||
        label.includes(normalized) ||
        reference.searchTerms?.some((term) => normalize(term).includes(normalized))
      );
    })
    .sort((left, right) => {
      const leftStarts = normalize(left.token).startsWith(normalized) ? 0 : 1;
      const rightStarts = normalize(right.token).startsWith(normalized) ? 0 : 1;
      return leftStarts - rightStarts || left.label.localeCompare(right.label);
    });
}

/**
 * The local completion state after a frame namespace has been accepted.
 *
 * Typed completion normally answers this from core. Some formula surfaces do
 * not have a frame scope, though, and even scoped editors briefly render their
 * local references while the next typed response is in flight. The trailing
 * dot must mean the same thing in both cases: offer that frame's columns and
 * insert only the member after the already-written qualifier.
 */
export function contextualFormulaReferenceCompletion(
  references: FormulaReference[],
  source: string,
  cursor: number,
  query: string
): { qualifier?: FormulaReference; suggestions: FormulaReference[] } {
  const qualifier = frameQualifierAt(references, source, cursor, query);
  if (!qualifier)
    return { suggestions: filterFormulaReferences(references, query) };
  const partial = query.slice(1);
  return {
    qualifier,
    suggestions: references.filter(
      (reference) =>
        reference.kind === "column" &&
        (reference.frameId === qualifier.id ||
          (!reference.frameId && reference.token.startsWith(qualifier.token))) &&
        filterFormulaReferences([reference], partial).length > 0
    ),
  };
}

export function contextualFormulaReferenceToken(
  reference: FormulaReference,
  qualifier?: FormulaReference
): string {
  if (!qualifier) return reference.token;
  const member = reference.token.startsWith(qualifier.token)
    ? reference.token.slice(qualifier.token.length)
    : reference.token;
  return `.${member}`;
}

function frameQualifierAt(
  references: FormulaReference[],
  source: string,
  cursor: number,
  query: string
): FormulaReference | undefined {
  if (!query.startsWith(".")) return undefined;
  const beforeCursor = source.slice(0, cursor);
  const memberLength = query.length - 1;
  const beforeMember = memberLength
    ? beforeCursor.slice(0, -memberLength)
    : beforeCursor;
  return references
    .filter(
      (reference) =>
        reference.kind === "frame" && beforeMember.endsWith(reference.token)
    )
    .sort((left, right) => right.token.length - left.token.length)[0];
}

/** The exact token this editor already declares for a clicked column. */
export function columnReferenceForPick(
  references: FormulaReference[],
  columnId: string
): FormulaReference | null {
  return (
    references.find(
      (reference) => reference.kind === "column" && reference.id === columnId
    ) ?? null
  );
}

/** Turn a cell pick into the column expression it demonstrates at the anchor. */
export function columnTokenForCellPick(
  token: string,
  anchorRowIndex: number | undefined,
  pickedRowIndex: number | undefined
): string {
  if (anchorRowIndex === undefined || pickedRowIndex === undefined) return token;
  const periods = anchorRowIndex - pickedRowIndex;
  return periods === 0 ? token : `${token}.shift(${periods})`;
}

/** One clicked Scratchwork cell, reduced from its frame column by row. */
export function scalarTokenForCellPick(token: string, pickedRowIndex: number): string {
  return `${token}.head(${pickedRowIndex + 1}).last()`;
}

/**
 * Where the text after an inserted reference resumes.
 *
 * A reference the user opened may already have its closing backtick waiting
 * past the cursor — typing over an existing one leaves it there. The token
 * brings its own, so that one is consumed rather than left behind as the
 * second half of `Name``.
 *
 * Shared because there are two insertion paths — the local reference list
 * and the backend's typed completion — and a rule that lives in only one of
 * them is a rule that half works.
 */
export function insertionResumesAt(
  source: string,
  cursor: number,
  insertText: string
): number {
  return insertText.endsWith("`") && source[cursor] === "`" ? cursor + 1 : cursor;
}

export function insertFormulaReference(
  source: string,
  cursor: number,
  token: string
): { source: string; cursor: number } {
  const query = getFormulaReferenceQuery(source, cursor);
  const start = cursor - query.length;
  const updated = `${source.slice(0, start)}${token}${source.slice(
    insertionResumesAt(source, cursor, token)
  )}`;
  return { source: updated, cursor: start + token.length };
}

export function isFormulaExecuteShortcut(event: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
}): boolean {
  return event.key === "Enter" && (event.metaKey || event.ctrlKey);
}

function normalize(value: string): string {
  return value.replace(/[^\p{L}\p{N}]/gu, "").toLocaleLowerCase();
}
