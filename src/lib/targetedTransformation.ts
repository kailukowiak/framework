import { parseNamedTransformation } from "../PipelineColumnNames";

/**
 * Read a committed calculation line, knowing which column it calculates.
 *
 * A line normally carries its own name: `` `Margin` = `Revenue` - `Cost` ``.
 * But the editor it is typed into already says whose formula this is -- the
 * bar reads "Edit Margin" -- so a line with the name and its `=` deleted,
 * `Revenue - Cost`, is not a sentence missing its subject; the subject is
 * on screen. Refusing it was how `=10*2` typed at a cell ended in "Write a
 * column name, =, and a formula": the grid's `=` had opened the column's
 * line, the typing had replaced all of it, and the commit then asked the
 * person to retype a name the surface was displaying. So a line without an
 * assignment is that column's expression, and a spreadsheet's leading `=`
 * is forgiven the way `parseNamedTransformation` forgives a doubled one.
 *
 * What is still refused is a line that *does* assign but leaves a side
 * empty (`` `Margin` = ``): that is a half-written sentence, not a bare
 * expression, and guessing at it would save a formula nobody wrote.
 */
export function transformationForTarget(
  draft: string,
  targetName: string
): { name: string; formula: string } | null {
  const named = parseNamedTransformation(draft);
  if (named) return named;
  const formula = draft.trim().replace(/^=\s*/, "").trim();
  if (!formula || hasBareAssignment(formula)) return null;
  return { name: targetName, formula };
}

/** An `=` that is neither quoted nor half of `==`, `!=`, `<=` or `>=`. */
function hasBareAssignment(source: string): boolean {
  let quote: string | null = null;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === "\\" && quote !== "`") index += 1;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "`" || character === '"' || character === "'") {
      quote = character;
      continue;
    }
    if (character !== "=") continue;
    const before = source[index - 1] ?? "";
    if (source[index + 1] === "=") {
      index += 1;
      continue;
    }
    if (!before || !"=!<>".includes(before)) return true;
  }
  return false;
}
