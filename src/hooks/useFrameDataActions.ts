import { useCallback } from "react";
import { adoptFrameRows } from "../lib/api";
import { updateOriginalDelimited, exportFrameAs } from "../lib/fileWriteback";
import type { DocumentView } from "../lib/types";

export function useFrameDataActions({
  setDocument,
  setDataRefreshRevision,
  setError,
  setNotice,
}: {
  setDocument: (value: DocumentView) => void;
  setDataRefreshRevision: (updater: (revision: number) => number) => void;
  setError: (value: string | null) => void;
  setNotice: (value: string | null) => void;
}) {
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

  const exportFrameFile = useCallback(
    async (frameId: string) => {
      try {
        const updated = await exportFrameAs(frameId);
        if (updated) {
          setDocument(updated);
          setDataRefreshRevision((revision) => revision + 1);
          setNotice("Exported and added as a new table. Your transformations remain on the original table.");
        }
        setError(null);
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
      }
    },
    [setDocument, setDataRefreshRevision, setError, setNotice]
  );

  const updateOriginalFile = useCallback(
    async (frameId: string) => {
      try {
        const updated = await updateOriginalDelimited(frameId);
        if (updated) {
          setDocument(updated);
          setDataRefreshRevision((revision) => revision + 1);
          setNotice("Original file updated and added as a direct read. Your transformation chain is still in this workbook.");
        }
        setError(null);
      } catch (reason) {
        setError(String(reason).replace(/^Error:\s*/, ""));
      }
    },
    [setDocument, setDataRefreshRevision, setError, setNotice]
  );

  return { takeOwnership, updateOriginalFile, exportFrameFile };
}
