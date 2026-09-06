import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  useActiveFormulaEditorCommands,
  useActiveFormulaEditorWatcher,
} from "./ActiveFormulaEditor";
import { BlockCard } from "./BlockCard";
import { NumberDisplayContext } from "./FrameGrid";
import { useModifierHints } from "./hooks/useModifierHints";
import { useThousandsSeparatorsPreference } from "./hooks/useThousandsSeparatorsPreference";
import { useScratchworkWindowMenu } from "./hooks/useScratchworkWindowMenu";
import {
  applyOperation,
  freezeValue,
  getDocument,
  publishScratchworkEditorState,
  redo,
  type ScratchworkWindowEdit,
  undo,
} from "./lib/api";
import { reportIgnoredFailure } from "./lib/errorReporting";
import { sameName, SCRATCHWORK } from "./lib/scratchwork";
import type { BlockObject, DocumentView, Operation } from "./lib/types";
import "./ScratchworkWindow.css";

const DOCUMENT_CHANGED_EVENT = "framework-document-changed";
const EDIT_EVENT = "framework-scratchwork-edit";
const CLEAR_EDITOR_EVENT = "framework-scratchwork-clear-editor";

/** The dense Scratchwork editor, with no canvas chrome around it. */
export default function ScratchworkWindow() {
  const [document, setDocument] = useState<DocumentView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [focusToken, setFocusToken] = useState(1);
  const activeCommands = useActiveFormulaEditorCommands();
  const [useThousandsSeparators] = useThousandsSeparatorsPreference();
  useModifierHints();
  const revision = useRef(0);

  useEffect(() => {
    let disposed = false;
    const stops: Array<() => void> = [];
    void Promise.all([
      getDocument().then((next) => {
        if (!disposed) setDocument(next);
      }),
      getCurrentWebviewWindow().listen<DocumentView>(
        DOCUMENT_CHANGED_EVENT,
        (event) => {
          setDocument(event.payload);
          setError(null);
        }
      ),
      getCurrentWebviewWindow().listen<ScratchworkWindowEdit>(EDIT_EVENT, (event) => {
        if (event.payload.replaceSelection)
          activeCommands.replaceSelection(event.payload.text);
        else
          activeCommands.insertReference(event.payload.text, event.payload.refocus);
      }),
      getCurrentWebviewWindow().listen(CLEAR_EDITOR_EVENT, () => activeCommands.clear()),
    ])
      .then((results) => {
        const unlisten = results.filter(
          (result): result is () => void => typeof result === "function"
        );
        if (disposed) unlisten.forEach((stop) => stop());
        else stops.push(...unlisten);
      })
      .catch((reason) => {
        if (!disposed) setError(String(reason).replace(/^Error:\s*/, ""));
      });
    return () => {
      disposed = true;
      stops.forEach((stop) => stop());
    };
  }, [activeCommands]);

  // Watched, not rendered: this window's only interest in the draft is
  // handing it to the workbook, and rendering the whole window per keystroke
  // re-rendered the editor being watched, which rebound itself and published
  // again.
  useActiveFormulaEditorWatcher((active) => {
    revision.current += 1;
    void publishScratchworkEditorState({
      revision: revision.current,
      active: Boolean(active),
      draft: active?.draft ?? "",
      selectionStart: active?.selection.start ?? 0,
      selectionEnd: active?.selection.end ?? 0,
    }).catch(reportIgnoredFailure("publish Scratchwork editor state"));
  });

  const run = useCallback(
    async (operation: Operation, options?: { inlineError?: boolean }) => {
      try {
        setDocument(await applyOperation(operation));
        setError(null);
        return null;
      } catch (reason) {
        const message = String(reason).replace(/^Error:\s*/, "");
        if (!options?.inlineError) setError(message);
        return message;
      }
    },
    []
  );
  const navigateHistory = useCallback(async (direction: "undo" | "redo") => {
    try {
      setDocument(await (direction === "undo" ? undo() : redo()));
      setError(null);
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, []);
  const requestFocus = useCallback(
    () => setFocusToken((token) => token + 1),
    []
  );
  useScratchworkWindowMenu({
    activeCommands,
    navigateHistory,
    requestFocus,
    setError,
  });

  const block = document?.objects.find(
    (object): object is BlockObject =>
      object.kind === "block" && sameName(object.name, SCRATCHWORK)
  );

  if (!document) return <main className="scratchwork-window" />;
  return (
    <NumberDisplayContext.Provider value={useThousandsSeparators}>
      <main className="scratchwork-window" aria-label="Scratchwork window">
        {block ? (
          <BlockCard
            block={block}
            computed={document.computedBlocks[block.id]}
            focusToken={focusToken}
            objects={document.objects}
            computedFrames={document.computedFrames}
            formulaFunctions={document.formulaFunctions}
            onOperation={run}
            onFreeze={async (objectId) => {
              setDocument(await freezeValue(objectId));
            }}
          />
        ) : (
          <p>Scratchwork is not available in this workbook.</p>
        )}
        {error && <p className="scratchwork-window-error">{error}</p>}
      </main>
    </NumberDisplayContext.Provider>
  );
}
