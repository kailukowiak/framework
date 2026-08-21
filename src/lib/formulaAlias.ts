// Naming a calculated column after the formula that computes it.
//
// `` `debit`.sum() `` is called "Debit Sum" by anyone describing it out
// loud, and being made to type that before the formula will even run is
// friction for no gain. So the name follows the formula until somebody
// types in the name field, which the draft records as `nameTouched`.
//
// Only an unambiguous formula gets a name. One that reads two columns
// (`` `debit`.sum() - `credit`.sum() ``) has no obvious short name, and
// guessing one badly is worse than leaving the field for the user: a wrong
// name that looks deliberate survives into every formula written against
// it.

/** Long enough to stay descriptive, short enough to stay a column header. */
const MAX_ALIAS_LENGTH = 40;

/**
 * A column name for `formula`, or empty when it does not suggest one.
 *
 * Empty is the common answer and the safe one: the caller leaves whatever
 * name is already there.
 */
export function aliasFromFormula(formula: string): string {
  const columns = new Set<string>();
  for (const match of formula.matchAll(/`((?:[^`]|``)*)`/g)) {
    columns.add(match[1].replaceAll("``", "`").trim());
  }
  // Two columns, or none, and the formula is no longer describing one
  // thing that a short name could stand for.
  if (columns.size !== 1) return "";
  const [column] = columns;
  if (!column) return "";

  // Method calls in order, so `.sum()` becomes " Sum" and a chain reads
  // the way it was written.
  const methods = [...formula.matchAll(/\.([\p{L}_][\p{L}\p{N}_]*)\s*\(/gu)].map(
    (match) => match[1]
  );
  const alias = [column, ...methods].map(capitalize).join(" ");
  return alias.length > MAX_ALIAS_LENGTH
    ? `${alias.slice(0, MAX_ALIAS_LENGTH - 1).trimEnd()}…`
    : alias;
}

function capitalize(value: string): string {
  return value.charAt(0).toLocaleUpperCase() + value.slice(1);
}
