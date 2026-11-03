import type { FormulaArgument } from "./lib/bindings/FormulaArgument";
import type { FormulaReference } from "./lib/formulaReferences";
import { useHelpPanelContext } from "./lib/helpPanel";
import { functionExample } from "./lib/helpSearch";

/**
 * The highlighted function's explanation, under the suggestions: its
 * signature, one line of description or of the argument being typed, and a
 * worked call where the signature alone does not show the shape.
 *
 * It is also the way into the full entry. Clicking it opens help on that
 * function beside the canvas without taking the formula's focus, so the menu
 * needs no separate "more" control. Every completion menu mounts this one,
 * so a Scratchwork line explains a function the same way a Wrangle step does.
 */
export function FunctionHelp({
  reference,
  parameter,
  parameterLabel,
}: {
  reference?: FormulaReference;
  parameter?: FormulaArgument;
  parameterLabel?: string | null;
}) {
  const help = useHelpPanelContext();
  if (!reference?.signature || !reference.description) return null;
  const example = parameter ? undefined : functionExample(reference.id);
  const content = (
    <>
      <code>{reference.signature}</code>
      {parameter ? (
        <span>
          <strong>{parameterLabel ?? parameter.name}</strong>
          {parameter.required ? " · " : " · optional · "}
          {parameter.description}
          {parameter.example && (
            <>
              {" "}Try <code>{parameter.example}</code>.
            </>
          )}
        </span>
      ) : (
        <span>{reference.description}</span>
      )}
      {example && <code className="formula-function-example">{example}</code>}
    </>
  );
  if (!help) return <div className="formula-function-help">{content}</div>;
  return (
    <button
      type="button"
      className="formula-function-help"
      title="Open in help"
      // The editor keeps focus: the click is a request to read, not a
      // decision to stop writing.
      onMouseDown={(event) => event.preventDefault()}
      onClick={() =>
        help.open({ query: reference.label, entryId: reference.id, keepFocus: true })
      }
    >
      {content}
    </button>
  );
}
