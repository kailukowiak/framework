import { formulaToken } from "./formulaReferences";

/**
 * Whether a column's formula only re-states the column itself.
 *
 * A branched tab is a pass-through child: every column arrives through an
 * identity projection, so the engine reports a formula for all of them and
 * the grid drew a ƒ badge over an entire table nobody had calculated
 * anything in. The badge means "this column is worked out from something
 * else" — `` `Revenue` `` against a column named Revenue says nothing of
 * the kind, and a badge that is always on tells you nothing when it is.
 *
 * The test is textual on purpose: the frontend does not parse formulas, and
 * the engine echoes them canonically, so the two spellings a pass-through
 * can arrive in — backticked and bare — are the whole set.
 */
export function isIdentityColumnFormula(
  formula: string | null | undefined,
  columnName: string
): boolean {
  if (!formula) return false;
  const trimmed = formula.trim();
  return trimmed === formulaToken(columnName) || trimmed === columnName;
}
