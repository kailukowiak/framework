import { ChevronRight, FolderOpen } from "lucide-react";
import type { RecentDocument } from "./lib/api";
import { libraryEntryState } from "./lib/datasetLibraryEntries";

/**
 * The last folder or two a document lives in, e.g. "The FrameWork tour ›
 * Start" for a path ending ".../The FrameWork tour/Start/document.fw". Two
 * documents with the same name are told apart by this, not by however much
 * of the shared absolute path happens to survive a left-anchored
 * truncation — that shows identical prefixes for documents like the tour's
 * "Start" and "Answer key" copies.
 */
function pathContext(path: string): string {
  const segments = path.split(/[/\\]/).filter(Boolean);
  // Drop the filename itself, then keep the last two enclosing folders.
  const folders = segments.slice(0, -1);
  return folders.slice(-2).join(" › ");
}

/**
 * The device-local recents, each openable normally or (⌥) in safe mode. One
 * that FrameWork can no longer read — most often a stored macOS TCC deny —
 * stays listed, disabled and labelled, rather than opening into a bare
 * "Operation not permitted" error.
 */
export function RecentDocuments({
  loading,
  recents,
  opening,
  onOpen,
}: {
  loading: boolean;
  recents: RecentDocument[];
  opening: string | null;
  onOpen: (recent: RecentDocument, safeMode: boolean) => void;
}) {
  return (
    <>
      <div className="dataset-section-heading">
        <strong>Recent documents</strong>
        <span>On this device</span>
      </div>
      <div className="recent-document-list">
        {loading && (
          <div className="sample-loading">Looking for recent documents…</div>
        )}
        {!loading && recents.length === 0 && (
          <div className="sample-loading">
            Documents you open or create will appear here.
          </div>
        )}
        {recents.map((recent) => {
          const state = libraryEntryState({ exists: true, readable: recent.readable });
          return (
            <button
              className="recent-document"
              key={recent.path}
              disabled={opening !== null || state.disabled}
              title="Hold ⌥ to open in safe mode (no evaluation or data loading)"
              onClick={(event) => onOpen(recent, event.altKey)}
            >
              <span className="sample-icon">
                <FolderOpen size={16} />
              </span>
              <span>
                <strong>
                  {recent.title}
                  {state.suffix && (
                    <span className="library-entry-suffix"> — {state.suffix}</span>
                  )}
                </strong>
                <small title={recent.path}>{pathContext(recent.path)}</small>
              </span>
              <ChevronRight size={14} />
            </button>
          );
        })}
      </div>
    </>
  );
}
