import { useCallback, useState, type Dispatch, type SetStateAction } from "react";
import type { ContextMenuState, GridFocus } from "../FrameGrid";
import { importAndAppendDatasetFile, importDatasetFile } from "../lib/api";
import {
  DEFAULT_IMPORT_MODE,
  parseAskOnImport,
  parseImportMode,
  type ImportMode,
} from "../lib/preferences";
import type { DocumentView, Selection } from "../lib/types";

const IMPORT_MODE_PREFERENCE = "framework.importMode";
const ASK_ON_IMPORT_PREFERENCE = "framework.askOnImport";

function readImportMode(): ImportMode {
  try {
    return parseImportMode(window.localStorage.getItem(IMPORT_MODE_PREFERENCE));
  } catch {
    return DEFAULT_IMPORT_MODE;
  }
}

function readAskOnImport(): boolean {
  try {
    return parseAskOnImport(window.localStorage.getItem(ASK_ON_IMPORT_PREFERENCE));
  } catch {
    return true;
  }
}

/**
 * The import mode/ask-first preferences (persisted, like the other
 * preference hooks), and the two ways a file actually lands in the
 * document: as a new frame, or appended beneath one a menu named.
 */
export function useImportFlow({
  setDocument,
  setSelection,
  setContextMenu,
  setError,
  setInspectorSection,
  setGridFocus,
  setDatasetLibrary,
}: {
  setDocument: Dispatch<SetStateAction<DocumentView | null>>;
  setSelection: (value: Selection | null) => void;
  setContextMenu: Dispatch<SetStateAction<ContextMenuState | null>>;
  setError: (value: string | null) => void;
  setInspectorSection: (value: "wrangle") => void;
  setGridFocus: (value: GridFocus | null) => void;
  setDatasetLibrary: (value: boolean) => void;
}) {
  const [importMode, setImportModeState] = useState<ImportMode>(readImportMode);
  const [askOnImport, setAskOnImportState] = useState(readAskOnImport);

  const setImportMode = useCallback((mode: ImportMode) => {
    setImportModeState(mode);
    try {
      window.localStorage.setItem(IMPORT_MODE_PREFERENCE, mode);
    } catch {
      // The choice still applies to this import; it just does not carry.
    }
  }, []);

  const setAskOnImport = useCallback((ask: boolean) => {
    setAskOnImportState(ask);
    try {
      window.localStorage.setItem(ASK_ON_IMPORT_PREFERENCE, String(ask));
    } catch {
      // As above.
    }
  }, []);

  const runImport = useCallback(
    async (position: { x: number; y: number }, mode: ImportMode) => {
      try {
        const imported = await importDatasetFile(position, mode === "linked");
        // A cancelled file picker is not a failure and imports nothing.
        if (!imported) return false;
        setDocument(imported);
        setSelection(null);
        setContextMenu(null);
        setError(null);
        return true;
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
        return false;
      }
    },
    [setContextMenu, setDocument, setError, setSelection]
  );

  // Appending is deliberately an import of a second source plus a derived
  // frame, rather than a rewrite of the frame somebody right-clicked. The
  // original remains a stable input, the new file remains inspectable, and
  // the only connection between them is the ordinary, editable Stack step.
  const runAppendImport = useCallback(
    async (target: { frameId: string; x: number; y: number }, mode: ImportMode) => {
      try {
        const appended = await importAndAppendDatasetFile(
          target.frameId,
          { x: target.x, y: target.y },
          mode === "linked"
        );
        if (!appended) return false;
        setDocument(appended.document);
        setSelection({ objectId: appended.appendedFrameId });
        setInspectorSection("wrangle");
        setGridFocus(null);
        setContextMenu(null);
        setError(null);
        return true;
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
        return false;
      }
    },
    [setContextMenu, setDocument, setError, setGridFocus, setInspectorSection, setSelection]
  );

  const handleOpenDocument = useCallback(async () => {
    setDatasetLibrary(true);
  }, [setDatasetLibrary]);

  return {
    importMode,
    setImportMode,
    askOnImport,
    setAskOnImport,
    runImport,
    runAppendImport,
    handleOpenDocument,
  };
}
