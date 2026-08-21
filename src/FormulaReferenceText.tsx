import {
  forwardRef,
  type ReactNode,
  type TextareaHTMLAttributes,
  useImperativeHandle,
  useRef,
} from "react";
import {
  formulaReferenceDecorations,
  type FormulaReferenceDecoration,
} from "./lib/formulaReferenceDecorations";
import type { FormulaReference } from "./lib/formulaReferences";

export function FormulaReferenceText({
  source,
  references,
}: {
  source: string;
  references: FormulaReference[];
}) {
  const decorations = formulaReferenceDecorations(source, references);
  const parts: ReactNode[] = [];
  let index = 0;
  for (const decoration of decorations) {
    if (decoration.start > index) parts.push(source.slice(index, decoration.start));
    parts.push(referenceSpan(source, decoration));
    index = decoration.end;
  }
  if (index < source.length) parts.push(source.slice(index));
  // A final newline otherwise contributes no height to a pre, while it does
  // to a textarea. The zero-width marker keeps both layers metrically equal.
  if (source.endsWith("\n")) parts.push("\u200b");
  return <>{parts}</>;
}

function referenceSpan(source: string, decoration: FormulaReferenceDecoration) {
  return (
    <span
      key={`${decoration.start}-${decoration.reference.id}`}
      className={`formula-reference-text formula-ref-color-${decoration.colorIndex}`}
      data-reference-id={decoration.reference.id}
    >
      {source.slice(decoration.start, decoration.end)}
    </span>
  );
}

type HighlightedFormulaTextareaProps = Omit<
  TextareaHTMLAttributes<HTMLTextAreaElement>,
  "value"
> & {
  value: string;
  references: FormulaReference[];
  layerClassName?: string;
};

/** A native textarea with a read-only, reference-coloured mirror behind it. */
export const HighlightedFormulaTextarea = forwardRef<
  HTMLTextAreaElement,
  HighlightedFormulaTextareaProps
>(function HighlightedFormulaTextarea(
  { value, references, layerClassName = "", onScroll, ...props },
  forwardedRef
) {
  const textarea = useRef<HTMLTextAreaElement>(null);
  const mirror = useRef<HTMLPreElement>(null);
  useImperativeHandle(forwardedRef, () => textarea.current!, []);
  return (
    <div className={`formula-textarea-layer ${layerClassName}`.trim()}>
      <pre className="formula-textarea-highlight" aria-hidden ref={mirror}>
        <FormulaReferenceText source={value} references={references} />
      </pre>
      <textarea
        {...props}
        ref={textarea}
        value={value}
        onScroll={(event) => {
          if (mirror.current) {
            mirror.current.scrollTop = event.currentTarget.scrollTop;
            mirror.current.scrollLeft = event.currentTarget.scrollLeft;
          }
          onScroll?.(event);
        }}
      />
    </div>
  );
});
