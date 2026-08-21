import { Check, Database, FolderOpen } from "lucide-react";
import { useState } from "react";
import type { SetFrameSourceHandler } from "./FrameGrid";
import { FormulaErrorDetails } from "./FormulaEditor";
import { connectorSourceLabel } from "./lib/dataSources";
import type { FrameObject } from "./lib/types";

function connectorName(frame: FrameObject): string {
  if (!frame.connector) return "Imported snapshot";
  return frame.connector.kind === "file" ? "File connector" : "Command connector";
}

function ConnectorActions({
  frame,
  busy,
  run,
  onSourceChanged,
}: {
  frame: FrameObject;
  busy: "refresh" | "repoint" | null;
  run: (
    kind: "refresh" | "repoint",
    action: () => Promise<string | null>,
    success: string
  ) => void;
  onSourceChanged: SetFrameSourceHandler;
}) {
  const connector = frame.connector;
  const canRepoint = !connector || connector.kind === "file";
  return (
    <div className="connector-actions">
      {canRepoint && (
        <button
          className="secondary-action"
          disabled={busy !== null}
          title="Read this frame from a different file"
          onClick={() => run("repoint", () => onSourceChanged(frame.id), "Source file changed")}
        >
          <FolderOpen size={13} />{" "}
          {busy === "repoint" ? "Opening…" : connector ? "Change file…" : "Link a file…"}
        </button>
      )}
    </div>
  );
}

function ConnectorResult({
  frame,
  done,
  error,
}: {
  frame: FrameObject;
  done: string | null;
  error: string | null;
}) {
  return (
    <>
      {done && (
        <p className="connector-refresh-success">
          <Check size={12} /> {done} · {(frame.artifact?.rowCount ?? 0).toLocaleString()} rows
        </p>
      )}
      {error && <FormulaErrorDetails title="Could not refresh the source" error={error} />}
    </>
  );
}

export function FrameSourcePanel({
  frame,
  onSourceChanged,
}: {
  frame: FrameObject;
  onSourceChanged: SetFrameSourceHandler;
}) {
  const [busy, setBusy] = useState<"refresh" | "repoint" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const connector = frame.connector;
  const source = connector ? connectorSourceLabel(connector) : undefined;
  const shortSource = source?.split(/[\\/]/).pop();
  const description = connector
    ? "Refresh reads the source into a new immutable artifact while this frame and its surviving columns keep their IDs. New columns are added, unused missing columns are removed, and a missing column used downstream stays visible with an explicit error."
    : "This snapshot has no source file recorded. Linking one lets it be refreshed from then on; columns are reconciled by their physical source names so surviving formulas keep their references.";

  const run = (
    kind: "refresh" | "repoint",
    action: () => Promise<string | null>,
    success: string
  ) => {
    setBusy(kind);
    setError(null);
    setDone(null);
    void action()
      .then((failure) => {
        setError(failure);
        setDone(failure ? null : success);
      })
      .finally(() => setBusy(null));
  };

  return (
    <div className="connector-refresh-panel">
      <div>
        <Database size={15} />
        <span>
          <strong>{connectorName(frame)}</strong>
          <small title={source ?? frame.artifact?.sourceName}>
            {shortSource ?? frame.artifact?.sourceName ?? "No source file recorded"}
          </small>
        </span>
      </div>
      <ConnectorActions
        frame={frame}
        busy={busy}
        run={run}
        onSourceChanged={onSourceChanged}
      />
      <ConnectorResult frame={frame} done={done} error={error} />
      <p>{description}</p>
    </div>
  );
}
