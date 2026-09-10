import type { FormulaBarCell } from "./lib/formulaBarCell";

/** What the bar says about the selected cell while no session owns it. */
export function SelectedCellNotice({
  cell,
  error,
  dirty,
}: {
  cell: FormulaBarCell;
  error: string | null;
  dirty: boolean;
}) {
  return (
    <span
      className={`scratchwork-formula-answer${
        error || cell.kind === "readOnly" ? " invalid" : ""
      }`}
      title={error ?? cell.reason}
    >
      <strong>{cell.label}</strong>
      {error
        ? ` · ${error}`
        : cell.kind === "calculated"
        ? " · calculated column · click to edit"
        : cell.kind === "override"
        ? " · legacy cell formula · click to inspect"
        : cell.kind === "readOnly"
        ? ` · ${cell.reason}`
        : dirty
        ? " · Enter saves"
        : " · literal value"}
    </span>
  );
}

