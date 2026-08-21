import {
  continueFormula,
  logicalGroups,
  physicalToLogical,
  scratchworkExpressionStart,
} from "./blockLines";

export type AppendedScratchworkLine = {
  source: string;
  lineIndex: number;
};

/**
 * The block ⌘J goes to.
 *
 * A name rather than a flag on the object: a document that has never been
 * opened in this build still has one as soon as somebody names a block this,
 * and renaming it is how you say you would rather ⌘J went somewhere else.
 * The core knows nothing about it, which is the point — it is an ordinary
 * block, and only the key treats it specially.
 */
export const SCRATCHWORK = "Scratchwork";

/**
 * Whether two canvas names are the same name.
 *
 * Matches the core's own rule for resolving a reference: case and anything
 * that is not a letter or a digit are ignored, so `Scratch work` and
 * `scratchwork` are one name. Anything looser here than there would let ⌘J
 * find a block that a formula could not.
 */
export function sameName(left: string, right: string): boolean {
  const bare = (name: string) => name.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "");
  return bare(left) === bare(right);
}

export type ScratchworkLine = {
  start: number;
  end: number;
  source: string;
};

/**
 * The one block line the formula bar is allowed to edit — the *logical*
 * line: a formula continued across indented rows is still one calculation,
 * and the bar mirrors all of it, embedded newlines included.
 */
export function scratchworkLineAt(source: string, cursor: number): ScratchworkLine {
  const at = Math.max(0, Math.min(cursor, source.length));
  const row = source.slice(0, at).split("\n").length - 1;
  const map = physicalToLogical(source);
  const logical = map[row] ?? 0;
  const rows = source.split("\n");
  let start = 0;
  let index = 0;
  while (index < rows.length && (map[index] ?? 0) < logical) {
    start += rows[index].length + 1;
    index += 1;
  }
  let end = start;
  while (index < rows.length && (map[index] ?? 0) === logical) {
    end += rows[index].length + 1;
    index += 1;
  }
  end -= 1; // The trailing newline belongs to the break, not the line.
  return { start, end, source: source.slice(start, end) };
}

export function replaceScratchworkLine(
  source: string,
  cursor: number,
  replacement: string,
  selection: { start: number; end: number }
) {
  const line = scratchworkLineAt(source, cursor);
  return {
    source: `${source.slice(0, line.start)}${replacement}${source.slice(line.end)}`,
    selection: {
      start: line.start + selection.start,
      end: line.start + selection.end,
    },
  };
}

/**
 * Accept a document rewrite without retyping the declaration under the cursor.
 *
 * The document owns stable-id rewrites in the expression body: a frame rename
 * must change the visible reference even while this line has focus. Only the
 * declaration before `=` is temporarily the author's, because the core holds
 * its old name until the cursor leaves rather than renaming on every prefix.
 * Keeping the whole line here used to throw away unrelated frame renames.
 */
export function mergeStoredScratchwork(
  stored: string,
  draft: string,
  editing: number | null
): string {
  if (editing === null) return stored;
  const theirs = logicalGroups(stored);
  const ours = logicalGroups(draft);
  if (theirs.length !== ours.length || editing >= ours.length) return stored;

  const oursExpression = scratchworkExpressionStart(ours[editing]);
  if (oursExpression === 0) return stored;
  const theirsExpression = scratchworkExpressionStart(theirs[editing]);
  theirs[editing] = `${ours[editing].slice(0, oursExpression)}${theirs[editing].slice(
    theirsExpression
  )}`;
  return theirs.join("\n");
}

/** Apply Alt+Return to the logical calculation under a block editor's cursor. */
export function continueScratchworkLine(
  source: string,
  selectionStart: number,
  selectionEnd: number
) {
  const line = scratchworkLineAt(source, selectionStart);
  const expanded = continueFormula(
    line.source,
    selectionStart - line.start,
    selectionEnd - line.start
  );
  return {
    source: `${source.slice(0, line.start)}${expanded.source}${source.slice(line.end)}`,
    selection: {
      start: line.start + expanded.selection.start,
      end: line.start + expanded.selection.end,
    },
  };
}

/**
 * Put one formula after the block's existing work without manufacturing a
 * blank line. The returned index addresses the exact computed line the bar
 * just added — a *logical* index, because that is what computed lines are —
 * including when the block deliberately contains blank lines.
 */
export function appendScratchworkLine(
  current: string,
  formula: string
): AppendedScratchworkLine {
  const prefix = current.length === 0 || current.endsWith("\n") ? current : `${current}\n`;
  const source = `${prefix}${formula}`;
  return {
    source,
    lineIndex: logicalGroups(source).length - 1,
  };
}
