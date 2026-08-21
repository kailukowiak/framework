import { useEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { FloatingMenu } from "./FloatingMenu";
import { acceptCompletionOnce } from "./lib/completionAcceptance";
import { completeFormula } from "./lib/api";
import {
  contextualFormulaReferenceCompletion,
  contextualFormulaReferenceToken,
  formulaToken,
  getFormulaReferenceQuery,
  insertFormulaReference,
  insertionResumesAt,
  type FormulaReference,
} from "./lib/formulaReferences";
import type { FormulaCompletionContext } from "./lib/activeFormulaEditor";
import type { CompletionResult, CompletionSuggestion } from "./lib/types";

const SUGGESTION_KIND_BADGE: Record<CompletionSuggestion["kind"], string> = {
  frame: "FRM",
  column: "COL",
  value: "VAL",
  rootFunction: "FX",
  namespace: "NS",
  method: "FX",
};

const SUGGESTION_KIND_CLASS: Record<CompletionSuggestion["kind"], string> = {
  frame: "frame",
  column: "column",
  value: "value",
  rootFunction: "function",
  namespace: "namespace",
  method: "function",
};

export function argumentAt(signature: string, index: number): string | null {
  const opening = signature.indexOf("(");
  const closing = signature.lastIndexOf(")");
  if (opening < 0 || closing <= opening) return null;
  const argumentsText = signature.slice(opening + 1, closing);
  if (!argumentsText) return null;
  const parameters: string[] = [];
  let start = 0;
  let depth = 0;
  for (let offset = 0; offset < argumentsText.length; offset += 1) {
    const character = argumentsText[offset];
    if (character === "[" || character === "(") depth += 1;
    else if (character === "]" || character === ")") depth -= 1;
    else if (character === "," && depth === 0) {
      parameters.push(argumentsText.slice(start, offset).trim());
      start = offset + 1;
    }
  }
  parameters.push(argumentsText.slice(start).trim());
  return parameters[index] ?? null;
}

export function shouldOfferFormulaSuggestions(
  source: string,
  cursor: number,
  query: string,
  enabled: boolean,
  dismissedAt: string | null = null
): boolean {
  if (!enabled || dismissedAt === `${source}\u0000${cursor}`) return false;
  let quote: string | null = null;
  let escaped = false;
  for (const character of source.slice(0, cursor)) {
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quote && character === "\\") {
      escaped = true;
      continue;
    }
    if (character === '"' || character === "'") {
      if (quote === character) quote = null;
      else if (quote === null) quote = character;
    }
  }
  if (quote) return false;
  const explicitQuery = query.startsWith(".") || query.startsWith("`");
  const implicit = query.replace(/^[.`]/, "");
  return (
    explicitQuery ||
    (/^[\p{L}_]/u.test(implicit) && implicit.length >= 3)
  );
}

export function useFormulaCompletion({
  source,
  cursor,
  enabled,
  context,
  onInsert,
}: {
  source: string;
  cursor: number;
  enabled: boolean;
  context: FormulaCompletionContext;
  onInsert: (source: string, cursor: number) => void;
}) {
  const [activeIndex, setActiveIndex] = useState(0);
  const [typedResponse, setTypedResponse] = useState<{
    position: string;
    result: CompletionResult;
  } | null>(null);
  const [dismissedAt, setDismissedAt] = useState<string | null>(null);
  const requestId = useRef(0);
  const acceptedAt = useRef<string | null>(null);
  const query = getFormulaReferenceQuery(source, cursor);
  const completionPosition = `${source}\u0000${cursor}`;
  const typed =
    typedResponse?.position === completionPosition
      ? typedResponse.result
      : null;
  // A dot or an opening backtick is an explicit request for help. Everywhere
  // else, three characters are enough to establish intent without making a
  // completed expression followed by a space erupt into an unrelated menu.
  const offersSuggestions = shouldOfferFormulaSuggestions(
    source,
    cursor,
    query,
    enabled,
    dismissedAt
  );
  // A frame completion deliberately inserts its trailing dot: accepting
  // `` `Ledger` `` is the first half of choosing `` `Ledger`.`Amount` ``.
  // Until the typed backend response for that new cursor arrives, the local
  // reference list is the visible completion surface. It must preserve that
  // namespace transition instead of offering the frame again; accepting the
  // stale frame would replace the dot with the entire frame token and produce
  // `` `Ledger`.`Ledger`. `` on every further Enter, Tab, or click.
  const fallbackCompletion = contextualFormulaReferenceCompletion(
    context.references,
    source,
    cursor,
    query
  );
  const fallbackSuggestions = offersSuggestions
    ? fallbackCompletion.suggestions.slice(0, 7)
    : [];

  useEffect(() => {
    if (!context.frameId || !enabled) {
      setTypedResponse(null);
      return;
    }
    const id = ++requestId.current;
    // Rebuilt from the two tracked fields rather than passed as
    // `context.scope` directly: that keeps the effect's own dependency
    // list — not `context.scope`'s object identity — in charge of when a
    // fresh request goes out.
    const steps = context.scope?.steps;
    const stepIndex = context.scope?.stepIndex;
    const scope = steps !== undefined && stepIndex !== undefined ? { steps, stepIndex } : undefined;
    const timer = window.setTimeout(() => {
      completeFormula(context.frameId!, source, cursor, scope)
        .then((result) => {
          if (requestId.current === id)
            setTypedResponse({ position: completionPosition, result });
        })
        .catch(() => {
          if (requestId.current === id) setTypedResponse(null);
        });
    }, 80);
    return () => window.clearTimeout(timer);
  }, [
    context.frameId,
    context.scope?.steps,
    context.scope?.stepIndex,
    source,
    cursor,
    enabled,
    completionPosition,
  ]);

  const usingTyped = Boolean(context.frameId) && typed !== null;
  const typedSuggestions = offersSuggestions
    ? typed?.suggestions.slice(0, 8) ?? []
    : [];
  const suggestionCount = usingTyped
    ? typedSuggestions.length
    : fallbackSuggestions.length;
  const activeSuggestionId = usingTyped
    ? (typedSuggestions[activeIndex] ?? typedSuggestions[0])?.id
    : (fallbackSuggestions[activeIndex] ?? fallbackSuggestions[0])?.id;
  const suggestionHelp = context.references.find(
    (reference) =>
      reference.kind === "function" && reference.id === activeSuggestionId
  );
  const parameterHelp = context.references.find(
    (reference) =>
      reference.kind === "function" && reference.id === typed?.activeFunctionId
  );
  const activeParameter =
    parameterHelp?.signature && typed?.activeArgument != null
      ? argumentAt(parameterHelp.signature, typed.activeArgument)
      : null;
  // The core returns the cursor's call and argument index. Pairing that with
  // the catalog entry already carried by this editor gives every formula
  // surface the same explanation, without a second JavaScript function map.
  const activeParameterHelp =
    parameterHelp?.arguments && typed?.activeArgument != null
      ? parameterHelp.arguments[typed.activeArgument]
      : undefined;

  useEffect(() => setActiveIndex(0), [query, typed]);

  const insertReference = (reference: FormulaReference) => {
    acceptCompletionOnce(acceptedAt, completionPosition, () => {
      // The query begins with the dot and is replaced as one token. Put that
      // separator back while inserting only the member missing after it.
      const result = insertFormulaReference(
        source,
        cursor,
        contextualFormulaReferenceToken(reference, fallbackCompletion.qualifier)
      );
      onInsert(result.source, result.cursor);
    });
  };

  const insertTypedSuggestion = (suggestion: CompletionSuggestion) => {
    if (!typed) return;
    acceptCompletionOnce(acceptedAt, completionPosition, () => {
      const updated = `${source.slice(0, typed.replaceStart)}${
        suggestion.insertText
      }${source.slice(insertionResumesAt(source, cursor, suggestion.insertText))}`;
      onInsert(updated, typed.replaceStart + suggestion.insertText.length);
    });
  };

  const insertActive = (index = activeIndex) => {
    if (usingTyped) {
      const suggestion = typedSuggestions[index] ?? typedSuggestions[0];
      if (suggestion) insertTypedSuggestion(suggestion);
    } else {
      const reference = fallbackSuggestions[index] ?? fallbackSuggestions[0];
      if (reference) insertReference(reference);
    }
  };

  const dismissSuggestions = () => setDismissedAt(completionPosition);

  return {
    activeIndex,
    setActiveIndex,
    query,
    typed,
    usingTyped,
    typedSuggestions,
    fallbackSuggestions,
    suggestionCount,
    suggestionHelp,
    parameterHelp,
    activeParameter,
    activeParameterHelp,
    offersSuggestions,
    dismissSuggestions,
    insertActive,
    insertReference,
    insertTypedSuggestion,
  };
}

export type FormulaCompletionState = ReturnType<typeof useFormulaCompletion>;

export function FormulaCompletionMenu({
  completion,
  anchorRef,
}: {
  completion: FormulaCompletionState;
  anchorRef: RefObject<HTMLElement | null>;
}) {
  const {
    activeIndex,
    setActiveIndex,
    typed,
    usingTyped,
    typedSuggestions,
    fallbackSuggestions,
    suggestionHelp,
    parameterHelp,
    activeParameter,
    activeParameterHelp,
    offersSuggestions,
    insertReference,
    insertTypedSuggestion,
  } = completion;

  if (
    offersSuggestions &&
    usingTyped &&
    typedSuggestions.length === 0 &&
    typed?.note
  )
    return (
      <FloatingMenu anchorRef={anchorRef} className="reference-menu reference-menu-note">
        {typed.note}
      </FloatingMenu>
    );
  if (usingTyped && typedSuggestions.length > 0)
    return (
      <FloatingMenu anchorRef={anchorRef} className="reference-menu">
        {typedSuggestions.map((suggestion, index) => (
          <button
            type="button"
            key={suggestion.id}
            className={index === activeIndex ? "active" : ""}
            onMouseDown={(event) => {
              event.preventDefault();
              insertTypedSuggestion(suggestion);
            }}
            onMouseEnter={() => setActiveIndex(index)}
          >
            <span
              className={`reference-kind ${SUGGESTION_KIND_CLASS[suggestion.kind]}`}
            >
              {SUGGESTION_KIND_BADGE[suggestion.kind]}
            </span>
            <span className="reference-copy">
              <strong>{suggestion.label}</strong>
              <small>{suggestion.detail}</small>
            </span>
            <span className="reference-label">{typed?.receiverDtype ?? ""}</span>
          </button>
        ))}
        <FunctionHelp reference={suggestionHelp} />
      </FloatingMenu>
    );
  if (!usingTyped && fallbackSuggestions.length > 0)
    return (
      <FloatingMenu anchorRef={anchorRef} className="reference-menu">
        {fallbackSuggestions.map((reference, index) => (
          <button
            type="button"
            key={`${reference.kind}-${reference.id}`}
            className={index === activeIndex ? "active" : ""}
            onMouseDown={(event) => {
              event.preventDefault();
              insertReference(reference);
            }}
            onMouseEnter={() => setActiveIndex(index)}
          >
            <span className={`reference-kind ${reference.kind}`}>
              {reference.kind === "column"
                ? "COL"
                : reference.kind === "frame"
                  ? "TBL"
                : reference.kind === "value"
                  ? "VAL"
                  : "FX"}
            </span>
            <span className="reference-copy">
              <strong>{reference.token}</strong>
              <small>{reference.detail}</small>
            </span>
            <span className="reference-label">
              {reference.kind === "function" &&
              formulaToken(reference.label) !== reference.token
                ? reference.label
                : ""}
            </span>
          </button>
        ))}
        <FunctionHelp reference={suggestionHelp} />
      </FloatingMenu>
    );
  // Completion is not only a menu. Once the cursor is inside a call, a
  // single dense hint is more useful than a list of unrelated identifiers.
  if (parameterHelp && activeParameterHelp)
    return (
      <FloatingMenu
        anchorRef={anchorRef}
        className="reference-menu formula-parameter-menu"
      >
        <FunctionHelp
          reference={parameterHelp}
          parameter={activeParameterHelp}
          parameterLabel={activeParameter}
        />
      </FloatingMenu>
    );
  return null;
}

function FunctionHelp({
  reference,
  parameter,
  parameterLabel,
}: {
  reference?: FormulaReference;
  parameter?: NonNullable<FormulaReference["arguments"]>[number];
  parameterLabel?: string | null;
}) {
  if (!reference?.signature || !reference.description) return null;
  return (
    <div className="formula-function-help">
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
    </div>
  );
}
