import { CircleAlert, Database, FolderOpen, KeyRound, Plus, RefreshCw, X } from "lucide-react";
import { useMemo, useState } from "react";
import { DATA_SOURCE_LABELS, groupDataSources } from "./lib/dataSources";
import type { DocumentView } from "./lib/types";
import type { RefreshConnectorHandler, SetFrameSourceHandler } from "./FrameGrid";

export function DataSidebar({
  document,
  selectedObjectId,
  onJump,
  onImport,
  onRefreshConnector,
  onSourceChanged,
  onClose,
}: {
  document: DocumentView;
  selectedObjectId?: string;
  onJump: (objectId: string) => void;
  onImport: () => void;
  onRefreshConnector: RefreshConnectorHandler;
  onSourceChanged: SetFrameSourceHandler;
  onClose: () => void;
}) {
  const groups = useMemo(() => groupDataSources(document), [document]);
  const [busyFrameId, setBusyFrameId] = useState<string | null>(null);
  const [failure, setFailure] = useState<{ frameId: string; message: string } | null>(
    null
  );

  const runAction = (frameId: string, action: () => Promise<string | null>) => {
    setBusyFrameId(frameId);
    setFailure(null);
    void action()
      .then((message) => setFailure(message ? { frameId, message } : null))
      .finally(() => setBusyFrameId(null));
  };

  return (
    <aside className="data-sidebar">
      <div className="data-sidebar-header">
        <div>
          <span className="eyebrow">SOURCES</span>
          <h2>Data</h2>
        </div>
        <button className="icon-button" onClick={onClose} aria-label="Close sources">
          <X size={16} />
        </button>
      </div>
      <button className="secondary-action" onClick={onImport}>
        <Plus size={14} /> Add data…
      </button>

      {groups.length === 0 && (
        <p className="data-sidebar-empty">
          Nothing here yet. Import a file, or add a frame and type into it.
        </p>
      )}

      {groups.map((group) => (
        <section className="data-sidebar-group" key={group.kind}>
          {/* The heading is the legend, so it is written in the same two
              colours the cards use — read it once here and every card's
              nature is legible without coming back. */}
          <div className={`data-sidebar-group-heading kind-${group.kind}`}>
            <span className="source-kind-dot" title={DATA_SOURCE_LABELS[group.kind]} />
            <strong className="nature-words">
              <span className={`origin-${group.nature.origin}`}>
                {group.nature.origin}
              </span>
              <i>·</i>
              <span className={`refresh-${group.nature.refresh}`}>
                {group.nature.refresh}
              </span>
            </strong>
            <span>{group.entries.length}</span>
          </div>
          {group.entries.map((entry) => {
            const busy = busyFrameId === entry.frame.id;
            const reason = entry.computed?.editing?.reason;
            return (
              <div
                className={`source-entry kind-${entry.kind} ${
                  selectedObjectId === entry.frame.id ? "selected" : ""
                }`}
                key={entry.frame.id}
              >
                <button
                  className="source-jump"
                  onClick={() => onJump(entry.frame.id)}
                  title="Show this on the canvas"
                >
                  <span className="source-name">
                    <strong>{entry.frame.name}</strong>
                    {!entry.editable && (
                      <KeyRound
                        className="source-locked"
                        size={11}
                        aria-label="Read-only"
                      />
                    )}
                  </span>
                  <small title={entry.title ?? reason}>{entry.detail}</small>
                  {(entry.stale || entry.upstreamStale || entry.cached) && (
                    <span className="source-flags">
                      {/* No "live" flag here any more. It existed because the
                          old headings named where data came from and never
                          said whether it moved, so a derived frame downstream
                          of a file had to say so itself. The heading says it
                          now. */}
                      {entry.stale ? (
                        <span className="source-flag stale">
                          <CircleAlert size={10} /> out of date
                        </span>
                      ) : entry.upstreamStale ? (
                        <span className="source-flag stale">
                          <CircleAlert size={10} /> reading old numbers
                        </span>
                      ) : entry.cached ? (
                        <span className="source-flag">
                          <Database size={10} /> cached
                        </span>
                      ) : null}
                    </span>
                  )}
                </button>
                {(entry.frame.connector ||
                  entry.frame.artifact ||
                  entry.frame.sourceFile) && (
                  <div className="source-actions">
                    {entry.frame.connector && (
                      <button
                        className="icon-button subtle"
                        disabled={busy}
                        title="Refresh this cached result"
                        aria-label={`Refresh ${entry.frame.name}`}
                        onClick={() =>
                          runAction(entry.frame.id, () =>
                            onRefreshConnector(entry.frame.id, { inlineError: true })
                          )
                        }
                      >
                        <RefreshCw className={busy ? "spinning" : ""} size={13} />
                      </button>
                    )}
                    <button
                      className="icon-button subtle"
                      disabled={busy}
                      title={
                        entry.frame.connector
                          ? "Point this frame at a different file"
                          : "Link a file so this frame can be refreshed"
                      }
                      aria-label={`Change the file behind ${entry.frame.name}`}
                      onClick={() =>
                        runAction(entry.frame.id, () => onSourceChanged(entry.frame.id))
                      }
                    >
                      <FolderOpen size={13} />
                    </button>
                  </div>
                )}
                {failure?.frameId === entry.frame.id && (
                  <p className="source-failure">
                    <CircleAlert size={11} /> {failure.message}
                  </p>
                )}
              </div>
            );
          })}
        </section>
      ))}
    </aside>
  );
}
