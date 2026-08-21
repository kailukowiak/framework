/**
 * The written column list an unpivot melts: `` `Jan`, `Feb`,
 * starts_with("Q") ``. The core owns this notation — it parses the text
 * against the schema when the chain is saved, and its answer is the one
 * that counts. What lives here is the local approximation the editor
 * needs between keystrokes: which of the columns it can currently see the
 * list would melt, so the walk down the chain costs no round trip.
 *
 * Approximate on purpose, in one direction only: a piece this reader
 * cannot make sense of melts nothing, rather than guessing. The preview
 * that comes back from the core replaces this answer whenever the chain
 * parses, so the cost of the gap is a moment of the step below seeing a
 * column that is about to be gone.
 */

interface VisibleColumnName {
  id: string;
  name: string;
}

/** Which of `visible` the written list melts, in matched order. */
export function meltedColumnIds(
  text: string,
  visible: VisibleColumnName[]
): string[] {
  const collected: string[] = [];
  const add = (id: string) => {
    if (!collected.includes(id)) collected.push(id);
  };
  for (const piece of splitPieces(text)) {
    const name = backtickedName(piece);
    if (name !== null) {
      const column = visible.find((candidate) => candidate.name === name);
      if (column) add(column.id);
      continue;
    }
    const pattern = matchPatternSelector(piece);
    if (pattern) {
      for (const column of visible) {
        if (pattern.matches(column.name)) add(column.id);
      }
      continue;
    }
    const excepted = matchExcept(piece);
    if (excepted) {
      for (const column of visible) {
        if (!excepted.includes(column.name)) add(column.id);
      }
    }
    // Anything else is a piece the core will refuse or one still being
    // typed; either way it melts nothing yet.
  }
  return collected;
}

/**
 * The list split at top-level commas: commas inside backticks, quoted
 * strings, and parentheses belong to their piece. Mirrors how the core's
 * lexer reads the same text.
 */
function splitPieces(text: string): string[] {
  const pieces: string[] = [];
  let current = "";
  let depth = 0;
  let quote: string | null = null;
  let inBacktick = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (quote) {
      current += character;
      if (character === "\\") {
        current += text[index + 1] ?? "";
        index += 1;
      } else if (character === quote) {
        quote = null;
      }
      continue;
    }
    if (inBacktick) {
      current += character;
      if (character === "`") {
        // A doubled backtick is a literal one inside the name.
        if (text[index + 1] === "`") {
          current += "`";
          index += 1;
        } else {
          inBacktick = false;
        }
      }
      continue;
    }
    if (character === "`") inBacktick = true;
    else if (character === "'" || character === '"') quote = character;
    else if (character === "(") depth += 1;
    else if (character === ")") depth = Math.max(0, depth - 1);
    else if (character === "," && depth === 0) {
      pieces.push(current);
      current = "";
      continue;
    }
    current += character;
  }
  pieces.push(current);
  return pieces.map((piece) => piece.trim()).filter(Boolean);
}

/**
 * The name inside a piece that is exactly one backticked reference, with
 * doubled backticks unescaped — or null when the piece is anything else,
 * including a reference with text after it.
 */
function backtickedName(piece: string): string | null {
  if (!piece.startsWith("`")) return null;
  let name = "";
  let index = 1;
  while (index < piece.length) {
    const character = piece[index];
    if (character === "`") {
      if (piece[index + 1] === "`") {
        name += "`";
        index += 2;
        continue;
      }
      return piece.slice(index + 1).trim() === "" ? name : null;
    }
    name += character;
    index += 1;
  }
  return null;
}

const PATTERN_SELECTOR =
  /^(starts_with|ends_with|contains)\s*\(\s*(['"])((?:\\.|(?!\2).)*)\2\s*\)$/i;

function matchPatternSelector(
  piece: string
): { matches: (name: string) => boolean } | null {
  const match = PATTERN_SELECTOR.exec(piece);
  if (!match) return null;
  const pattern = unescapeString(match[3]);
  // The same refusal as the core, by omission: an empty pattern would
  // match everything, so it matches nothing here either.
  if (!pattern) return null;
  switch (match[1].toLowerCase()) {
    case "starts_with":
      return { matches: (name) => name.startsWith(pattern) };
    case "ends_with":
      return { matches: (name) => name.endsWith(pattern) };
    default:
      return { matches: (name) => name.includes(pattern) };
  }
}

/** The names inside `except(…)`, or null when the piece is not one. */
function matchExcept(piece: string): string[] | null {
  const match = /^except\s*\((.*)\)$/is.exec(piece);
  if (!match) return null;
  const names: string[] = [];
  for (const inner of splitPieces(match[1])) {
    const name = backtickedName(inner);
    if (name === null) return null;
    names.push(name);
  }
  return names.length > 0 ? names : null;
}

function unescapeString(value: string): string {
  return value.replace(/\\(.)/g, (_, escaped: string) => {
    if (escaped === "n") return "\n";
    if (escaped === "r") return "\r";
    if (escaped === "t") return "\t";
    return escaped;
  });
}
