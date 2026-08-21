import { useCallback, useState } from "react";
import {
  adoptFrameRows,
  clearFrameMaterialization,
  compactDocumentData,
  freezeFrameCopy,
  freezeValue,
  materializeFrame,
  packageDocument,
  redo,
  refreshFrameConnector,
  refreshStaleSnapshots,
  setFrameSource,
  undo,
} from "../lib/api";
import { formatBytes } from "../lib/formatBytes";
import { reconcileSelection } from "../lib/reconcileSelection";
import type { DocumentView, Selection } from "../lib/types";

/**
 * The document-level operations that read back a whole document and update
 * it, error, or a one-off notice: caching a frame to a snapshot or clearing
 * it, refreshing a connector or every stale snapshot at once, taking
 * ownership of a frame's rows, packaging or compacting the document's data,
 * and undo/redo. Each is its own gesture with its own confirmation, but they
 * share the same three-line shape closely enough that separating them by
 * gesture would just repeat it ten times.
 */
export function useDocumentLifecycle({
  setDocument,
  setError,
  setNotice,
  setSelection,
  setContextMenu,
  setDataRefreshRevision,
}: {
  setDocument: (value: DocumentView) => void;
  setError: (value: string | null) => void;
  setNotice: (value: string | null) => void;
  setSelection: (updater: (current: Selection | null) => Selection | null) => void;
  setContextMenu: (value: null) => void;
  setDataRefreshRevision: (updater: (revision: number) => number) => void;
}) {
  /// Writing a value's answer down, or refreshing the one written. Live data
  /// is read here and only here — everywhere else a value that reads a frame
  /// with no snapshot asks to be frozen first.
  const freeze = useCallback(
    async (objectId: string) => {
      try {
        setDocument(await freezeValue(objectId));
        setError(null);
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
      }
    },
    [setDocument, setError]
  );

  const refreshConnector = useCallback(
    async (frameId: string, options?: { inlineError?: boolean }) => {
      try {
        setDocument(await refreshFrameConnector(frameId));
        setDataRefreshRevision((revision) => revision + 1);
        setError(null);
        return null;
      } catch (reason) {
        const message = String(reason).replace(/^Error:\s*/, "");
        if (!options?.inlineError) setError(message);
        return message;
      }
    },
    [setDataRefreshRevision, setDocument, setError]
  );

  // Cancelling the picker is not a failure and leaves the frame alone; the
  // command returns no document, so there is nothing to apply.
  const changeFrameSource = useCallback(
    async (frameId: string) => {
      try {
        const changed = await setFrameSource(frameId);
        if (changed) {
          setDocument(changed);
          setDataRefreshRevision((revision) => revision + 1);
        }
        setError(null);
        return null;
      } catch (reason) {
        return String(reason).replace(/^Error:\s*/, "");
      }
    },
    [setDataRefreshRevision, setDocument, setError]
  );

  // Refreshing every stale snapshot at once. Partial success is the normal
  // outcome worth reporting: one frame failing to compute leaves the frames
  // below it stale on purpose, and saying "3 refreshed" while two are
  // untouched would be the same lie a stale snapshot tells.
  const [refreshingSnapshots, setRefreshingSnapshots] = useState(false);
  const refreshStale = useCallback(async () => {
    setRefreshingSnapshots(true);
    try {
      const result = await refreshStaleSnapshots();
      setDocument(result.document);
      setDataRefreshRevision((revision) => revision + 1);
      setError(
        result.failures.length
          ? `Could not refresh ${result.failures
              .map((failure) => failure.frame)
              .join(", ")}: ${result.failures[0].error}`
          : null
      );
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    } finally {
      setRefreshingSnapshots(false);
    }
  }, [setDataRefreshRevision, setDocument, setError]);

  // Taking ownership rewrites what a frame is, so every page already
  // fetched is now read from somewhere else.
  const takeOwnership = useCallback(
    async (frameId: string, options?: { inlineError?: boolean }) => {
      try {
        setDocument(await adoptFrameRows(frameId));
        setDataRefreshRevision((revision) => revision + 1);
        setError(null);
        return null;
      } catch (reason) {
        const message = String(reason).replace(/^Error:\s*/, "");
        if (!options?.inlineError) setError(message);
        return message;
      }
    },
    [setDataRefreshRevision, setDocument, setError]
  );

  const packageThisDocument = useCallback(async () => {
    try {
      setDocument(await packageDocument());
      setDataRefreshRevision((revision) => revision + 1);
      setError(null);
      setNotice("Packaged: nothing in this document reads a file outside it.");
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, [setDataRefreshRevision, setDocument, setError, setNotice]);

  const compactData = useCallback(async () => {
    try {
      const swept = await compactDocumentData();
      setError(null);
      setNotice(
        swept.files === 0
          ? "Nothing to reclaim — every data file here is still in use."
          : `Reclaimed ${formatBytes(swept.bytes)} from ${swept.files} data ${
              swept.files === 1 ? "file" : "files"
            }.`
      );
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
    }
  }, [setError, setNotice]);

  const freezeCopy = useCallback(
    async (frameId: string, position: { x: number; y: number }) => {
      try {
        setDocument(await freezeFrameCopy(frameId, position.x, position.y));
        setError(null);
        return null;
      } catch (reason) {
        return String(reason).replace(/^Error:\s*/, "");
      }
    },
    [setDocument, setError]
  );

  // Caching a frame to a snapshot, refreshing that snapshot, and dropping it
  // are all the same shape: the document comes back and any page already
  // fetched is now read from somewhere else, so the paged reads regenerate.
  const setFrameCached = useCallback(
    async (frameId: string, cached: boolean, options?: { inlineError?: boolean }) => {
      try {
        setDocument(
          cached
            ? await materializeFrame(frameId)
            : await clearFrameMaterialization(frameId)
        );
        setDataRefreshRevision((revision) => revision + 1);
        setError(null);
        return null;
      } catch (reason) {
        const message = String(reason).replace(/^Error:\s*/, "");
        if (!options?.inlineError) setError(message);
        return message;
      }
    },
    [setDataRefreshRevision, setDocument, setError]
  );

  const navigateHistory = useCallback(
    async (direction: "undo" | "redo") => {
      try {
        const next = direction === "undo" ? await undo() : await redo();
        setDocument(next);
        setSelection((current) => (current ? reconcileSelection(next, current) : null));
        setContextMenu(null);
        setError(null);
      } catch (reason) {
        setError(String(reason));
      }
    },
    [setContextMenu, setDocument, setError, setSelection]
  );

  return {
    freeze,
    refreshConnector,
    changeFrameSource,
    refreshingSnapshots,
    refreshStale,
    takeOwnership,
    packageThisDocument,
    compactData,
    freezeCopy,
    setFrameCached,
    navigateHistory,
  };
}
