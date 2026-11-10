/**
 * Whether a formula typed at a cell is plain arithmetic -- something with an
 * answer that no row, column or value of the document can change.
 *
 * `=10*2` typed into a hand-entered cell is somebody using the cell as a
 * calculator, the way every spreadsheet hand does: the answer is the value
 * they meant to type. Making it a column formula would overwrite every row
 * of a literal column with 20, which is a far bigger act than the gesture,
 * so literal data stays literal and only this one cell gets the answer.
 * Anything that *reads* something -- a column, a value, a function over the
 * column -- has an answer that moves, and keeps going to the column's
 * formula, where the whole column is visibly the subject.
 *
 * The frontend has no formula parser (the engine owns the language), so this
 * is a deliberately narrow scan rather than a parse: it accepts only the
 * characters arithmetic is made of -- digits, the decimal point, `$`, the
 * operators, parentheses -- plus a short list of element-wise numeric
 * methods called on a number or a parenthesis (`(10/3).round(2)`). No
 * letter outside that list, no quote, no backtick, no bracket can appear,
 * so no name of any kind can. Being narrow is the safe direction: a formula
 * this refuses is not wrong, it just goes to the column doorway as before.
 * What the accepted text *means* -- `4.25%`, `2024-10-10`, `10 // 3` -- is
 * still the engine's call, made when it evaluates the answer.
 *
 * Returns the expression without the spreadsheet's leading `=`, or null.
 */
export function referenceFreeArithmetic(source: string): string | null {
  const expression = source.trim().replace(/^=\s*/, "").trim();
  if (!expression) return null;
  let scan: Scan | null = { index: 0, depth: 0, sawNumber: false, previous: "operator" };
  while (scan && scan.index < expression.length) scan = advance(expression, scan);
  return scan && scan.depth === 0 && scan.sawNumber ? expression : null;
}

/**
 * Where the scan stands. `previous` is all a method call needs to know:
 * `.round(` must hang off a value, never begin one -- a leading `.round()`
 * would be a method on some implicit column.
 */
type Scan = {
  index: number;
  depth: number;
  sawNumber: boolean;
  previous: "value" | "operator";
};

/** Read one token past `scan.index`, or null when it is not arithmetic. */
function advance(expression: string, scan: Scan): Scan | null {
  const character = expression[scan.index];
  const next = expression[scan.index + 1] ?? "";
  const after = scan.index + 1;
  if (/\s/.test(character)) return { ...scan, index: after };
  if (isDigit(character) || (character === "." && isDigit(next))) {
    let index = after;
    while (index < expression.length && /[0-9.]/.test(expression[index])) index += 1;
    return { ...scan, index, sawNumber: true, previous: "value" };
  }
  if (character === "." && /[A-Za-z_]/.test(next)) return method(expression, scan);
  if (character === "(")
    return { ...scan, index: after, depth: scan.depth + 1, previous: "operator" };
  if (character === ")")
    return scan.depth === 0
      ? null
      : { ...scan, index: after, depth: scan.depth - 1, previous: "value" };
  if (character === "," && scan.depth > 0)
    return { ...scan, index: after, previous: "operator" };
  // `%` stays allowed in both of its meanings -- `4.25%` and `10 % 3` are
  // each arithmetic -- and the engine decides which one was meant.
  if (character === "%") return { ...scan, index: after };
  if ("+-*/$".includes(character)) return { ...scan, index: after, previous: "operator" };
  return null;
}

/** `.name(` after a value, where the name is an element-wise method. */
function method(expression: string, scan: Scan): Scan | null {
  const name = /^[A-Za-z_][A-Za-z0-9_]*/.exec(expression.slice(scan.index + 1))![0];
  const index = scan.index + 1 + name.length;
  const called = expression.slice(index).trimStart().startsWith("(");
  if (scan.previous !== "value" || !ELEMENTWISE_METHODS.has(name) || !called) return null;
  return { ...scan, index, previous: "operator" };
}

/**
 * Numeric methods whose answer for a number depends on nothing but that
 * number and the literal arguments beside it. Aggregates (`sum`, `mean`)
 * and row-relative methods (`shift`, `cum_sum`) are absent on purpose: on
 * a column they read other rows, which is exactly what this cell gesture
 * must not quietly stand in for.
 */
const ELEMENTWISE_METHODS = new Set([
  "abs",
  "round",
  "floor",
  "ceil",
  "sqrt",
  "cbrt",
  "pow",
  "exp",
  "log",
  "log1p",
  "clip",
]);

function isDigit(character: string): boolean {
  return character >= "0" && character <= "9";
}
