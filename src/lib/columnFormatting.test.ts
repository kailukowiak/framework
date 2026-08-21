import { describe, expect, it } from "vitest";
import {
  columnFormatBadge,
  currencySymbol,
  formatComputedScalar,
  formatCellText,
  formatCellValue,
} from "./columnFormatting";
import type { ColumnFormat } from "./types";

const format = (overrides: Partial<ColumnFormat>): ColumnFormat => ({
  style: "number",
  ...overrides,
});

describe("currencySymbol", () => {
  it("maps known ISO 4217 codes to symbols", () => {
    expect(currencySymbol("USD")).toBe("$");
    expect(currencySymbol("CAD")).toBe("$");
    expect(currencySymbol("AUD")).toBe("$");
    expect(currencySymbol("EUR")).toBe("€");
    expect(currencySymbol("GBP")).toBe("£");
    expect(currencySymbol("JPY")).toBe("¥");
    expect(currencySymbol("CHF")).toBe("CHF");
  });

  it("normalizes case and whitespace and defaults to the dollar sign", () => {
    expect(currencySymbol(" eur ")).toBe("€");
    expect(currencySymbol(null)).toBe("$");
    expect(currencySymbol(undefined)).toBe("$");
    expect(currencySymbol("")).toBe("$");
  });

  it("renders unknown codes as the code itself", () => {
    expect(currencySymbol("SEK")).toBe("SEK");
    expect(currencySymbol("xdr")).toBe("XDR");
  });
});

describe("formatCellValue", () => {
  it("passes plain style through untouched", () => {
    expect(formatCellValue("1234.5", format({ style: "plain", decimals: 1 }))).toEqual({
      symbol: "",
      value: "1234.5",
    });
    expect(formatCellValue("hello", format({ style: "plain" }))).toEqual({
      symbol: "",
      value: "hello",
    });
  });

  it("passes non-numeric and empty values through unchanged", () => {
    expect(formatCellValue("pending", format({ style: "currency" }))).toEqual({
      symbol: "",
      value: "pending",
    });
    expect(formatCellValue("", format({ style: "accounting" }))).toEqual({
      symbol: "",
      value: "",
    });
    expect(formatCellValue(null, format({ style: "number" }))).toEqual({
      symbol: "",
      value: "",
    });
    expect(formatCellValue(undefined, format({ style: "percent" }))).toEqual({
      symbol: "",
      value: "",
    });
    expect(formatCellValue(Number.NaN, format({ style: "number" }))).toEqual({
      symbol: "",
      value: "",
    });
  });

  it("formats numbers with grouping and display-only rounding", () => {
    expect(formatCellValue(1234567.891, format({ decimals: 2 }))).toEqual({
      symbol: "",
      value: "1,234,567.89",
    });
    expect(formatCellValue("1200.5", format({}))).toEqual({
      symbol: "",
      value: "1,200.5",
    });
    expect(formatCellValue(4.5, format({ decimals: 0 }))).toEqual({
      symbol: "",
      value: "5",
    });
  });

  it("distinguishes default float and integer display", () => {
    expect(
      formatCellValue(1200, format({}), { dataType: "number" })
    ).toEqual({ symbol: "", value: "1,200.00" });
    expect(
      formatCellValue(1200.5, format({}), { dataType: "number" })
    ).toEqual({ symbol: "", value: "1,200.50" });
    expect(
      formatCellValue(1200, format({}), { dataType: "integer" })
    ).toEqual({ symbol: "", value: "1,200" });
  });

  it("can disable grouping globally without changing column precision", () => {
    expect(
      formatCellValue(1234567.5, format({}), {
        dataType: "number",
        useGrouping: false,
      })
    ).toEqual({ symbol: "", value: "1234567.50" });
  });

  it("parses raw text that carries symbols, separators, and percent signs", () => {
    expect(formatCellValue("$18.50", format({ decimals: 1 }))).toEqual({
      symbol: "",
      value: "18.5",
    });
    expect(formatCellValue("1,250,000", format({}))).toEqual({
      symbol: "",
      value: "1,250,000",
    });
    expect(formatCellValue("5%", format({ style: "percent" }))).toEqual({
      symbol: "",
      value: "5%",
    });
    expect(formatCellValue("(1,200)", format({ decimals: 0 }))).toEqual({
      symbol: "",
      value: "-1,200",
    });
  });

  it("prefixes currency symbols inline and defaults currency to two decimals", () => {
    expect(formatCellValue(1234.5, format({ style: "currency" }))).toEqual({
      symbol: "",
      value: "$1,234.50",
    });
    expect(
      formatCellValue(1234.5, format({ style: "currency", currencyCode: "EUR" }))
    ).toEqual({ symbol: "", value: "€1,234.50" });
    expect(
      formatCellValue(
        9.75,
        format({ style: "currency", currencyCode: "JPY", decimals: 0 })
      )
    ).toEqual({ symbol: "", value: "¥10" });
  });

  it("renders unknown and multi-character currency codes as a spaced prefix", () => {
    expect(
      formatCellValue(50, format({ style: "currency", currencyCode: "CHF" }))
    ).toEqual({ symbol: "", value: "CHF 50.00" });
    expect(
      formatCellValue(50, format({ style: "currency", currencyCode: "SEK" }))
    ).toEqual({ symbol: "", value: "SEK 50.00" });
  });

  it("splits accounting cells into a pinned symbol and a right-aligned value", () => {
    expect(formatCellValue(1234.5, format({ style: "accounting" }))).toEqual({
      symbol: "$",
      value: "1,234.50",
    });
    expect(
      formatCellValue(1234.5, format({ style: "accounting", currencyCode: "GBP" }))
    ).toEqual({ symbol: "£", value: "1,234.50" });
  });

  it("implies parentheses and the zero dash for accounting alone", () => {
    expect(formatCellValue(-1234.5, format({ style: "accounting" }))).toEqual({
      symbol: "$",
      value: "(1,234.50)",
    });
    expect(formatCellValue(0, format({ style: "accounting" }))).toEqual({
      symbol: "$",
      value: "–",
    });
  });

  it("lets explicit flags override the accounting defaults", () => {
    expect(
      formatCellValue(-1, format({ style: "accounting", negativeParens: false }))
    ).toEqual({ symbol: "$", value: "-1.00" });
    expect(
      formatCellValue(0, format({ style: "accounting", zeroDash: false }))
    ).toEqual({ symbol: "$", value: "0.00" });
  });

  it("applies parens and zero dash to other styles when asked", () => {
    expect(
      formatCellValue(-1234.5, format({ style: "currency", negativeParens: true }))
    ).toEqual({ symbol: "", value: "($1,234.50)" });
    expect(
      formatCellValue(
        -0.25,
        format({ style: "percent", negativeParens: true, decimals: 0 })
      )
    ).toEqual({ symbol: "", value: "(25%)" });
    expect(formatCellValue(0, format({ style: "number", zeroDash: true }))).toEqual({
      symbol: "",
      value: "–",
    });
    expect(formatCellValue(-42, format({ decimals: 0 }))).toEqual({
      symbol: "",
      value: "-42",
    });
  });

  it("scales by thousands and millions for display only", () => {
    expect(
      formatCellValue(125000, format({ scale: "thousands", decimals: 0 }))
    ).toEqual({ symbol: "", value: "125" });
    expect(
      formatCellValue(
        2500000,
        format({ style: "currency", scale: "millions", decimals: 1 })
      )
    ).toEqual({ symbol: "", value: "$2.5" });
    expect(
      formatCellValue(
        -750000,
        format({ style: "accounting", scale: "thousands", decimals: 0 })
      )
    ).toEqual({ symbol: "$", value: "(750)" });
  });

  it("multiplies percent values by one hundred", () => {
    expect(formatCellValue(0.05, format({ style: "percent" }))).toEqual({
      symbol: "",
      value: "5%",
    });
    expect(formatCellValue(0.1234, format({ style: "percent", decimals: 1 }))).toEqual({
      symbol: "",
      value: "12.3%",
    });
    expect(formatCellValue(1.5, format({ style: "percent", decimals: 0 }))).toEqual({
      symbol: "",
      value: "150%",
    });
  });

  it("keeps stored precision when no decimals are requested", () => {
    expect(formatCellValue(0.1 + 0.2, format({ style: "percent" }))).toEqual({
      symbol: "",
      value: "30%",
    });
    expect(formatCellValue(1234.56789, format({}))).toEqual({
      symbol: "",
      value: "1,234.56789",
    });
  });
});

describe("formatComputedScalar", () => {
  it("uses the same default numeric presentation outside frame cells", () => {
    expect(
      formatComputedScalar(
        { type: "number", value: 1234.5 },
        "number",
        "1234.5"
      )
    ).toBe("1,234.50");
    expect(
      formatComputedScalar(
        { type: "number", value: 1234.5 },
        "number",
        "1234.5",
        false
      )
    ).toBe("1234.50");
  });

  it("preserves semantic currency and percentage display", () => {
    expect(
      formatComputedScalar(
        { type: "number", value: 1234.5 },
        "currency",
        "$1234.50"
      )
    ).toBe("$1,234.50");
    expect(
      formatComputedScalar(
        { type: "number", value: 0.0425 },
        "percentage",
        "4.25%"
      )
    ).toBe("4.25%");
  });
});

describe("formatCellText", () => {
  it("joins accounting parts with a space and leaves other styles alone", () => {
    expect(formatCellText(-1234.5, format({ style: "accounting" }))).toBe(
      "$ (1,234.50)"
    );
    expect(formatCellText(1234.5, format({ style: "currency" }))).toBe("$1,234.50");
    expect(formatCellText("note", format({ style: "accounting" }))).toBe("note");
  });
});

describe("columnFormatBadge", () => {
  it("labels scaled columns with their unit", () => {
    expect(columnFormatBadge(format({ style: "currency", scale: "thousands" }))).toBe(
      "$K"
    );
    expect(
      columnFormatBadge(
        format({ style: "accounting", scale: "millions", currencyCode: "EUR" })
      )
    ).toBe("€M");
    expect(columnFormatBadge(format({ style: "number", scale: "thousands" }))).toBe(
      "K"
    );
    expect(columnFormatBadge(format({ style: "plain", scale: "millions" }))).toBe("M");
  });

  it("labels percent columns even without scaling", () => {
    expect(columnFormatBadge(format({ style: "percent" }))).toBe("%");
    expect(columnFormatBadge(format({ style: "percent", scale: "thousands" }))).toBe(
      "%K"
    );
  });

  it("returns null when cells already carry the whole story", () => {
    expect(columnFormatBadge(format({ style: "number" }))).toBeNull();
    expect(columnFormatBadge(format({ style: "currency" }))).toBeNull();
    expect(columnFormatBadge(format({ style: "accounting" }))).toBeNull();
    expect(columnFormatBadge(format({ style: "plain" }))).toBeNull();
  });
});
