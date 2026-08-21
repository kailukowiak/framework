import { CircleAlert, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  listDatabaseConnections,
  saveDatabaseConnection,
  type DatabaseConnection,
  type DatabaseSourceInput,
} from "./lib/api";

const NEW_CONNECTION = "__new__";
const blankConnection: DatabaseConnection = { id: "", name: "", uri: "" };

export function DatabaseConnectorDialog({
  onClose,
  onImport,
}: {
  onClose: () => void;
  onImport: (source: DatabaseSourceInput) => Promise<void>;
}) {
  const [connections, setConnections] = useState<DatabaseConnection[]>([]);
  const [selected, setSelected] = useState(NEW_CONNECTION);
  const [draft, setDraft] = useState<DatabaseConnection>(blankConnection);
  const [sourceName, setSourceName] = useState("");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listDatabaseConnections()
      .then((items) => {
        setConnections(items);
        if (items.length > 0) {
          setSelected(items[0].id);
          setDraft(items[0]);
        }
      })
      .catch((reason) => setError(String(reason).replace(/^Error:\s*/, "")));
  }, []);

  const canSubmit = useMemo(
    () =>
      draft.name.trim().length > 0 &&
      draft.uri.trim().length > 0 &&
      sourceName.trim().length > 0 &&
      query.trim().length > 0,
    [draft.name, draft.uri, query, sourceName]
  );

  const chooseConnection = (id: string) => {
    setSelected(id);
    setError(null);
    setDraft(connections.find((item) => item.id === id) ?? blankConnection);
  };

  const submit = async () => {
    if (!canSubmit || busy) return;
    setBusy(true);
    setError(null);
    try {
      const connection = await saveDatabaseConnection(draft);
      await onImport({
        connectionId: connection.id,
        sourceName: sourceName.trim(),
        query: query.trim(),
      });
    } catch (reason) {
      setError(String(reason).replace(/^Error:\s*/, ""));
      setBusy(false);
    }
  };

  return (
    <div className="dialog-backdrop" onPointerDown={(event) => {
      if (event.target === event.currentTarget && !busy) onClose();
    }}>
      <div className="insert-dialog cli-connector-dialog">
        <div className="dialog-header">
          <div><span className="eyebrow">ADD DATA</span><h2>Database</h2></div>
          <button className="icon-button" onClick={onClose} disabled={busy} aria-label="Close database connector"><X size={18} /></button>
        </div>
        <div className="cli-connector-grid">
          <label>Connection
            <select value={selected} onChange={(event) => chooseConnection(event.target.value)}>
              {connections.map((connection) => <option key={connection.id} value={connection.id}>{connection.name}</option>)}
              <option value={NEW_CONNECTION}>New connection…</option>
            </select>
          </label>
          <label>Connection name
            <input value={draft.name} placeholder="Warehouse" onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
          </label>
          <label className="cli-program-field">URI
            <input value={draft.uri} placeholder="postgresql://user:password@server/database" onChange={(event) => setDraft({ ...draft, uri: event.target.value })} />
          </label>
          <label className="cli-source-field">Table name
            <input value={sourceName} placeholder="Ledger" onChange={(event) => setSourceName(event.target.value)} />
          </label>
          <label className="cli-arguments-field">SQL
            <textarea value={query} placeholder="select * from finance.ledger" onChange={(event) => setQuery(event.target.value)} />
          </label>
          <div className="cli-result-mode cli-source-field"><span>Result</span><strong>Cached result</strong></div>
        </div>
        {error && <p className="formula-editor-error"><CircleAlert size={12} /> {error}</p>}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose} disabled={busy}>Cancel</button>
          <button className="primary-action" onClick={() => void submit()} disabled={!canSubmit || busy}>{busy ? "Reading…" : "Add"}</button>
        </div>
      </div>
    </div>
  );
}
