import { ChevronRight, FolderOpen } from "lucide-react";
import type { RecentDocument } from "./lib/api";

/** The device-local recents, each openable normally or (⌥) in safe mode. */
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
        {recents.map((recent) => (
          <button
            className="recent-document"
            key={recent.path}
            disabled={opening !== null}
            title="Hold ⌥ to open in safe mode (no evaluation or data loading)"
            onClick={(event) => onOpen(recent, event.altKey)}
          >
            <span className="sample-icon">
              <FolderOpen size={16} />
            </span>
            <span>
              <strong>{recent.title}</strong>
              <small>{recent.path}</small>
            </span>
            <ChevronRight size={14} />
          </button>
        ))}
      </div>
    </>
  );
}
