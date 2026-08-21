import {
  useCallback,
  useEffect,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from "react";
import type { useActiveFormulaEditorCommands } from "../ActiveFormulaEditor";
import { applyOperation, getDocument } from "../lib/api";
import type { FormulaReference } from "../lib/formulaReferences";
import { appendScratchworkLine, sameName, SCRATCHWORK } from "../lib/scratchwork";
import type { BlockObject, DocumentView, Operation, Selection } from "../lib/types";
import { scalarFormulaReferences } from "../ScalarCards";
import type { ScratchworkFormulaFeedback } from "../ScratchworkFormulaBar";

type ScratchFocus = { blockId: string | null; token: number } | null;
type ScratchReturn = { left: number; top: number; selection: Selection | null } | null;

/**
 * The one Scratchwork block ⌘J drops you into, and the top formula bar's
 * write path to it. Both write the same block by the same name match, so
 * they live together: a fix to how the block is found should not have two
 * places to remember.
 */
export function useScratchwork({
  document,
  setDocument,
  scratchFocus,
  setScratchFocus,
  scratchworkDrawerOpen,
  setScratchworkDrawerOpen,
  scratchReturn,
  canvasRef,
  selection,
  setSelection,
  run,
  insertPosition,
  jumpToObject,
  getActiveFormulaEditor,
  commitActiveFormulaEditor,
}: {
  document: DocumentView | null;
  setDocument: Dispatch<SetStateAction<DocumentView | null>>;
  scratchFocus: ScratchFocus;
  setScratchFocus: Dispatch<SetStateAction<ScratchFocus>>;
  scratchworkDrawerOpen: boolean;
  setScratchworkDrawerOpen: Dispatch<SetStateAction<boolean>>;
  scratchReturn: RefObject<ScratchReturn>;
  canvasRef: RefObject<HTMLDivElement | null>;
  selection: Selection | null;
  setSelection: (value: Selection | null) => void;
  run: (
    operation: Operation,
    options?: { inlineError?: boolean }
  ) => Promise<string | null>;
  insertPosition: () => { x: number; y: number };
  jumpToObject: (objectId: string) => void;
  getActiveFormulaEditor: ReturnType<typeof useActiveFormulaEditorCommands>["getActive"];
  commitActiveFormulaEditor: ReturnType<typeof useActiveFormulaEditorCommands>["commit"];
}) {
  const scratchTargetId = document
    ? (scratchFocus
        ? (scratchFocus.blockId ??
          document.objects.filter((object) => object.kind === "block").at(-1)?.id ??
          null)
        : null)
    : null;
  const scratchworkBlock = document?.objects.find(
    (object): object is BlockObject =>
      object.kind === "block" && sameName(object.name, SCRATCHWORK)
  );
  const scratchworkBarReferences: FormulaReference[] = document
    ? [
        ...(scratchworkBlock
          ? (document.computedBlocks[scratchworkBlock.id]?.lines ?? []).flatMap((line) =>
              line.name
                ? [
                    {
                      id: line.id,
                      label: line.name,
                      token: line.name,
                      kind: "value" as const,
                      detail: "Scratchwork line",
                    },
                  ]
                : []
            )
          : []),
        ...scalarFormulaReferences(
          document.objects,
          document.formulaFunctions,
          document.computedFrames,
          scratchworkBlock?.id
        ),
      ]
    : [];

  const summonScratchpad = useCallback(async () => {
    if (!document) return;
    // The drawer and the card are the same editor in two places, never two
    // drafts. Finish the drawer's current text before moving that editor back
    // onto the canvas, then let the ordinary ⌘J path reveal and focus it.
    if (scratchworkDrawerOpen) {
      if (getActiveFormulaEditor()?.kind === "scratchwork")
        await commitActiveFormulaEditor();
      setScratchworkDrawerOpen(false);
    } else {
      // Already in the scratchpad: this is the way back. A key that only ever
      // went one way would make the scratchpad a place you have to climb out
      // of, when the whole point is that it is somewhere you drop into for a
      // moment while looking at something else.
      const active = window.document.activeElement;
      if (
        active instanceof HTMLElement &&
        active.classList.contains("block-source")
      ) {
        active.blur();
        const back = scratchReturn.current;
        scratchReturn.current = null;
        if (back) {
          setSelection(back.selection);
          canvasRef.current?.scrollTo({ ...back, behavior: "smooth" });
        }
        return;
      }
    }
    scratchReturn.current = {
      left: canvasRef.current?.scrollLeft ?? 0,
      top: canvasRef.current?.scrollTop ?? 0,
      selection,
    };
    const target = document.objects.find(
      (object): object is BlockObject =>
        object.kind === "block" && sameName(object.name, SCRATCHWORK)
    );
    const ask = (blockId: string | null) =>
      setScratchFocus((previous) => ({
        blockId,
        token: (previous?.token ?? 0) + 1,
      }));
    if (!target) {
      const failed = await run({
        type: "addBlock",
        name: SCRATCHWORK,
        ...insertPosition(),
      });
      // The block arrives holding `line_1`, which is the line being asked
      // for. It is the newest block and has no id here yet, hence the null.
      if (!failed) ask(null);
      return;
    }
    // A folded card would swallow the line the key just asked for, so being
    // summoned unfolds it. Putting something away is not the same as saying
    // never open it again.
    const view = document.views.find((candidate) => candidate.objectId === target.id);
    if (view?.collapsed)
      await run({ type: "setViewCollapsed", viewId: view.id, collapsed: false });
    jumpToObject(target.id);
    // The cursor lands at the end, on a line of its own — which is where
    // somebody pressing this is about to write. A scratchpad that already
    // ends in a blank line needs no help.
    const source = document.computedBlocks[target.id]?.source ?? "";
    if (source.trim() !== "" && !source.endsWith("\n")) {
      const failed = await run({
        type: "setBlockSource",
        blockId: target.id,
        source: `${source}\n`,
      });
      if (failed) return;
    }
    ask(target.id);
  }, [
    commitActiveFormulaEditor,
    document,
    getActiveFormulaEditor,
    insertPosition,
    jumpToObject,
    run,
    scratchReturn,
    scratchworkDrawerOpen,
    selection,
    setScratchFocus,
    setScratchworkDrawerOpen,
    setSelection,
    canvasRef,
  ]);

  /**
   * With no editor active, the top bar writes an ordinary final line into the
   * ordinary Scratchwork block. It owns no result tier of its own: the little
   * answer beside the input is read back from the block after the operation,
   * and the block is the durable history from the first Enter onward.
   */
  const appendScratchworkFromBar = useCallback(
    async (formula: string): Promise<ScratchworkFormulaFeedback> => {
      if (!document)
        return { saved: false, error: "The document is still opening." };
      try {
        let next = await getDocument();
        let block = next.objects.find(
          (object): object is BlockObject =>
            object.kind === "block" && sameName(object.name, SCRATCHWORK)
        );
        if (!block) {
          next = await applyOperation({
            type: "addBlock",
            name: SCRATCHWORK,
            ...insertPosition(),
          });
          setDocument(next);
          block = next.objects.find(
            (object): object is BlockObject =>
              object.kind === "block" && sameName(object.name, SCRATCHWORK)
          );
        }
        if (!block)
          return { saved: false, error: "Scratchwork could not be created." };

        const appended = appendScratchworkLine(
          next.computedBlocks[block.id]?.source ?? "",
          formula
        );
        next = await applyOperation({
          type: "setBlockSource",
          blockId: block.id,
          source: appended.source,
          editing: null,
        });
        setDocument(next);
        const line = next.computedBlocks[block.id]?.lines[appended.lineIndex];
        if (!line)
          return {
            saved: true,
            error: "Saved in Scratchwork; its answer is not available yet.",
          };
        return {
          saved: true,
          name: line.name || undefined,
          display: line.error ? undefined : line.display,
          error: line.error ?? undefined,
        };
      } catch (reason) {
        return {
          saved: false,
          error: String(reason).replace(/^Error:\s*/, ""),
        };
      }
    },
    [document, insertPosition, setDocument]
  );

  /** Move the one Scratchwork editor between its canvas card and the top. */
  const toggleScratchworkDrawer = useCallback(async () => {
    if (!document) return;
    if (getActiveFormulaEditor()?.kind === "scratchwork")
      await commitActiveFormulaEditor();
    if (scratchworkDrawerOpen) {
      setScratchworkDrawerOpen(false);
      return;
    }
    const exists = document.objects.some(
      (object) => object.kind === "block" && sameName(object.name, SCRATCHWORK)
    );
    if (!exists) {
      const failed = await run({
        type: "addBlock",
        name: SCRATCHWORK,
        ...insertPosition(),
      });
      if (failed) return;
    }
    setScratchworkDrawerOpen(true);
  }, [
    commitActiveFormulaEditor,
    document,
    getActiveFormulaEditor,
    insertPosition,
    run,
    scratchworkDrawerOpen,
    setScratchworkDrawerOpen,
  ]);

  // If the block the drawer is showing gets deleted out from under it (an
  // undo, a remote edit), the drawer has nothing left to edit and closes
  // rather than show a stale or empty editor.
  useEffect(() => {
    if (!scratchworkDrawerOpen || !document) return;
    const stillExists = document.objects.some(
      (object) => object.kind === "block" && sameName(object.name, SCRATCHWORK)
    );
    if (!stillExists) setScratchworkDrawerOpen(false);
  }, [document, scratchworkDrawerOpen, setScratchworkDrawerOpen]);

  return {
    scratchTargetId,
    scratchworkBlock,
    scratchworkBarReferences,
    summonScratchpad,
    appendScratchworkFromBar,
    toggleScratchworkDrawer,
  };
}
