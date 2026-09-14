import { describe, expect, it } from "vitest";
import { displayedColumnFormat } from "./FrameCells";
import type { Column, DataType } from "./lib/types";

function column(dataType: DataType, extra: Partial<Column> = {}): Column {
  return {
    id: "c1",
    name: "Amount",
    dataType,
    formula: null,
    ...extra,
  } as Column;
}

describe("displayedColumnFormat", () => {
  it("gives an accounting column its presentation without a second gesture", () => {
    // The type is the whole declaration: choosing Accounting must render as
    // accounting, at the places the column declares, with nobody opening
    // Format afterwards to say what the type already said.
    expect(displayedColumnFormat(column("accounting"))).toEqual({
      style: "accounting",
      decimals: 2,
    });
    expect(displayedColumnFormat(column("accounting", { scale: 4 }))).toEqual({
      style: "accounting",
      decimals: 4,
    });
    expect(displayedColumnFormat(column("accounting", { scale: 0 }))).toEqual({
      style: "accounting",
      decimals: 0,
    });
  });

  it("still lets an explicit format override the type's presentation", () => {
    const format = { style: "number" as const, decimals: 1 };
    expect(displayedColumnFormat(column("accounting", { format }))).toBe(format);
  });

  it("leaves the other typed defaults alone", () => {
    expect(displayedColumnFormat(column("currency"))).toEqual({
      style: "currency",
      decimals: null,
    });
    expect(displayedColumnFormat(column("string"))).toBeNull();
  });
});
