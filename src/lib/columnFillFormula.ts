/**
 * A list-shaped formula entered at one cell is a column declaration, not a
 * literal string and not a one-cell override. Keep this deliberately narrow:
 * only a sequence explicitly tied to the frame's row count has an honest
 * one-result-per-row meaning.
 */
export function columnFillFormula(raw: string): string | null {
  const source = raw.trim();
  if (!source.startsWith("=")) return null;
  const formula = source.slice(1).trim();
  if (
    !/^sequence\s*\(/i.test(formula) ||
    !/frame\s*\.\s*(?:len|n_rows)\s*\(\s*\)/i.test(formula)
  )
    return null;
  return formula;
}
