export type GridCellFormulaAction =
  | { kind: "edit"; seed: string | null }
  | { kind: "clear" }
  | { kind: "column" };

/** Formula-scope keys routed before literal grid editing sees them. */
export function gridCellFormulaAction({
  key,
  modifier,
  printable,
  isOverride,
  singleCell,
}: {
  key: string;
  modifier: boolean;
  printable: boolean;
  isOverride: boolean;
  singleCell: boolean;
}): GridCellFormulaAction | null {
  // `=` is a doorway, not a place to type. From any cell, or a selected
  // column, it opens that column's formula in Wrangle, where the whole
  // column is visibly the subject: a column is the declaration slot a
  // spreadsheet hand is reaching for. Excel fills a range; FrameWork fills
  // a column. A legacy override keeps its own editor so old documents are
  // not stranded, but nothing new is ever stored in a cell.
  if (key === "=" && !modifier)
    return isOverride && singleCell ? { kind: "edit", seed: "" } : { kind: "column" };
  if (key === "F2" && isOverride) return { kind: "edit", seed: null };
  if ((key === "Delete" || key === "Backspace") && isOverride && singleCell)
    return { kind: "clear" };
  if (printable && isOverride) return { kind: "edit", seed: key };
  return null;
}
