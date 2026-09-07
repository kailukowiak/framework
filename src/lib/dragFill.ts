import { inferDateGeneratorPattern, inferNumericGeneratorPattern } from "./generatorInference";
import type { Column, Row } from "./types";

/** A drag writes literal data only into its visible destination. Two numeric
 * seeds declare an arithmetic series; other selections repeat their values.
 * Dates also support calendar months, anchored to the first seed so February
 * does not permanently turn a month-end series into the 28th of every month.
 */
export function dragFillCells(column: Column, rows: Row[], first: number, last: number, destination: number) {
  const seeds = rows.slice(first, last + 1).map((row) => row.cells[column.id]?.raw ?? "");
  const numeric = seeds.length > 1 && ["integer", "number"].includes(column.dataType)
    ? inferNumericGeneratorPattern(seeds) : null;
  const date = column.dataType === "date" ? inferDateGeneratorPattern(seeds) : null;
  const hasBlanks = seeds.some((raw) => !raw.trim());
  const valueAt = (index: number) => {
    if (!hasBlanks && numeric) return String(Number((numeric.start + numeric.step * index).toPrecision(15)));
    if (!hasBlanks && date) {
      const start = new Date(`${date.start}T00:00:00Z`);
      if (date.unit === "d") start.setUTCDate(start.getUTCDate() + date.step * index);
      else {
        const day = start.getUTCDate();
        start.setUTCDate(1);
        start.setUTCMonth(start.getUTCMonth() + date.step * index);
        const end = new Date(Date.UTC(start.getUTCFullYear(), start.getUTCMonth() + 1, 0)).getUTCDate();
        start.setUTCDate(Math.min(day, end));
      }
      return start.toISOString().slice(0, 10);
    }
    return seeds[((index % seeds.length) + seeds.length) % seeds.length];
  };
  const from = destination < first ? destination : last + 1;
  const to = destination < first ? first - 1 : destination;
  return rows.slice(from, to + 1).map((row, index) => ({
    rowId: row.id, columnId: column.id, raw: valueAt(from + index - first),
  }));
}
