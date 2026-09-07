import { aliasFromFormula } from "./lib/formulaAlias";

/** One named expression as the Wrangle editor holds it before save. */
export type NamedDraft = {
  id: string;
  outputColumnId: string;
  /** Empty means the person has not named it, not that it has no name. */
  name: string;
  formula: string;
  fallbackName: string;
  focusToken?: number;
  focusAtEnd?: boolean;
  anchorRowIndex?: number;
};

export function draftName(draft: NamedDraft): string {
  return draft.name.trim() || suggestedName(draft);
}

export function suggestedName(draft: NamedDraft): string {
  return aliasFromFormula(draft.formula) || draft.fallbackName;
}

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

export function nextBlankColumnName(existing: string[]): string {
  let largest = 0;
  for (const name of existing) {
    const match = name.match(/^Column (\d+)$/);
    if (match) largest = Math.max(largest, Number(match[1]));
  }
  return `Column ${largest + 1}`;
}

export function exactName(token: string): string | null {
  const trimmed = token.trim();
  if (!trimmed.startsWith("`") || !trimmed.endsWith("`")) return null;
  return trimmed.slice(1, -1).replaceAll("``", "`");
}

/**
 * A name written without its backticks. Nobody reading "name it Profit and
 * enter `Revenue` - `Cost`" types the quoting, and refusing `Profit = …`
 * taught the syntax by rejection — the worst way to teach it. Accepted only
 * for text that could not be an expression instead: a word, or several
 * words, of letters, digits and underscores, starting with a letter or an
 * underscore. Anything with an operator, a call, a literal or a leading
 * digit in it is still refused, so `count(x) = 1` cannot quietly become a
 * column called something nobody typed. The name is stored unquoted and the
 * command reprints it backticked, so the saved chain is canonical either
 * way.
 */
function plainName(token: string): string | null {
  const trimmed = token.trim();
  return /^[A-Za-z_][A-Za-z0-9_]*(?: [A-Za-z0-9_]+)*$/.test(trimmed)
    ? trimmed
    : null;
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
    const written = source.slice(0, index);
    const name = exactName(written) ?? plainName(written);
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
