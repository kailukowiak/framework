import type { PointerEvent as ReactPointerEvent } from "react";
import type { ActiveFormulaEditor } from "./lib/activeFormulaEditor";
import type { RecurrenceState } from "./ColumnAuthoringDialogs";
import {
  formulaCellRangePick,
  formulaColumnPick,
  formulaSummaryPick,
} from "./lib/formulaPicking";
import { formulaReferenceDecorations } from "./lib/formulaReferenceDecorations";
import type { DocumentView, SummaryOperation, FrameObject } from "./lib/types";

type PickingOptions = {
  document: DocumentView;
  getActive: () => ActiveFormulaEditor | null;
  insertReference: (token: string, refocus?: boolean) => void;
  clear: () => void;
  disengage: () => void;
  onNotice: (notice: string | null) => void;
  onRecurrence: (state: RecurrenceState) => void;
};

function rowIndex(target: HTMLElement): number | undefined {
  const raw = target.closest<HTMLElement>("[data-row-index]")?.dataset.rowIndex;
  const parsed = raw === undefined ? NaN : Number(raw);
  return Number.isFinite(parsed) ? parsed : undefined;
}

type PointedCell = {
  element: HTMLElement;
  columnId: string;
  frameId: string;
  rowIndex: number;
};

function pointedCell(target: EventTarget | null): PointedCell | null {
  if (!(target instanceof HTMLElement)) return null;
  const element = target.closest<HTMLElement>("td[data-column-id], tr[data-column-id]");
  const columnId = element?.dataset.columnId;
  const frameId = element?.closest<HTMLElement>("[data-frame-id]")?.dataset.frameId;
  const pickedRow = element ? rowIndex(element) : undefined;
  return element && columnId && frameId && pickedRow !== undefined
    ? { element, columnId, frameId, rowIndex: pickedRow }
    : null;
}

function cellUnderPointer(event: PointerEvent): PointedCell | null {
  return (
    pointedCell(event.target) ??
    pointedCell(window.document.elementFromPoint(event.clientX, event.clientY))
  );
}

function paintRangePreview(start: PointedCell, end: PointedCell): Set<HTMLElement> {
  const painted = new Set<HTMLElement>();
  if (start.frameId !== end.frameId || start.columnId !== end.columnId) return painted;
  const first = Math.min(start.rowIndex, end.rowIndex);
  const last = Math.max(start.rowIndex, end.rowIndex);
  for (const candidate of window.document.querySelectorAll<HTMLElement>(
    "td[data-column-id], tr[data-column-id]"
  )) {
    if (candidate.dataset.columnId !== start.columnId) continue;
    if (candidate.closest<HTMLElement>("[data-frame-id]")?.dataset.frameId !== start.frameId)
      continue;
    const candidateRow = rowIndex(candidate);
    if (candidateRow === undefined || candidateRow < first || candidateRow > last) continue;
    candidate.classList.add("formula-pick-range-preview");
    painted.add(candidate);
  }
  return painted;
}

function clearRangePreview(painted: Set<HTMLElement>) {
  for (const element of painted) element.classList.remove("formula-pick-range-preview");
}

/**
 * Whether the current ordinal spelling still means the clicked internal row.
 *
 * Formula references do not persist screen coordinates. A document-owned
 * frame with no transformation or display ordering has the one honest
 * exception: its visible ordinal is its stored ordinal. This is an encoding
 * limit, not a general restriction on formulas over live data. Imported and
 * derived cells have no durable identity; a filtered or sorted internal cell
 * can be broadened later by putting its literal row id in the expression tree,
 * never by pretending the displayed ordinal is stable.
 */
function hasStableCellAddresses(document: DocumentView, frameId: string): boolean {
  const frame = document.objects.find(
    (object): object is FrameObject => object.kind === "frame" && object.id === frameId
  );
  const computed = document.computedFrames[frameId];
  return Boolean(
    frame &&
      computed?.editing.rows &&
      !computed.live &&
      frame.derivation === null &&
      !(frame.steps?.length) &&
      !(frame.display?.steps?.length)
  );
}

const SUMMARY_OPERATIONS = new Set<SummaryOperation>([
  "sum",
  "mean",
  "quartile25",
  "median",
  "quartile75",
  "min",
  "max",
  "count",
  "missing",
  "countDistinct",
  "mode",
]);

function trySummaryPick(
  event: ReactPointerEvent,
  target: HTMLElement,
  active: ActiveFormulaEditor,
  editingFromBar: boolean,
  options: PickingOptions
): boolean {
  const cell = target.closest<HTMLElement>("[data-summary-operation]");
  if (!cell || event.button !== 0) return false;
  const operation = cell.dataset.summaryOperation as SummaryOperation | undefined;
  const columnId = cell.dataset.columnId;
  event.preventDefault();
  event.stopPropagation();
  if (!operation || !SUMMARY_OPERATIONS.has(operation) || !columnId) {
    options.onNotice("That profile cell has no formula address.");
    return true;
  }
  if (cell.dataset.summaryReferenceable !== "true") {
    options.onNotice("That statistic does not apply to this column.");
    return true;
  }
  const pick = formulaSummaryPick(active, operation, columnId);
  if (pick.kind === "insert") {
    options.insertReference(pick.token, !editingFromBar);
    options.onNotice(null);
  } else {
    options.onNotice(pick.message);
  }
  return true;
}

function tryColumnPick(
  event: ReactPointerEvent,
  target: HTMLElement,
  active: ActiveFormulaEditor,
  editingFromBar: boolean,
  options: PickingOptions
): boolean {
  const columnId = target.closest<HTMLElement>("[data-column-id]")?.dataset.columnId;
  const frameId = target.closest<HTMLElement>("[data-frame-id]")?.dataset.frameId;
  const hitControl = target.closest("button, input, textarea, select, a");
  const semanticHeader = target.closest("button.column-select");
  if (event.button !== 0 || !columnId || !frameId || (hitControl && !semanticHeader))
    return false;
  event.preventDefault();
  event.stopPropagation();
  const pickedRow = rowIndex(target);
  const pick = formulaColumnPick(
    active,
    columnId,
    frameId,
    pickedRow,
    active.kind !== "scratchwork" ||
      pickedRow === undefined ||
      hasStableCellAddresses(options.document, frameId)
  );
  if (pick.kind === "insert") {
    options.insertReference(pick.token, !editingFromBar);
    options.onNotice(null);
    return true;
  }
  if (pick.kind === "recurrence") {
    const source = formulaReferenceDecorations(
      active.draft,
      active.completion.references
    )
      .map((decoration) => decoration.reference)
      .find(
        (reference) =>
          reference.kind === "column" &&
          reference.id !== active.completion.targetColumnId
      );
    options.clear();
    options.onRecurrence({
      frameId,
      targetColumnId: columnId,
      viewId: target.closest<HTMLElement>("[data-view-id]")?.dataset.viewId,
      initialSourceColumnId: source?.id,
    });
    options.onNotice(null);
    return true;
  }
  const frame = options.document.objects.find(
    (object): object is FrameObject => object.kind === "frame" && object.id === frameId
  );
  const column = frame?.columns.find((candidate) => candidate.id === columnId);
  options.onNotice(
    pick.message ?? `${column?.name ?? "That column"} is not available to ${active.label}.`
  );
  return true;
}

function tryScratchworkRangePick(
  event: ReactPointerEvent,
  target: HTMLElement,
  active: ActiveFormulaEditor,
  editingFromBar: boolean,
  options: PickingOptions
): boolean {
  if (active.kind !== "scratchwork" || event.button !== 0) return false;
  const start = pointedCell(target);
  if (!start) return false;
  const control = target.closest("button, input, textarea, select, a");
  if (control && !control.matches("button.computed-cell")) return false;
  event.preventDefault();
  event.stopPropagation();
  let end = start;
  let painted = paintRangePreview(start, end);
  const move = (moveEvent: PointerEvent) => {
    const next = cellUnderPointer(moveEvent);
    if (!next) return;
    clearRangePreview(painted);
    end = next;
    painted = paintRangePreview(start, end);
  };
  const cleanup = () => {
    clearRangePreview(painted);
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", finish);
    window.removeEventListener("pointercancel", cancel);
  };
  const cancel = () => cleanup();
  const finish = (upEvent: PointerEvent) => {
    const pointed = cellUnderPointer(upEvent);
    if (pointed) end = pointed;
    cleanup();
    if (start.frameId !== end.frameId || start.columnId !== end.columnId) {
      options.onNotice("Drag within one column to insert a row slice.");
      return;
    }
    const pick = formulaCellRangePick(
      active,
      start.columnId,
      start.frameId,
      start.rowIndex,
      end.rowIndex,
      hasStableCellAddresses(options.document, start.frameId)
    );
    if (pick.kind === "insert") {
      options.insertReference(pick.token, !editingFromBar);
      options.onNotice(null);
    } else options.onNotice(pick.message);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", finish);
  window.addEventListener("pointercancel", cancel);
  return true;
}

function tryObjectPick(
  event: ReactPointerEvent,
  target: HTMLElement,
  active: ActiveFormulaEditor,
  editingFromBar: boolean,
  options: PickingOptions
): boolean {
  const objectId = target.closest<HTMLElement>("[data-object-id]")?.dataset.objectId;
  const hitControl = target.closest("button, input, textarea, select, a");
  if (event.button !== 0 || !objectId || hitControl) return false;
  const reference = active.completion.references.find(
    (candidate) =>
      candidate.id === objectId &&
      (candidate.kind === "value" || candidate.kind === "frame")
  );
  if (!reference) return false;
  event.preventDefault();
  event.stopPropagation();
  options.insertReference(reference.token, !editingFromBar);
  options.onNotice(null);
  return true;
}

/** Routes canvas pointer gestures into the one active formula draft. */
export function canvasFormulaPointerHandler(options: PickingOptions) {
  return (event: ReactPointerEvent) => {
    const target = event.target as HTMLElement;
    const active = options.getActive();
    const focused = window.document.activeElement;
    const fromBar =
      focused instanceof HTMLElement &&
      Boolean(focused.closest(".scratchwork-formula-bar"));
    if (active && (active.focused || fromBar)) {
      if (trySummaryPick(event, target, active, fromBar, options)) return;
      if (tryScratchworkRangePick(event, target, active, fromBar, options)) return;
      if (tryColumnPick(event, target, active, fromBar, options)) return;
      if (tryObjectPick(event, target, active, fromBar, options)) return;
    }
    if (target.closest(".formula-editor, .block-source, .scratchwork-formula-bar"))
      return;
    options.disengage();
  };
}
