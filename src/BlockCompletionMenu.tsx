import type { RefObject } from "react";
import { FloatingMenu } from "./FloatingMenu";
import type { FormulaReference } from "./lib/formulaReferences";

export function BlockCompletionMenu({
  anchorRef,
  suggestions,
  activeIndex,
  onTake,
}: {
  anchorRef: RefObject<HTMLElement | null>;
  suggestions: FormulaReference[];
  activeIndex: number;
  onTake: (suggestion: FormulaReference) => void;
}) {
  return (
    <FloatingMenu anchorRef={anchorRef} className="block-suggestions">
      <ul>
        {suggestions.map((suggestion, index) => (
          <li key={suggestion.id}>
            <button
              type="button"
              className={index === activeIndex ? "active" : ""}
              // The blur would close the list before the click landed.
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onTake(suggestion)}
            >
              <span>{suggestion.label}</span>
              <small>{suggestion.detail}</small>
            </button>
          </li>
        ))}
        <li className="block-suggestions-hint">Tab</li>
      </ul>
    </FloatingMenu>
  );
}
