/**
 * Where an offset in `before` has ended up in `after`.
 *
 * An offset is not a position. The document may hand a block back with text
 * nobody typed — naming a line rewrites the lines that read it — and text
 * arriving back two characters longer leaves a caret counted in characters
 * two characters behind where the author left it. A word to the left, or a
 * line up, every time somebody finishes typing a name.
 *
 * So the caret is carried across the change instead of counted again: what
 * the two texts still share at the front and at the back is what did not
 * move, and an offset inside either of those is an offset that can be kept.
 * Only the middle is guessed at, and the middle is by construction text the
 * author was not in — their own edit is what was sent.
 */
export function carryCaret(before: string, after: string, at: number): number {
  const shortest = Math.min(before.length, after.length);
  let prefix = 0;
  while (prefix < shortest && before[prefix] === after[prefix]) prefix += 1;
  let suffix = 0;
  while (
    suffix < shortest - prefix &&
    before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
  )
    suffix += 1;
  if (at <= prefix) return at;
  if (at >= before.length - suffix) return after.length - (before.length - at);
  return Math.max(prefix, Math.min(at, after.length - suffix));
}
