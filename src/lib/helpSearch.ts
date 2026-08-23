import type { FormulaFunction } from "./types";
import { HELP_GUIDES, type HelpGuide, type HelpScope } from "./helpContent";

export type FormulaHelpEntry = {
  kind: "Function";
  id: string;
  title: string;
  summary: string;
  function: FormulaFunction;
  copyText: string;
  surfaces: string[];
};

export type GuideHelpEntry = {
  kind: "Guide" | "Rule";
  id: string;
  title: string;
  summary: string;
  guide: HelpGuide;
};

export type HelpEntry = FormulaHelpEntry | GuideHelpEntry;

const INTENT_CLUSTERS = [
  [
    "list", "lists", "array", "arrays", "vector", "vectors", "series", "range",
    "ranges", "iterator", "iterate", "collection", "sequence", "generator",
  ],
  ["loop", "recursive", "recursion", "recurrence", "previous", "running", "cumulative"],
  ["lookup", "vlookup", "xlookup", "join", "relationship", "match"],
  ["blank", "empty", "missing", "null", "none"],
] as const;

const FUNCTION_EXAMPLES: Record<string, string> = {
  "root.sequence": "sequence(1, 13)",
  "root.recur": "recur(`Opening`, previous() + `Change`)",
  "root.previous": "previous()",
  "root.frame_len": "frame.len()",
  "root.coalesce": "coalesce([`Actual`, `Budget`])",
  "root.when": "when(`Amount` > 100).then(\"High\").otherwise(\"Low\")",
  "root.date": "date(2026, 1, 31)",
  "expr.at": "`months`.at(3)",
  "expr.is_between": "`Amount`.is_between(0, 100)",
  "expr.is_in": "`Region`.is_in([\"West\", \"North\"])",
  "expr.filter": "`Amount`.filter(`Region` == \"West\").sum()",
  "expr.shift": "`Revenue`.shift(1)",
  "expr.fill_null": "`Amount`.fill_null(0)",
};

const STOP_WORDS = new Set(["a", "an", "and", "for", "how", "i", "in", "is", "of", "the", "to"]);

const normalize = (value: string) =>
  value
    .normalize("NFKD")
    .replace(/[^\p{L}\p{N}_.]+/gu, " ")
    .trim()
    .toLocaleLowerCase();

const words = (value: string) =>
  normalize(value)
    .split(/[\s._]+/)
    .filter((word) => word && !STOP_WORDS.has(word));

function expandedWords(query: string): string[] {
  const original = words(query);
  const expanded = new Set(original);
  for (const cluster of INTENT_CLUSTERS) {
    if (cluster.some((term) => original.includes(term)))
      cluster.forEach((term) => expanded.add(term));
  }
  return [...expanded];
}

function editDistanceWithin(left: string, right: string, limit = 2): boolean {
  if (Math.abs(left.length - right.length) > limit) return false;
  const row = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    let previous = row[0];
    row[0] = leftIndex;
    let rowMinimum = row[0];
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      const above = row[rightIndex];
      row[rightIndex] = Math.min(
        row[rightIndex] + 1,
        row[rightIndex - 1] + 1,
        previous + (left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1)
      );
      previous = above;
      rowMinimum = Math.min(rowMinimum, row[rightIndex]);
    }
    if (rowMinimum > limit) return false;
  }
  return row[right.length] <= limit;
}

function functionCopyText(fn: FormulaFunction): string {
  const curated = FUNCTION_EXAMPLES[fn.id];
  if (curated) return curated;
  if (fn.signature.startsWith(".")) return `\`Column\`${fn.signature}`;
  return fn.signature;
}

function functionSurfaces(fn: FormulaFunction): string[] {
  if (fn.id === "root.sequence") return ["Variable", "Scratchwork", "Ordered Wrangle fill"];
  if (
    fn.id === "root.recur" ||
    fn.id === "root.previous" ||
    fn.id === "expr.shift" ||
    fn.id.startsWith("expr.cum_")
  )
    return ["Ordered Wrangle"];
  if (fn.id === "expr.at") return ["Variable", "Scratchwork"];
  return ["Formula editors"];
}

export function helpEntries(
  scope: HelpScope,
  formulaFunctions: FormulaFunction[]
): HelpEntry[] {
  const guides: GuideHelpEntry[] = HELP_GUIDES.filter((guide) =>
    guide.scopes.includes(scope)
  ).map((guide) => ({
    kind: guide.kind,
    id: guide.id,
    title: guide.title,
    summary: guide.summary,
    guide,
  }));
  if (scope === "guide") return guides;
  return [
    ...guides,
    ...formulaFunctions.map((fn): FormulaHelpEntry => ({
      kind: "Function",
      id: fn.id,
      title: fn.signature,
      summary: fn.description,
      function: fn,
      copyText: functionCopyText(fn),
      surfaces: functionSurfaces(fn),
    })),
  ];
}

function entryFields(entry: HelpEntry): Array<[string, number]> {
  if (entry.kind === "Function")
    return [
      [entry.function.name, 120],
      [entry.function.signature, 105],
      [entry.function.aliases.join(" "), 95],
      [entry.function.category, 55],
      [entry.function.description, 35],
      [entry.function.returnType, 20],
    ];
  const { guide } = entry;
  return [
    [guide.title, 120],
    [guide.questions?.join(" ") ?? "", 105],
    [guide.searchTerms.join(" "), 90],
    [guide.summary, 55],
    [guide.body.join(" "), 30],
    [guide.related?.join(" ") ?? "", 20],
  ];
}

function scoreEntry(entry: HelpEntry, query: string): number | null {
  const normalizedQuery = normalize(query);
  const original = words(query);
  const expanded = expandedWords(query);
  const fields = entryFields(entry);
  if (!normalizedQuery) return entry.kind === "Guide" ? 25 : entry.kind === "Rule" ? 20 : 0;

  let score = 0;
  let matchedOriginal = 0;
  for (const [field, weight] of fields) {
    const normalizedField = normalize(field);
    const fieldWords = words(field);
    if (normalizedField === normalizedQuery) score += weight * 4;
    else if (normalizedField.startsWith(normalizedQuery)) score += weight * 2;
    else if (normalizedField.includes(normalizedQuery)) score += weight;

    for (const token of original) {
      if (fieldWords.some((candidate) => candidate === token || candidate.startsWith(token))) {
        score += weight;
        matchedOriginal += 1;
      } else if (
        token.length >= 4 &&
        fieldWords.some((candidate) => candidate.length >= 4 && editDistanceWithin(token, candidate))
      ) {
        score += Math.round(weight * 0.45);
        matchedOriginal += 1;
      }
    }
    for (const token of expanded) {
      if (original.includes(token)) continue;
      if (fieldWords.some((candidate) => candidate === token || candidate.startsWith(token)))
        score += Math.round(weight * 0.12);
    }
  }
  if (matchedOriginal === 0 && score === 0) return null;
  const coverage = original.length ? Math.min(1, matchedOriginal / original.length) : 1;
  return score + Math.round(coverage * 80) + (entry.kind === "Function" ? 0 : 12);
}

export function searchHelpEntries(entries: HelpEntry[], query: string): HelpEntry[] {
  return entries
    .map((entry) => ({ entry, score: scoreEntry(entry, query) }))
    .filter((candidate): candidate is { entry: HelpEntry; score: number } => candidate.score !== null)
    .sort(
      (left, right) =>
        right.score - left.score ||
        left.entry.title.localeCompare(right.entry.title)
    )
    .map(({ entry }) => entry);
}

