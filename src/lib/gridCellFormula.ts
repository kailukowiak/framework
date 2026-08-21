export type GridCellFormulaAction =
  | { kind: "edit"; seed: string | null }
  | { kind: "clear" }
  | { kind: "column" }
  | { kind: "scratchwork" };

/** Formula-scope keys routed before literal grid editing sees them. */
export function gridCellFormulaAction({
  key,
  modifier,
  printable,
  isOverride,
  singleCell,
  wholeColumn,
}: {
  key: string;
  modifier: boolean;
  printable: boolean;
  isOverride: boolean;
  singleCell: boolean;
  wholeColumn: boolean;
}): GridCellFormulaAction | null {
  if (key === "=" && !modifier && wholeColumn) return { kind: "column" };
  if (key === "F2" && isOverride) return { kind: "edit", seed: null };
  if ((key === "Delete" || key === "Backspace") && isOverride && singleCell)
    return { kind: "clear" };
  // A single cell is a value, not another formula scope. `=` still has a
  // useful spreadsheet-shaped meaning here, but the calculation belongs in
  // Scratchwork where it has a name, a durable address, and no hidden
  // exception to a typed column. Existing overrides remain editable so old
  // documents are not stranded; this gesture no longer creates a new one.
  if (key === "=" && !modifier)
    return isOverride && singleCell
      ? { kind: "edit", seed: "" }
      : { kind: "scratchwork" };
  if (printable && isOverride) return { kind: "edit", seed: key };
  return null;
}
