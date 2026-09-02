import type { SchemaDiff } from "./bindings/SchemaDiff";

/**
 * What a refresh did to a frame's schema, as lines of ordinary text.
 *
 * Text rather than a panel of chips or a table: this is four facts at most,
 * it is read once and then it is gone, and a refresh that changed nothing
 * should cost nothing on screen. An empty diff produces no lines, which is
 * the answer for almost every refresh anyone ever runs.
 *
 * A column kept without data carries the reason it was kept, because "Cost
 * (no data)" on its own reads as a bug while "still read by the Margin
 * formula" reads as the thing to go and fix.
 */
export function schemaDiffLines(diff: SchemaDiff | null | undefined): string[] {
  if (!diff) return [];
  const lines: string[] = [];
  if (diff.added.length) lines.push(`Added: ${diff.added.join(", ")}`);
  if (diff.removed.length) lines.push(`Removed: ${diff.removed.join(", ")}`);
  for (const [name, reason] of diff.keptMissing)
    lines.push(`Kept, ${reason}: ${name} (no data)`);
  for (const [name, before, after] of diff.typeChanged)
    lines.push(`Type changed: ${name} ${before} → ${after}`);
  return lines;
}

export function schemaDiffIsEmpty(diff: SchemaDiff | null | undefined): boolean {
  return schemaDiffLines(diff).length === 0;
}
