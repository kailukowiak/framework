import { CARD_SIZES, type CardSize } from "../lib/cardPlacement";
import type { DataObject, DocumentView, FrameObject, Operation } from "../lib/types";

/**
 * The six "make me a new card" commands the canvas offers, in one place.
 *
 * They are the same shape as each other — name the object, place it, run the
 * operation — so they read as a set rather than as six unrelated handlers
 * scattered through `App`. Every one takes an optional position because the
 * context menu drops a card where the pointer was and the menu bar does not.
 */
export function useCanvasObjectCreation({
  run,
  document,
  insertPosition,
}: {
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  document: DocumentView | null;
  insertPosition: (size?: CardSize) => { x: number; y: number };
}) {
  /**
   * A new formula block, named `Block 1`, `Block 2`, … until somebody says
   * otherwise.
   *
   * A block is the dense home for worked calculations that belong together.
   * A compact variable below is the one-object cousin for a single assumption
   * that should sit by itself on the canvas; its formula may format to lines.
   */
  const addBlock = (position?: { x: number; y: number }) =>
    run({
      type: "addBlock",
      name: nextObjectName(document?.objects ?? [], "Block"),
      ...(position ?? insertPosition(CARD_SIZES.block)),
    });

  const addVariable = (position?: { x: number; y: number }) =>
    run({
      type: "addVariable",
      name: nextObjectName(document?.objects ?? [], "x"),
      formula: "0",
      ...(position ?? insertPosition(CARD_SIZES.variable)),
    });

  const addCalculationMatrix = (position?: { x: number; y: number }) =>
    run({
      type: "addCalculationMatrix",
      name: nextObjectName(document?.objects ?? [], "Calculation Matrix"),
      ...(position ?? insertPosition(CARD_SIZES.calculationMatrix)),
    });

  /** A card of prose: markdown, with `{{…}}` holes that print live values. */
  const addText = (position?: { x: number; y: number }) =>
    run({
      type: "addText",
      ...(position ?? insertPosition(CARD_SIZES.text)),
    });

  /**
   * A new frame is an empty 2×2 you can type or paste into, not a dialog
   * asking for data up front.
   *
   * The dialog was a text box for pasting, which put a modal between the
   * user and the thing they wanted; the grid is already a better text box.
   * Pasting into it replaces it outright — see `handleGridPaste` — so the
   * old flow survives as "make one, then paste".
   */
  const addEmptyFrame = (position?: { x: number; y: number }) =>
    run({
      type: "addFrame",
      name: "Frame 1",
      grid: [
        ["Column 1", "Column 2"],
        ["", ""],
        ["", ""],
      ],
      ...(position ?? insertPosition(CARD_SIZES.frame)),
    });

  const addContainer = (position?: { x: number; y: number }) =>
    run({
      type: "addContainer",
      name: nextContainerName(document?.objects ?? []),
      ...(position ?? insertPosition(CARD_SIZES.container)),
    });

  return {
    addBlock,
    addVariable,
    addCalculationMatrix,
    addText,
    addEmptyFrame,
    addContainer,
  };
}

export function nextObjectName(objects: DataObject[], stem: string): string {
  const taken = new Set(objects.map((object) => object.name));
  if (!taken.has(stem)) return stem;
  let suffix = 2;
  while (taken.has(`${stem} ${suffix}`)) suffix += 1;
  return `${stem} ${suffix}`;
}

/** A container name nothing has taken yet. */
export function nextContainerName(objects: DataObject[]): string {
  const taken = new Set(objects.map((object) => object.name));
  if (!taken.has("Container")) return "Container";
  let suffix = 2;
  while (taken.has(`Container ${suffix}`)) suffix += 1;
  return `Container ${suffix}`;
}

/**
 * The next free name for an entry column: names are formula addresses, so
 * a second "Entry" must not shadow the first.
 */
export function nextEntryColumnName(frame: FrameObject): string {
  const names = new Set(frame.columns.map((column) => column.name));
  if (!names.has("Entry")) return "Entry";
  let index = 2;
  while (names.has(`Entry ${index}`)) index += 1;
  return `Entry ${index}`;
}
