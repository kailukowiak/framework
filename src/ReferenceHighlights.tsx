import { useLayoutEffect } from "react";
import { useActiveFormulaEditor } from "./ActiveFormulaEditor";
import { formulaReferenceDecorations } from "./lib/formulaReferenceDecorations";
import type { FormulaReference } from "./lib/formulaReferences";
import { scratchworkLineAt } from "./lib/scratchwork";

type MarkKind =
  | "formulaReferenceColor"
  | "formulaReferenceHeaderColor"
  | "formulaReferenceObjectColor";

function mark(
  marked: Set<HTMLElement>,
  element: HTMLElement,
  kind: MarkKind,
  colorIndex: number
) {
  element.dataset[kind] = String(colorIndex);
  element.classList.add(`formula-ref-color-${colorIndex}`);
  marked.add(element);
}

function rowIndexOf(element: HTMLElement): number | undefined {
  const holder = element.closest<HTMLElement>("[data-row-index]");
  const value = holder?.dataset.rowIndex;
  if (value === undefined) return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

/** The actual painted cells represented by either frame orientation. */
function columnElements(reference: FormulaReference): HTMLElement[] {
  const found = new Set<HTMLElement>();
  for (const owner of window.document.querySelectorAll<HTMLElement>(
    "[data-column-id]"
  )) {
    if (owner.dataset.columnId !== reference.id) continue;
    const frameId = owner.closest<HTMLElement>("[data-frame-id]")?.dataset.frameId;
    if (reference.frameId && frameId !== reference.frameId) continue;
    if (owner instanceof HTMLTableRowElement) {
      for (const cell of owner.querySelectorAll<HTMLElement>("th, td")) found.add(cell);
    } else found.add(owner);
  }
  return [...found];
}

function markColumnReference(
  marked: Set<HTMLElement>,
  reference: FormulaReference,
  colorIndex: number,
  sourceRowRange: { start: number; end: number } | undefined
) {
  for (const element of columnElements(reference)) {
    const rowIndex = rowIndexOf(element);
    if (rowIndex === undefined) {
      mark(marked, element, "formulaReferenceHeaderColor", colorIndex);
    } else if (
      sourceRowRange === undefined ||
      (rowIndex >= sourceRowRange.start && rowIndex <= sourceRowRange.end)
    ) {
      mark(marked, element, "formulaReferenceColor", colorIndex);
    }
  }
}

function cleanup(marked: Set<HTMLElement>) {
  for (const element of marked) {
    for (let index = 0; index < 6; index += 1)
      element.classList.remove(`formula-ref-color-${index}`);
    delete element.dataset.formulaReferenceColor;
    delete element.dataset.formulaReferenceHeaderColor;
    delete element.dataset.formulaReferenceObjectColor;
    delete element.dataset.formulaTarget;
  }
}

/**
 * Paint the places named by the one active reference-bearing editor.
 *
 * DOM marking is deliberate here. Subscribing the whole App and every grid
 * cell to each formula keystroke would make typing rebuild the canvas; this
 * isolated component updates only the already-rendered source elements and
 * removes every transient mark when editing ends.
 */
export function ReferenceHighlights() {
  const { active } = useActiveFormulaEditor();
  useLayoutEffect(() => {
    if (!active?.focused) return;
    const marked = new Set<HTMLElement>();
    const activeSource =
      active.kind === "scratchwork"
        ? scratchworkLineAt(active.draft, active.selection.end).source
        : active.draft;
    const decorations = formulaReferenceDecorations(
      activeSource,
      active.completion.references
    );
    for (const decoration of decorations) {
      const reference = decoration.reference;
      if (reference.kind === "column") {
        const sharesAnchorFrame =
          active.completion.anchorFrameId !== undefined &&
          (!reference.frameId ||
            reference.frameId === active.completion.anchorFrameId);
        const anchoredRowIndex = sharesAnchorFrame
          ? active.completion.anchorRowIndex === undefined
            ? undefined
            : active.completion.anchorRowIndex - decoration.rowOffset
          : undefined;
        const sourceRowRange =
          decoration.rowRange ??
          (anchoredRowIndex === undefined
            ? undefined
            : { start: anchoredRowIndex, end: anchoredRowIndex });
        markColumnReference(
          marked,
          reference,
          decoration.colorIndex,
          sourceRowRange
        );
      } else if (reference.objectId) {
        for (const element of window.document.querySelectorAll<HTMLElement>(
          "[data-object-id]"
        )) {
          if (element.dataset.objectId === reference.objectId)
            mark(
              marked,
              element,
              "formulaReferenceObjectColor",
              decoration.colorIndex
            );
        }
      }
    }
    const targetId = active.completion.targetColumnId;
    if (targetId) {
      const target: FormulaReference = {
        id: targetId,
        label: "target",
        token: "",
        kind: "column",
        detail: "formula target",
        frameId: active.completion.anchorFrameId,
      };
      for (const element of columnElements(target)) {
        const rowIndex = rowIndexOf(element);
        if (
          rowIndex === undefined ||
          active.completion.anchorRowIndex === undefined ||
          rowIndex === active.completion.anchorRowIndex
        ) {
          element.dataset.formulaTarget = "true";
          marked.add(element);
        }
      }
    }
    return () => cleanup(marked);
  }, [active]);
  return null;
}
