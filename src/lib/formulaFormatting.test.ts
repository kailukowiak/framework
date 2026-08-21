import { describe, expect, it } from "vitest";
import { formatFormulaChains } from "./formulaFormatting";

describe("formatFormulaChains", () => {
  it("lays out nested method chains by their receiver depth", () => {
    const source =
      "`Date` = when(`Controls`.`Date`.dt.day() <= 15).then(`Controls`.`Date`.dt.month_start()).otherwise(1.25)";
    expect(formatFormulaChains(source).source).toBe(
      "`Date` = when(`Controls`.`Date`\n    .dt\n    .day() <= 15)\n  .then(`Controls`.`Date`\n    .dt\n    .month_start())\n  .otherwise(1.25)"
    );
  });

  it("does not alter dots inside strings or backticked names", () => {
    const source = '`A.B`.str.replace("a.b", "c.d")';
    expect(formatFormulaChains(source).source).toBe(
      '`A.B`\n  .str\n  .replace("a.b", "c.d")'
    );
  });

  it("is idempotent and carries the selection past inserted whitespace", () => {
    const source = "value.somefunc().other()";
    const once = formatFormulaChains(source, source.length, source.length);
    expect(once.selection).toEqual({
      start: once.source.length,
      end: once.source.length,
    });
    expect(formatFormulaChains(once.source).source).toBe(once.source);
  });
});
