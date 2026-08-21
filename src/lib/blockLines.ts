/**
 * The scratchpad's continuation rule, mirrored from the core
 * (`formula/line.rs::logical_lines`). An indented physical line continues
 * the line above it, and an explicit open delimiter keeps every physical
 * line through its matching close in the same calculation. The delimiter
 * matters because indentation should be free to describe layout without a
 * closing `)` at the margin accidentally becoming a new calculation.
 *
 * The frontend needs the same rule the core applies because everything the
 * card does per line — the gutter row an answer lands in, the line said to
 * be under the cursor, the stripe a formula sits on — is per *logical*
 * line, while the textarea only knows physical rows.
 */

/** Physical row index → logical line index, one entry per physical row. */
export function physicalToLogical(source: string): number[] {
  const map: number[] = [];
  let logical = -1;
  let currentBlank = true;
  let delimiterDepth = 0;
  for (const row of source.split("\n")) {
    const continues =
      logical >= 0 &&
      (delimiterDepth > 0 ||
        (/^[ \t]/.test(row) && row.trim() !== "" && !currentBlank));
    if (!continues) {
      logical += 1;
      currentBlank = row.trim() === "";
      delimiterDepth = 0;
    }
    map.push(logical);
    delimiterDepth = delimiterDepthAfter(row, delimiterDepth);
  }
  return map;
}

/**
 * Count structural delimiters without letting a bracket printed in a string
 * or backticked name reshape the editor. This deliberately tolerates an
 * incomplete draft: unmatched closes stop at zero, while an unmatched open
 * simply keeps the next physical row with the expression being written.
 */
function delimiterDepthAfter(source: string, initial: number): number {
  if (initial === 0 && source.trimStart().startsWith("#")) return 0;
  let depth = initial;
  let quote: "'" | '"' | "`" | null = null;
  let escaped = false;
  for (const character of source) {
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quote) {
      if (quote !== "`" && character === "\\") escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "'" || character === '"' || character === "`") {
      quote = character;
    } else if (character === "(" || character === "[" || character === "{") {
      depth += 1;
    } else if (character === ")" || character === "]" || character === "}") {
      depth = Math.max(0, depth - 1);
    }
  }
  return depth;
}

export type FormulaContinuation = {
  source: string;
  selection: { start: number; end: number };
};

/**
 * Expand one calculation for a second physical line.
 *
 * The first Alt+Return makes the grouping visible instead of asking
 * indentation to carry syntax by itself. A scratchwork name stays outside
 * the parentheses, so `total = ...` remains a named calculation, and the
 * closing parenthesis is left at the margin so a terminal method can follow:
 *
 * ```text
 * total = (
 *   values
 *   .filter(condition)
 * ).sum()
 * ```
 *
 * Once a calculation is already expanded, Alt+Return is just another
 * indented visual break inside its explicit boundary.
 */
export function continueFormula(
  source: string,
  selectionStart: number,
  selectionEnd: number
): FormulaContinuation {
  const start = Math.max(0, Math.min(selectionStart, source.length));
  const end = Math.max(start, Math.min(selectionEnd, source.length));
  if (source.includes("\n")) {
    const next = `${source.slice(0, start)}\n  ${source.slice(end)}`;
    const at = start + 3;
    return { source: next, selection: { start: at, end: at } };
  }

  const expressionStart = scratchworkExpressionStart(source);
  const at = Math.max(expressionStart, start);
  const before = source.slice(expressionStart, at);
  const after = source.slice(Math.max(expressionStart, end));
  const prefix = source.slice(0, expressionStart);
  const next = `${prefix}(\n  ${before}\n  ${after}\n)`;
  const caret = prefix.length + 4 + before.length + 3;
  return { source: next, selection: { start: caret, end: caret } };
}

/** Match the core's deliberately narrow `name = expression` convention. */
export function scratchworkExpressionStart(source: string): number {
  const match = /^(\s*(?:[\p{L}_][\p{L}\p{N}_]*(?: [\p{L}\p{N}_]+)*|`(?:[^`]|``)+`)\s*=\s*)/u.exec(
    source
  );
  if (!match) return source.startsWith("=") ? 1 : 0;
  const equals = match[0].indexOf("=");
  const before = source[equals - 1];
  const after = source[equals + 1];
  if (before === "<" || before === ">" || before === "!" || after === "=") return 0;
  return match[0].length;
}

/** The logical line the character at `offset` sits on. */
export function logicalLineIndexAt(source: string, offset: number): number {
  const row = source.slice(0, offset).split("\n").length - 1;
  return physicalToLogical(source)[row] ?? 0;
}

/** Physical rows per logical line, in order. */
export function logicalLineSpans(source: string): number[] {
  const spans: number[] = [];
  for (const logical of physicalToLogical(source)) {
    spans[logical] = (spans[logical] ?? 0) + 1;
  }
  return spans;
}

/** The text regrouped: one string per logical line, newlines kept inside. */
export function logicalGroups(source: string): string[] {
  const rows = source.split("\n");
  const map = physicalToLogical(source);
  const groups: string[] = [];
  rows.forEach((row, index) => {
    const logical = map[index];
    groups[logical] = groups[logical] === undefined ? row : `${groups[logical]}\n${row}`;
  });
  return groups;
}

export type ScratchworkStripeRow = {
  id: string;
  span: number;
  on: boolean;
};

/**
 * Alternating bands for the draft that is actually under the caret.
 *
 * The computed block arrives after the debounced edit. Building bands from
 * that older text lets a newly inserted row's background slide over the live
 * caret. Logical groups are the stable unit: adding physical rows inside an
 * open parenthesized formula grows its existing band instead of changing the
 * alternation below it.
 */
export function scratchworkStripeRows(
  source: string,
  persistedIds: string[] = []
): ScratchworkStripeRow[] {
  let band = 0;
  return logicalGroups(source).map((text, index) => {
    const span = text.split("\n").length;
    if (!text.trim())
      return { id: persistedIds[index] ?? `draft-${index}`, span, on: false };
    band += 1;
    return {
      id: persistedIds[index] ?? `draft-${index}`,
      span,
      on: band % 2 === 0,
    };
  });
}
