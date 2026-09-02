import type { ColumnFormat, DataType, DatePattern, ScalarValue } from "./types";

// Display-only formatting: stored values are never rounded or mutated here.
// The value / unit / presentation layers stay separate — this module only
// produces presentation text from a raw value and a column's format.

const CURRENCY_SYMBOLS: Record<string, string> = {
  USD: "$",
  CAD: "$",
  AUD: "$",
  EUR: "€",
  GBP: "£",
  JPY: "¥",
  CHF: "CHF",
};

const MAX_DISPLAY_DECIMALS = 20;
const EN_DASH = "–";

// Fixed English month table: deterministic, no `Intl` (a document's
// display should not depend on the machine that opens it).
const MONTH_NAMES = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

const ISO_DATE_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

/**
 * Render a strict `YYYY-MM-DD` string per the column's date pattern.
 * Anything that doesn't match that exact shape — including an already
 * malformed month or day — passes through unchanged.
 */
function formatDateValue(
  raw: string | number | null | undefined,
  pattern: DatePattern | null | undefined
): string {
  const text = typeof raw === "string" ? raw : raw == null ? "" : String(raw);
  const match = ISO_DATE_PATTERN.exec(text);
  if (!match) return text;
  const year = Number(match[1]);
  const monthIndex = Number(match[2]) - 1;
  const day = Number(match[3]);
  if (monthIndex < 0 || monthIndex > 11 || day < 1 || day > 31) return text;
  const month = MONTH_NAMES[monthIndex];
  switch (pattern) {
    case "dayMonthYear":
      return `${day} ${month} ${year}`;
    case "monthDayYear":
      return `${month} ${day}, ${year}`;
    case "monthYear":
      return `${month} ${year}`;
    case "quarter":
      return `Q${Math.floor(monthIndex / 3) + 1} ${year}`;
    case "iso":
    default:
      return text;
  }
}

export interface FormattedCellParts {
  /** Left-pinned currency symbol for accounting cells; empty otherwise. */
  symbol: string;
  /** Right-aligned display text (includes an inline symbol for currency style). */
  value: string;
}

export function currencySymbol(code: string | null | undefined): string {
  const normalized = (code ?? "").trim().toUpperCase();
  if (!normalized) return "$";
  return CURRENCY_SYMBOLS[normalized] ?? normalized;
}

function scaleFactor(format: ColumnFormat): number {
  return format.scale === "thousands" ? 1e3 : format.scale === "millions" ? 1e6 : 1;
}

function negativeParens(format: ColumnFormat): boolean {
  return format.negativeParens ?? format.style === "accounting";
}

function zeroDash(format: ColumnFormat): boolean {
  return format.zeroDash ?? format.style === "accounting";
}

function parseNumericValue(raw: string | number | null | undefined): number | null {
  if (typeof raw === "number") return Number.isFinite(raw) ? raw : null;
  if (raw == null) return null;
  const text = raw.trim();
  if (!text) return null;
  const isPercent = text.endsWith("%");
  const cleaned = (isPercent ? text.slice(0, -1) : text)
    .replace(/[$€£¥,\s]/g, "")
    .replace(/^\((.*)\)$/, "-$1");
  if (!cleaned || !/^[-+]?(\d+\.?\d*|\.\d+)$/.test(cleaned)) return null;
  const value = Number(cleaned);
  if (!Number.isFinite(value)) return null;
  return isPercent ? value / 100 : value;
}

function groupDigits(text: string, useGrouping: boolean): string {
  if (!useGrouping) return text;
  const [integer, fraction] = text.split(".");
  const grouped = integer.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return fraction ? `${grouped}.${fraction}` : grouped;
}

function magnitudeText(magnitude: number, decimals: number | null | undefined): string {
  if (decimals != null) {
    return magnitude.toFixed(
      Math.min(Math.max(Math.trunc(decimals), 0), MAX_DISPLAY_DECIMALS)
    );
  }
  // No display rounding requested: trim float noise without inventing precision.
  const fixed = magnitude.toFixed(6);
  return fixed.replace(/0+$/, "").replace(/\.$/, "");
}

function defaultDecimals(
  format: ColumnFormat,
  dataType?: DataType
): number | null | undefined {
  if (format.decimals != null) return format.decimals;
  if (format.style === "currency" || format.style === "accounting") return 2;
  if (dataType === "integer") return 0;
  if (dataType === "number") return 2;
  return undefined;
}

export interface NumberDisplayOptions {
  dataType?: DataType;
  useGrouping?: boolean;
}

/**
 * Format one raw cell value for display. Returns structured parts so the
 * renderer can pin an accounting symbol left while the value stays right.
 * Non-numeric text passes through unchanged.
 */
export function formatCellValue(
  raw: string | number | null | undefined,
  format: ColumnFormat,
  options: NumberDisplayOptions = {}
): FormattedCellParts {
  if (options.dataType === "date") {
    return { symbol: "", value: formatDateValue(raw, format.datePattern) };
  }
  if (format.style === "plain") {
    if (typeof raw === "number")
      return { symbol: "", value: Number.isFinite(raw) ? String(raw) : "" };
    return { symbol: "", value: raw ?? "" };
  }
  const numeric = parseNumericValue(raw);
  if (numeric === null) {
    return { symbol: "", value: typeof raw === "string" ? raw : "" };
  }

  const accounting = format.style === "accounting";
  const symbol = currencySymbol(format.currencyCode);
  const inlineSymbol =
    format.style === "currency" ? (symbol.length > 1 ? `${symbol} ` : symbol) : "";
  if (numeric === 0 && zeroDash(format)) {
    return { symbol: accounting ? symbol : "", value: EN_DASH };
  }

  let display = numeric / scaleFactor(format);
  if (format.style === "percent") display *= 100;
  const negative = display < 0;
  const digits = groupDigits(
    magnitudeText(Math.abs(display), defaultDecimals(format, options.dataType)),
    options.useGrouping ?? true
  );
  const suffix = format.style === "percent" ? "%" : "";
  const unsigned = `${inlineSymbol}${digits}${suffix}`;
  const value = !negative
    ? unsigned
    : negativeParens(format)
    ? `(${unsigned})`
    : `-${unsigned}`;
  return { symbol: accounting ? symbol : "", value };
}

/** Flat single-string variant for places that cannot render two-part cells. */
export function formatCellText(
  raw: string | number | null | undefined,
  format: ColumnFormat,
  options?: NumberDisplayOptions
): string {
  const parts = formatCellValue(raw, format, options);
  if (!parts.symbol) return parts.value;
  return parts.value ? `${parts.symbol} ${parts.value}` : parts.value;
}

/**
 * A computed scalar outside the grid should read the way the same value does
 * inside it. The core owns the value and its semantic type; grouping is a
 * machine-local reading preference, so that final presentation step belongs
 * here with frame cells rather than being baked into serialized engine text.
 */
export function formatComputedScalar(
  typedValue: ScalarValue,
  dataType: DataType,
  fallback: string,
  useGrouping = true
): string {
  if (typedValue.type !== "number") return fallback;
  const style =
    dataType === "currency"
      ? "currency"
      : dataType === "percentage"
        ? "percent"
        : dataType === "integer" || dataType === "number"
          ? "number"
          : null;
  if (!style) return fallback;
  return formatCellText(
    typedValue.value,
    { style, decimals: dataType === "integer" ? 0 : null },
    { dataType, useGrouping }
  );
}

/**
 * Compact unit badge for column headers, e.g. "$K", "€M", "%". Returns null
 * when the header needs no badge (unscaled plain, number, and currency
 * columns whose symbols already appear in each cell).
 */
export function columnFormatBadge(format: ColumnFormat): string | null {
  if (format.datePattern) return null;
  const scaleSuffix =
    format.scale === "thousands" ? "K" : format.scale === "millions" ? "M" : "";
  if (format.style === "percent") return `%${scaleSuffix}`;
  if (!scaleSuffix) return null;
  if (format.style === "currency" || format.style === "accounting") {
    return `${currencySymbol(format.currencyCode)}${scaleSuffix}`;
  }
  return scaleSuffix;
}
