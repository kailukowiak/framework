import { ChevronDown, ChevronsUpDown } from "lucide-react";

export function FormulaBarActions({
  canFormat,
  expanded,
  onFormat,
  onToggle,
  formulaExpanded,
  onToggleFormula,
}: {
  formulaExpanded: boolean;
  onToggleFormula: () => void;
  canFormat: boolean;
  expanded: boolean;
  onFormat: () => void;
  onToggle: () => void;
}) {
  return (
    <>
      <button type="button" className="scratchwork-formula-expand"
        aria-label={formulaExpanded ? "Collapse formula bar" : "Expand formula bar"}
        aria-expanded={formulaExpanded}
        title={formulaExpanded ? "Collapse formula bar" : "Expand formula bar"}
        onPointerDown={(event) => event.preventDefault()} onClick={onToggleFormula}>
        <ChevronsUpDown size={13} />
      </button>
      {canFormat && (
        <button
          type="button"
          className="scratchwork-formula-format"
          onPointerDown={(event) => event.preventDefault()}
          onClick={onFormat}
        >
          Format
        </button>
      )}
      <button
        type="button"
        className="scratchwork-formula-toggle"
        aria-expanded={expanded}
        aria-controls="scratchwork-drawer"
        data-shortcut="⌘J"
        title="Scratchwork (⌘J)"
        onPointerDown={(event) => event.preventDefault()}
        onClick={onToggle}
      >
        <span>Scratchwork</span>
        <ChevronDown className={expanded ? "expanded" : ""} size={13} />
      </button>
    </>
  );
}
