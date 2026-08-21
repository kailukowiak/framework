export type FormattedFormula = {
  source: string;
  selection: { start: number; end: number };
};

type QuotedCharacter = {
  text: string;
  skip: number;
  quote: '"' | "'" | "`" | null;
  escaped: boolean;
};

function readQuotedCharacter(
  source: string,
  index: number,
  quote: '"' | "'" | "`",
  escaped: boolean
): QuotedCharacter {
  const character = source[index];
  if (quote === "`" && character === "`" && source[index + 1] === "`") {
    return { text: "``", skip: 1, quote, escaped: false };
  }
  if (quote !== "`" && character === "\\" && !escaped) {
    return { text: character, skip: 0, quote, escaped: true };
  }
  return {
    text: character,
    skip: 0,
    quote: character === quote && !escaped ? null : quote,
    escaped: false,
  };
}

/**
 * Put each member in a formula chain on its own indented line. This is a
 * whitespace-only formatter: strings and backticked names pass through byte
 * for byte, decimal points stay numbers, and an already-leading dot is left
 * alone so pressing Format twice is harmless.
 */
export function formatFormulaChains(
  source: string,
  selectionStart = source.length,
  selectionEnd = selectionStart
): FormattedFormula {
  let output = "";
  let depth = 0;
  let quote: '"' | "'" | "`" | null = null;
  let escaped = false;
  let start = selectionStart;
  let end = selectionEnd;

  const insert = (value: string, at: number) => {
    output += value;
    if (at <= selectionStart) start += value.length;
    if (at <= selectionEnd) end += value.length;
  };

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      const quoted = readQuotedCharacter(source, index, quote, escaped);
      output += quoted.text;
      index += quoted.skip;
      quote = quoted.quote;
      escaped = quoted.escaped;
      continue;
    }
    if (character === '"' || character === "'" || character === "`") {
      quote = character;
      output += character;
      continue;
    }
    if (character === "(" || character === "[") depth += 1;
    if (character === ")" || character === "]") depth = Math.max(0, depth - 1);
    if (character === "." && /[\p{L}_]/u.test(source[index + 1] ?? "")) {
      const line = output.slice(output.lastIndexOf("\n") + 1);
      if (line.trim()) insert(`\n${"  ".repeat(depth + 1)}`, index);
    }
    output += character;
  }
  return { source: output, selection: { start, end } };
}
