/**
 * Reads clipboard text into rows of cells.
 *
 * Text pasted into cells of an existing frame is a spreadsheet's clipboard (or
 * this app's own `serializeGrid`), and both are always tab-separated, so there
 * a comma is data: `1,234` and `Smith, John` are one cell, and text with no
 * tab is one column, one cell per line. Only the caller reading text that
 * might be CSV (`csv: true`, used to size a frame the core is about to read
 * the same text into) lets a comma delimit, and only then are cells trimmed,
 * because `a, b` is padding there while a tab-separated cell's spaces were
 * typed by someone and are theirs.
 */
export function parseGrid(source: string, options: { csv?: boolean } = {}): string[][] {
  const input = source.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  if (!input.length) return [];
  const csv = options.csv === true;
  const delimiter = csv && !input.includes("\t") ? "," : "\t";
  const rows: string[][] = [];
  let row: string[] = [];
  let value = "";
  let quoted = false;

  const pushValue = () => {
    row.push(csv ? value.trim() : value);
    value = "";
  };
  const pushRow = () => {
    pushValue();
    rows.push(row);
    row = [];
  };

  for (let index = 0; index < input.length; index += 1) {
    const character = input[index];
    if (character === '"') {
      if (quoted && input[index + 1] === '"') {
        value += '"';
        index += 1;
      } else {
        quoted = !quoted;
      }
    } else if (character === delimiter && !quoted) {
      pushValue();
    } else if (character === "\n" && !quoted) {
      pushRow();
    } else {
      value += character;
    }
  }
  if (row.length || value.length || !input.endsWith("\n")) pushRow();
  return rows;
}
