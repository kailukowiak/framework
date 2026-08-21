import { FormulaReferenceLegend } from "./FormulaReferenceLegend";
import type { ActiveFormulaEditor } from "./lib/activeFormulaEditor";
import type { FormulaReference } from "./lib/formulaReferences";
import { formulaReferenceDecorations } from "./lib/formulaReferenceDecorations";

export function formulaScopeReading(active: ActiveFormulaEditor): string {
  if (active.completion.appliesToAllRows) return "all rows";
  if (active.completion.appliesToAllRows === false) return "this cell";
  return "";
}

function shiftNeedsDeclaredOrder(
  active: ActiveFormulaEditor,
  draft: string
): boolean {
  const scope = active.completion.scope;
  const ordered =
    active.completion.orderingDeclared ??
    scope?.steps
      .slice(0, scope.stepIndex)
      .some((step) => step.kind === "sort") ??
    true;
  return /\.shift\s*\(/.test(draft) && !ordered;
}

/** The persistent scope label and compact reference legend beside the bar. */
export function ActiveFormulaAnswer({
  active,
  draft,
  references,
  onSelect,
}: {
  active: ActiveFormulaEditor;
  draft: string;
  references: FormulaReference[];
  onSelect: (start: number, end: number) => void;
}) {
  const needsOrder = shiftNeedsDeclaredOrder(active, draft);
  const decorated = formulaReferenceDecorations(draft, references).length > 0;
  const scope = formulaScopeReading(active);
  return (
    <span
      className={`scratchwork-formula-answer${needsOrder ? " guidance" : ""}`}
    >
      <strong>{active.label}</strong>
      {scope && ` · ${scope}`}
      {needsOrder ? (
        " · previous/next row needs a Sort step above"
      ) : decorated ? (
        <>
          {" · "}
          <FormulaReferenceLegend
            source={draft}
            references={references}
            onSelect={onSelect}
          />
        </>
      ) : (
        " · click a cell or column to reference"
      )}
    </span>
  );
}
