import { ChevronDown } from "lucide-react";

export function FormulaBarActions({
  canFormat,
  expanded,
  onFormat,
  onToggle,
}: {
  canFormat: boolean;
  expanded: boolean;
  onFormat: () => void;
  onToggle: () => void;
}) {
  return (
    <>
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
        onPointerDown={(event) => event.preventDefault()}
        onClick={onToggle}
      >
        <span>Scratchwork</span>
        <ChevronDown className={expanded ? "expanded" : ""} size={13} />
      </button>
    </>
  );
}
