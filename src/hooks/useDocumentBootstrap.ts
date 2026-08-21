import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getDocument,
  getDocumentPath,
  shouldOpenLibrary,
} from "../lib/api";
import type { ContextMenuState } from "../FrameGrid";
import type { DocumentView, Selection } from "../lib/types";

/**
 * Loads the document this window opened with, and — inside Tauri — stays
 * subscribed for as long as the window lives: another window saving this
 * same file, a document opened from the Finder or a second-instance launch,
 * or the collaboration link failing all arrive as events rather than
 * anything this window asked for.
 */
export function useDocumentBootstrap({
  setDocument,
  setDocumentPath,
  setSelection,
  setContextMenu,
  setError,
  setDatasetLibrary,
}: {
  setDocument: (value: DocumentView) => void;
  setDocumentPath: (value: string | null) => void;
  setSelection: (value: Selection | null) => void;
  setContextMenu: (value: ContextMenuState | null) => void;
  setError: (value: string | null) => void;
  setDatasetLibrary: (value: boolean) => void;
}) {
  useEffect(() => {
    let disposed = false;
    let stopOpened: (() => void) | undefined;
    let stopFailed: (() => void) | undefined;
    let stopChanged: (() => void) | undefined;
    let stopCollaborationFailed: (() => void) | undefined;

    const initialize = async () => {
      const documentLoad = Promise.all([getDocument(), getDocumentPath()]).then(
        ([nextDocument, path]) => {
          if (!disposed) {
            setDocument(nextDocument);
            setDocumentPath(path);
          }
        }
      );

      if ("__TAURI_INTERNALS__" in window) {
        // A launch that was not handed a document opens on the blank scratch
        // document, so the library is the only thing there is to act on.
        void shouldOpenLibrary().then((open) => {
          if (open && !disposed) setDatasetLibrary(true);
        });

        void (async () => {
          const opened = await listen<{ document: DocumentView; path: string }>(
            "framework-document-opened",
            (event) => {
              setDocument(event.payload.document);
              setDocumentPath(event.payload.path);
              setSelection(null);
              setContextMenu(null);
              setError(null);
            }
          );
          if (disposed) opened();
          else stopOpened = opened;

          const failed = await listen<string>(
            "framework-document-open-failed",
            (event) => setError(event.payload)
          );
          if (disposed) failed();
          else stopFailed = failed;

          const changed = await listen<DocumentView>(
            "framework-document-changed",
            (event) => {
              setDocument(event.payload);
              setError(null);
            }
          );
          if (disposed) changed();
          else stopChanged = changed;

          const collaborationFailed = await listen<string>(
            "framework-collaboration-failed",
            (event) => setError(event.payload)
          );
          if (disposed) collaborationFailed();
          else stopCollaborationFailed = collaborationFailed;
        })().catch((reason) => {
          if (!disposed)
            setError(`Could not subscribe to document updates: ${String(reason)}`);
        });
      }

      await documentLoad;
    };

    void initialize().catch((reason) => {
      if (!disposed) setError(String(reason));
    });

    return () => {
      disposed = true;
      stopOpened?.();
      stopFailed?.();
      stopChanged?.();
      stopCollaborationFailed?.();
    };
  }, [setContextMenu]);
}
