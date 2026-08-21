export function parseGrid(source: string): string[][] {
  const input = source.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  if (!input.length) return [];
  const delimiter = input.includes("\t") ? "\t" : ",";
  const rows: string[][] = [];
  let row: string[] = [];
  let value = "";
  let quoted = false;

  const pushValue = () => {
    row.push(value.trim());
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
