import type { FormulaReference } from "./formulaReferences";
import type { ComputedBlockLine } from "./types";

/** What one block line may read: earlier members, then the document outside. */
export function blockFormulaReferences(
  lines: ComputedBlockLine[],
  currentLine: number,
  outside: FormulaReference[],
  blockId: string
): FormulaReference[] {
  return [
    ...lines.slice(0, currentLine).flatMap((line) =>
      line.name
        ? [
            {
              id: line.id,
              objectId: blockId,
              label: line.name,
              token: line.name,
              kind: "value" as const,
              detail: "line above",
            },
          ]
        : []
    ),
    ...outside,
  ];
}
