import { ArrowDown, ArrowUp, Eraser, Replace } from "lucide-react";
import { formulaToken } from "./lib/formulaReferences";
import type { Column } from "./lib/types";

export type ColumnQuickAction = {
  label: string;
  formula: string;
  Icon: typeof Eraser;
};

/** Common spreadsheet cleanup gestures, expressed as ordinary Wrangle formulas. */
export function columnQuickActions(column: Column): ColumnQuickAction[] {
  const token = formulaToken(column.name);
  if (column.dataType === "string") {
    return [
      {
        label: "Trim whitespace",
        formula: `${token}.str.strip_chars(None)`,
        Icon: Eraser,
      },
      {
        label: "Make uppercase",
        formula: `${token}.str.to_uppercase()`,
        Icon: ArrowUp,
      },
      {
        label: "Make lowercase",
        formula: `${token}.str.to_lowercase()`,
        Icon: ArrowDown,
      },
    ];
  }
  if (["integer", "number", "currency", "percentage"].includes(column.dataType)) {
    return [
      {
        label: "Fill missing with 0",
        formula: `${token}.fill_null(0)`,
        Icon: Replace,
      },
    ];
  }
  if (column.dataType === "boolean") {
    return [
      {
        label: "Fill missing with False",
        formula: `${token}.fill_null(False)`,
        Icon: Replace,
      },
    ];
  }
  return [];
}

export function ColumnQuickActions({
  column,
  onTransform,
}: {
  column: Column;
  onTransform: (formula: string) => void;
}) {
  const actions = columnQuickActions(column);
  if (!actions.length) return null;
  return (
    <>
      {actions.map(({ label, formula, Icon }) => (
        <button key={label} onClick={() => onTransform(formula)}>
          <Icon size={14} />
          <span>{label}</span>
        </button>
      ))}
      <span className="menu-separator" />
    </>
  );
}
