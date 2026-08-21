import { CircleAlert, X } from "lucide-react";
import { useEffect } from "react";
import { connectorApprovalSubject } from "./lib/connectorApproval";
import type { ConnectorRecipe } from "./lib/types";

/**
 * Guards the moment a query or a file path that arrived *inside a document*
 * is about to run with the local user's own access — a shared workbook's
 * database connector otherwise executes its embedded SQL, using whatever
 * credentials the opener has saved for that connection, the instant they
 * click Refresh. Approval is remembered per document and frame (see
 * lib/connectorApproval.ts) so this only interrupts once, and again
 * whenever the query or path actually changes.
 */
export function ConnectorRefreshConfirmDialog({
  frameName,
  connector,
  onConfirm,
  onCancel,
}: {
  frameName: string;
  connector: ConnectorRecipe;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onCancel]);

  const isDatabase = connector.kind === "database";
  const subject = connectorApprovalSubject(connector);
  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <div className="insert-dialog connector-refresh-confirm">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">REFRESH</span>
            <h2>
              <CircleAlert size={16} /> Confirm this refresh
            </h2>
          </div>
          <button className="icon-button" onClick={onCancel}>
            <X size={18} />
          </button>
        </div>

        <p>
          <strong>{frameName}</strong> reads from a{" "}
          {isDatabase ? "database connection" : "file path"} named in this document,
          not one you just chose. Refreshing it will{" "}
          {isDatabase
            ? "run the following query using your saved connection"
            : "read from the following path"}
          :
        </p>
        <pre className="connector-refresh-subject">{subject}</pre>
        {isDatabase && (
          <p className="preference-note">Against connection: {connector.sourceName}</p>
        )}

        <div className="dialog-actions">
          <button className="secondary-action" onClick={onCancel}>
            Cancel
          </button>
          <button className="primary-action" onClick={onConfirm}>
            Refresh
          </button>
        </div>
      </div>
    </div>
  );
}
