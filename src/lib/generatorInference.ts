/**
 * Reading a selection the way Excel's fill handle reads it: the first two
 * values declare the step, and the whole run declares the bounds. The
 * inferred rule reproduces exactly the selected values — extending the run
 * is then one edit to the rule's stop, on the card, rather than a dialog.
 *
 * `sequence` excludes its stop (like Python's range), so the rule's stop is
 * one step past the last selected value.
 */
export function inferGeneratorRule(raws: string[]): string | null {
  const values = raws.map((raw) => raw.trim()).filter((raw) => raw.length > 0);
  if (values.length === 0) return null;

  const numbers = values.map(parseLooseNumber);
  if (numbers.every((value) => value !== null)) {
    const parsed = numbers as number[];
    const step = parsed.length > 1 ? round6(parsed[1] - parsed[0]) : 1;
    if (step === 0 || !arithmetic(parsed, step)) return null;
    const stop = round6(parsed[parsed.length - 1] + step);
    return step === 1
      ? `sequence(${parsed[0]}, ${stop})`
      : `sequence(${parsed[0]}, ${stop}, ${step})`;
  }

  const dates = values.map(parseIsoDate);
  if (dates.every((value) => value !== null)) {
    const parsed = dates as number[];
    const dayStep = parsed.length > 1 ? (parsed[1] - parsed[0]) / DAY : 1;
    if (dayStep === 0 || !Number.isInteger(dayStep) || !arithmetic(parsed, dayStep * DAY))
      return null;
    const stop = formatIsoDate(parsed[parsed.length - 1] + dayStep * DAY);
    const start = values[0];
    return dayStep === 1
      ? `sequence(${start}, ${stop})`
      : `sequence(${start}, ${stop}, ${dayStep}d)`;
  }

  return null;
}

/**
 * The numeric part of generator inference, kept as values rather than a
 * finished formula so the in-frame gesture can bind its stop to
 * `frame.len()`. A selected run says how the series starts and changes;
 * the frame says how far it goes. That is the spreadsheet fill-handle
 * contract, and avoids turning three example cells into a second card.
 */
export function inferNumericGeneratorPattern(
  raws: string[]
): NumericGeneratorPattern | null {
  const values = raws.map((raw) => raw.trim()).filter((raw) => raw.length > 0);
  if (values.length === 0) return null;
  const numbers = values.map(parseLooseNumber);
  if (!numbers.every((value) => value !== null)) return null;
  const parsed = numbers as number[];
  const step = parsed.length > 1 ? round6(parsed[1] - parsed[0]) : 1;
  if (step === 0 || !arithmetic(parsed, step)) return null;
  return { kind: "number", start: parsed[0], step };
}

export type NumericGeneratorPattern = {
  kind: "number";
  start: number;
  step: number;
};

export type DateGeneratorPattern = {
  kind: "date";
  start: string;
  step: number;
  unit: "d" | "mo";
};

export type GeneratorPattern = NumericGeneratorPattern | DateGeneratorPattern;

/** Infer the date patterns Excel users reach for most: days and months. */
export function inferDateGeneratorPattern(raws: string[]): DateGeneratorPattern | null {
  const values = raws.map((raw) => raw.trim()).filter((raw) => raw.length > 0);
  if (values.length === 0) return null;
  const dates = values.map(parseIsoDate);
  if (!dates.every((value) => value !== null)) return null;
  const parsed = dates as number[];

  if (parsed.length > 1) {
    const monthStep = monthDistance(parsed[0], parsed[1]);
    if (
      monthStep !== 0 &&
      parsed.every((value, index) => value === shiftMonths(parsed[0], monthStep * index))
    ) {
      return { kind: "date", start: values[0], step: monthStep, unit: "mo" };
    }
  }

  const dayStep = parsed.length > 1 ? (parsed[1] - parsed[0]) / DAY : 1;
  if (dayStep === 0 || !Number.isInteger(dayStep) || !arithmetic(parsed, dayStep * DAY))
    return null;
  return { kind: "date", start: values[0], step: dayStep, unit: "d" };
}

export function inferGeneratorPattern(raws: string[]): GeneratorPattern | null {
  return inferNumericGeneratorPattern(raws) ?? inferDateGeneratorPattern(raws);
}

const DAY = 86_400_000;

function parseLooseNumber(raw: string): number | null {
  const cleaned = raw.replace(/,/g, "");
  if (!/^-?\d+(\.\d+)?$/.test(cleaned)) return null;
  const value = Number(cleaned);
  return Number.isFinite(value) ? value : null;
}

function parseIsoDate(raw: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) return null;
  const value = Date.parse(`${raw}T00:00:00Z`);
  return Number.isNaN(value) ? null : value;
}

function formatIsoDate(timestamp: number): string {
  return new Date(timestamp).toISOString().slice(0, 10);
}

function monthDistance(left: number, right: number): number {
  const start = new Date(left);
  const end = new Date(right);
  return (end.getUTCFullYear() - start.getUTCFullYear()) * 12 +
    end.getUTCMonth() - start.getUTCMonth();
}

/** Calendar-month movement anchored to the first date, including month ends. */
function shiftMonths(timestamp: number, months: number): number {
  const start = new Date(timestamp);
  const targetMonth = start.getUTCMonth() + months;
  const year = start.getUTCFullYear() + Math.floor(targetMonth / 12);
  const month = ((targetMonth % 12) + 12) % 12;
  const lastDay = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  return Date.UTC(year, month, Math.min(start.getUTCDate(), lastDay));
}

/** Whether every gap matches the declared step, within float noise. */
function arithmetic(values: number[], step: number): boolean {
  return values.every(
    (value, index) =>
      index === 0 || Math.abs(value - values[index - 1] - step) < 1e-6 * Math.max(1, Math.abs(step))
  );
}

function round6(value: number): number {
  return Math.round(value * 1e6) / 1e6;
}
