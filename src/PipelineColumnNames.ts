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
