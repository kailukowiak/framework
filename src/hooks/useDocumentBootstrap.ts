import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getDocument,
  getDocumentPath,
  shouldOpenLibrary,
} from "../lib/api";
import type { DocumentView } from "../lib/types";

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
  setError,
  setDatasetLibrary,
  onDocumentOpened,
}: {
  setDocument: (value: DocumentView) => void;
  setDocumentPath: (value: string | null) => void;
  setError: (value: string | null) => void;
  setDatasetLibrary: (value: boolean) => void;
  /**
   * A document arriving from outside this window — the Finder, a second
   * launch, another window saving over this file. It is an *open*, not a
   * refresh, so it goes through the same adoption the in-app open paths use
   * rather than clearing a hand-kept subset of the state here.
   */
  onDocumentOpened: (opened: { document: DocumentView; path: string }) => void;
}) {
  // Read through a ref because the subscription below is made once for the
  // life of the window: a handler captured in that one effect would go on
  // adopting documents into the state the window had when it launched.
  const adopt = useRef(onDocumentOpened);
  adopt.current = onDocumentOpened;

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
            (event) => adopt.current(event.payload)
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
    // Subscribed once for the life of the window: the handlers read through
    // callbacks that are themselves stable, and re-subscribing on every
    // document change would drop events between the two listeners.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
