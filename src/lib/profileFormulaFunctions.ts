import type { FormulaFunction } from "./types";

/** Formula methods whose public spelling is defined by the profile drawer. */
export const profileFormulaFunctions: FormulaFunction[] = [
  {
    id: "expr.quantile",
    name: ".quantile",
    aliases: ["percentile", "quartile"],
    category: "Aggregation",
    signature: ".quantile(fraction)",
    description:
      "Find a percentile using linear interpolation; 0.25 is the first quartile.",
    minimumArguments: 1,
    maximumArguments: 1,
    returnType: "number",
    nullBehavior: "native Polars behavior",
    arguments: [],
  },
];
