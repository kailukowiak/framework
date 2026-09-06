import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ActiveFormulaEditor } from "../lib/activeFormulaEditor";
import {
  clearScratchworkWindowEditor,
  editScratchworkWindow,
  type ScratchworkEditorState,
} from "../lib/api";
import { reportIgnoredFailure } from "../lib/errorReporting";
import { insertFormulaReference, type FormulaReference } from "../lib/formulaReferences";
import type { BlockObject } from "../lib/types";

type LocalFormulaCommands = {
  getActive: () => ActiveFormulaEditor | null;
  insertReference: (token: string, refocus?: boolean) => void;
  replaceSelection: (text: string) => void;
  cancel: () => void;
  clear: () => void;
  disengage: () => void;
};

const EDITOR_STATE_EVENT = "framework-scratchwork-editor-state";
const WINDOW_CLOSED_EVENT = "framework-scratchwork-window-closed";

function optimisticEdit(
  current: ScratchworkEditorState,
  text: string,
  replaceSelection: boolean
): ScratchworkEditorState {
  const selection = {
    start: current.selectionStart,
    end: current.selectionEnd,
  };
  const inserted = replaceSelection || selection.start !== selection.end
    ? {
        source: `${current.draft.slice(0, selection.start)}${text}${current.draft.slice(
          selection.end
        )}`,
        cursor: selection.start + text.length,
      }
    : insertFormulaReference(current.draft, selection.end, text);
  return {
    ...current,
    draft: inserted.source,
    selectionStart: inserted.cursor,
    selectionEnd: inserted.cursor,
  };
}

/**
 * Carries the one logical Scratchwork cursor across two webviews.
 *
 * A native window necessarily has a separate DOM and React tree, so the
 * in-process editor registry cannot cross that boundary. Only its small,
 * serializable fact — draft plus selection — crosses. Canvas clicks still
 * use the ordinary formula-picking code and send the resulting token back
 * to the real editor; the workbook never owns a second draft.
 */
export function useScratchworkWindowBridge({
  local,
  localFocused,
  block,
  references,
}: {
  local: LocalFormulaCommands;
  localFocused: boolean;
  block: BlockObject | undefined;
  references: FormulaReference[];
}) {
  const external = useRef<ScratchworkEditorState | null>(null);
  const blockRef = useRef(block);
  const referencesRef = useRef(references);
  blockRef.current = block;
  referencesRef.current = references;
  const [externalEngaged, setExternalEngaged] = useState(false);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false;
    const stops: Array<() => void> = [];
    void Promise.all([
      getCurrentWebviewWindow().listen<ScratchworkEditorState>(
        EDITOR_STATE_EVENT,
        (event) => {
          const next = event.payload;
          if (!next.active) {
            external.current = null;
            setExternalEngaged(false);
            return;
          }
          if ((external.current?.revision ?? -1) <= next.revision) {
            external.current = next;
            setExternalEngaged(true);
          }
        }
      ),
      getCurrentWebviewWindow().listen(WINDOW_CLOSED_EVENT, () => {
        external.current = null;
        setExternalEngaged(false);
      }),
    ])
      .then((unlisten) => {
        if (disposed) unlisten.forEach((stop) => stop());
        else stops.push(...unlisten);
      })
      .catch(reportIgnoredFailure("Scratchwork window bridge"));
    return () => {
      disposed = true;
      stops.forEach((stop) => stop());
    };
  }, []);

  const externalActive = useCallback((): ActiveFormulaEditor | null => {
    const editor = external.current;
    const owner = blockRef.current;
    if (!editor?.active || !owner || !externalEngaged) return null;
    return {
      id: `scratchwork:${owner.id}`,
      label: owner.name,
      kind: "scratchwork",
      draft: editor.draft,
      selection: {
        start: editor.selectionStart,
        end: editor.selectionEnd,
      },
      focused: true,
      canCommit: true,
      completion: { references: referencesRef.current },
    };
  }, [externalEngaged]);

  // The editor somebody is typing in wins. A local session outlives its
  // focus on purpose (see ActiveFormulaEditorRegistry), so the canvas block
  // that was edited a minute ago is still "active" here while the pop-out
  // holds the caret -- and the canvas pointer handler only picks for a
  // *focused* editor. Consulting the local session first therefore handed
  // every click to a blurred editor that could not take it, and the pop-out
  // never saw the reference.
  const getActive = useCallback(() => {
    const own = local.getActive();
    if (own?.focused) return own;
    return externalActive() ?? own;
  }, [externalActive, local]);
  const externalTakesInput = useCallback(
    () => !local.getActive()?.focused && Boolean(externalActive()),
    [externalActive, local]
  );

  const updateOptimistically = useCallback((text: string, replaceSelection: boolean) => {
    const current = external.current;
    if (!current) return;
    external.current = optimisticEdit(current, text, replaceSelection);
  }, []);

  const editExternal = useCallback(
    (text: string, replaceSelection: boolean, refocus: boolean) => {
      updateOptimistically(text, replaceSelection);
      void editScratchworkWindow({ text, replaceSelection, refocus }).catch(
        reportIgnoredFailure("Scratchwork window edit")
      );
    },
    [updateOptimistically]
  );

  const insertReference = useCallback(
    (token: string) => {
      // Keep the canvas in front while pointing. The pop-out still receives
      // the caret update; raising it after every cell would make a range of
      // references impossible to author naturally.
      if (externalTakesInput()) editExternal(token, false, false);
      else if (local.getActive()) local.insertReference(token);
    },
    [editExternal, externalTakesInput, local]
  );
  const replaceSelection = useCallback(
    (text: string) => {
      if (externalTakesInput()) editExternal(text, true, true);
      else if (local.getActive()) local.replaceSelection(text);
    },
    [editExternal, externalTakesInput, local]
  );
  const clear = useCallback(() => {
    if (local.getActive()) local.clear();
    if (external.current) {
      external.current = null;
      setExternalEngaged(false);
      void clearScratchworkWindowEditor().catch(
        reportIgnoredFailure("clear Scratchwork window editor")
      );
    }
  }, [local]);
  const cancel = useCallback(() => {
    if (local.getActive()) local.cancel();
    if (external.current) {
      external.current = null;
      setExternalEngaged(false);
      void clearScratchworkWindowEditor().catch(
        reportIgnoredFailure("clear Scratchwork window editor")
      );
    }
  }, [local]);
  const disengage = useCallback(() => {
    if (local.getActive()) local.disengage();
    else setExternalEngaged(false);
  }, [local]);

  return {
    active: localFocused || externalEngaged,
    getActive,
    insertReference,
    replaceSelection,
    cancel,
    clear,
    disengage,
  };
}
