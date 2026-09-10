import { useCallback } from "react";
import { importDatabaseSource, setFrameSource, type DatabaseSourceInput } from "../lib/api";
import type { DocumentView } from "../lib/types";

export function useFrameSourceReplacement(
  setDocument: (document: DocumentView) => void,
  setDataRefreshRevision: (update: (revision: number) => number) => void,
  setError: (message: string | null) => void,
) {
  return useCallback(async (frameId: string, database?: DatabaseSourceInput) => {
    try {
      const changed = database
        ? await importDatabaseSource({ x: 0, y: 0 }, database, frameId)
        : await setFrameSource(frameId);
      if (changed) {
        setDocument(changed);
        setDataRefreshRevision((revision) => revision + 1);
      }
      setError(null);
      return null;
    } catch (reason) {
      return String(reason).replace(/^Error:\s*/, "");
    }
  }, [setDocument, setDataRefreshRevision, setError]);
}
