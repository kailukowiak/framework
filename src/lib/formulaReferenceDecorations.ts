import type { FormulaReference } from "./formulaReferences";

export const FORMULA_REFERENCE_COLOR_COUNT = 6;

export type FormulaReferenceDecoration = {
  start: number;
  end: number;
  colorIndex: number;
  reference: FormulaReference;
  /** Positive means an earlier row, matching the public `.shift(n)` syntax. */
  rowOffset: number;
  /** An explicit zero-based Scratchwork cell or slice address. */
  rowRange?: { start: number; end: number };
};

type Candidate = {
  text: string;
  reference: FormulaReference;
  bare: boolean;
};

/** Text ranges that are values or comments rather than reference syntax. */
function protectedRanges(source: string): Array<{ start: number; end: number }> {
  const ranges: Array<{ start: number; end: number }> = [];
  const declarations =
    /^\s*(?:`(?:[^`]|``)*`|[\p{L}_][\p{L}\p{N}_]*)\s*=(?!=)/gmu;
  for (const declaration of source.matchAll(declarations))
    ranges.push({
      start: declaration.index,
      end: declaration.index + declaration[0].length,
    });
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (character === "`") {
      index += 1;
      while (index < source.length) {
        if (source[index] !== "`") index += 1;
        else if (source[index + 1] === "`") index += 2;
        else break;
      }
      continue;
    }
    if (character === "#") {
      const end = source.indexOf("\n", index);
      ranges.push({ start: index, end: end < 0 ? source.length : end });
      index = (end < 0 ? source.length : end) - 1;
      continue;
    }
    if (character !== '"' && character !== "'") continue;
    const quote = character;
    const start = index;
    index += 1;
    while (index < source.length) {
      if (source[index] === "\\") index += 2;
      else if (source[index] === quote) {
        index += 1;
        break;
      } else index += 1;
    }
    ranges.push({ start, end: Math.min(index, source.length) });
    index -= 1;
  }
  return ranges.sort((left, right) => left.start - right.start);
}

function candidatesFor(references: FormulaReference[]): Candidate[] {
  const candidates: Candidate[] = [];
  for (const reference of references) {
    if (reference.kind === "function") continue;
    const canonical = reference.token.endsWith(".")
      ? reference.token.slice(0, -1)
      : reference.token;
    if (canonical)
      candidates.push({ text: canonical, reference, bare: false });
    // Forgiving local names are legal formula syntax too. Qualified labels
    // are deliberately excluded: the final component on its own may resolve
    // to a different local column, and painting it as the remote object would
    // teach a false dependency.
    if (
      !reference.label.includes(".") &&
      /^[\p{L}_][\p{L}\p{N}_]*$/u.test(reference.label) &&
      reference.label !== canonical
    ) {
      candidates.push({ text: reference.label, reference, bare: true });
    }
  }
  return candidates.sort((left, right) => right.text.length - left.text.length);
}

function isIdentifierCharacter(character: string | undefined): boolean {
  return Boolean(character && /[\p{L}\p{N}_]/u.test(character));
}

function candidateMatches(source: string, index: number, candidate: Candidate): boolean {
  const slice = source.slice(index, index + candidate.text.length);
  if (
    candidate.bare
      ? slice.toLocaleLowerCase() !== candidate.text.toLocaleLowerCase()
      : slice !== candidate.text
  ) {
    return false;
  }
  const simpleIdentifier = /^[\p{L}_][\p{L}\p{N}_]*$/u.test(candidate.text);
  if (!candidate.bare && !simpleIdentifier) return true;
  return (
    source[index - 1] !== "." &&
    !isIdentifierCharacter(source[index - 1]) &&
    !isIdentifierCharacter(source[index + candidate.text.length])
  );
}

function rowOffsetAfter(source: string, end: number): number {
  const shift = /^\s*\.shift\(\s*(-?\d+)\s*\)/.exec(source.slice(end));
  return shift ? Number(shift[1]) : 0;
}

export function rowRangeAfter(
  source: string,
  end: number
): { start: number; end: number } | undefined {
  const suffix = source.slice(end);
  const scalar = /^\s*\.head\(\s*(\d+)\s*\)\s*\.last\(\s*\)/.exec(suffix);
  if (!scalar) return undefined;
  const row = Number(scalar[1]) - 1;
  return row >= 0 ? { start: row, end: row } : undefined;
}

/**
 * Resolve the reference-shaped parts of an unfinished formula.
 *
 * This is intentionally a tolerant decoration pass, not a second parser.
 * The core remains the authority on whether the whole expression is valid;
 * this pass only recognizes exact tokens the editor itself offers, plus the
 * simple unquoted local names the core accepts. That lets feedback keep up
 * while someone is halfway through typing without inventing new semantics.
 */
export function formulaReferenceDecorations(
  source: string,
  references: FormulaReference[]
): FormulaReferenceDecoration[] {
  const protectedParts = protectedRanges(source);
  const candidates = candidatesFor(references);
  const colors = new Map<string, number>();
  const decorations: FormulaReferenceDecoration[] = [];
  let protectedIndex = 0;
  for (let index = 0; index < source.length; ) {
    const protectedPart = protectedParts[protectedIndex];
    if (protectedPart && index >= protectedPart.end) {
      protectedIndex += 1;
      continue;
    }
    if (protectedPart && index >= protectedPart.start) {
      index = protectedPart.end;
      continue;
    }
    const candidate = candidates.find((item) => candidateMatches(source, index, item));
    if (!candidate) {
      index += 1;
      continue;
    }
    let colorIndex = colors.get(candidate.reference.id);
    if (colorIndex === undefined) {
      colorIndex = colors.size % FORMULA_REFERENCE_COLOR_COUNT;
      colors.set(candidate.reference.id, colorIndex);
    }
    const end = index + candidate.text.length;
    decorations.push({
      start: index,
      end,
      colorIndex,
      reference: candidate.reference,
      rowOffset: rowOffsetAfter(source, end),
      rowRange: rowRangeAfter(source, end),
    });
    index = end;
  }
  return decorations;
}
