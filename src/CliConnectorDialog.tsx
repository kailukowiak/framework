import { CircleAlert, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  listCliConnectorProfiles,
  saveCliConnectorProfile,
  type CliConnectorProfile,
  type CliConnectionKind,
  type CliOutputFormat,
  type CliSourceInput,
} from "./lib/api";

const NEW_PROFILE = "__new__";

export type CommandSourceKind = "api" | "script";

function connectionKind(kind: CommandSourceKind): CliConnectionKind {
  return kind;
}

function blankProfile(kind: CommandSourceKind): CliConnectorProfile {
  return {
    id: "",
    kind: connectionKind(kind),
    name: "",
    program: "",
    arguments: [],
    output: "csv",
  };
}

export function CliConnectorDialog({
  kind,
  onClose,
  onImport,
}: {
  kind: CommandSourceKind;
  onClose: () => void;
  onImport: (source: CliSourceInput) => Promise<void>;
}) {
  const [profiles, setProfiles] = useState<CliConnectorProfile[]>([]);
  const [selected, setSelected] = useState(NEW_PROFILE);
  const [draft, setDraft] = useState<CliConnectorProfile>(() => blankProfile(kind));
  const [argumentsText, setArgumentsText] = useState("");
  const [sourceLabel, setSourceLabel] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listCliConnectorProfiles()
      .then((items) => {
        const matching = items.filter((item) => item.kind === connectionKind(kind));
        setProfiles(matching);
        if (matching.length > 0) {
          setSelected(matching[0].id);
          setDraft(matching[0]);
          setArgumentsText(matching[0].arguments.join("\n"));
        }
      })
      .catch((reason) => setError(String(reason).replace(/^Error:\s*/, "")));
  }, [kind]);

  const canSubmit = useMemo(
    () =>
      draft.name.trim().length > 0 &&
      draft.program.trim().length > 0 &&
      sourceLabel.trim().length > 0,
    [draft.name, draft.program, sourceLabel]
  );

  const chooseProfile = (id: string) => {
    setSelected(id);
    setError(null);
    const profile = profiles.find((candidate) => candidate.id === id) ?? blankProfile(kind);
    setDraft(profile);
    setArgumentsText(profile.arguments.join("\n"));
  };

  const submit = async () => {
    if (!canSubmit || busy) return;
    setBusy(true);
    setError(null);
    try {
      const profile = await saveCliConnectorProfile({
        ...draft,
        arguments: argumentsText.split("\n"),
      });
      await onImport({
        profileId: profile.id,
        sourceLabel: sourceLabel.trim(),
        query: null,
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
          <div><span className="eyebrow">ADD DATA</span><h2>{kind === "api" ? "Web API" : "Script / CLI"}</h2></div>
          <button className="icon-button" onClick={onClose} disabled={busy} aria-label="Close command connector"><X size={18} /></button>
        </div>
        <div className="cli-connector-grid">
          <label>Connection
            <select value={selected} onChange={(event) => chooseProfile(event.target.value)}>
              {profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
              <option value={NEW_PROFILE}>New connection…</option>
            </select>
          </label>
          <label>Connection name
            <input value={draft.name} placeholder="Read-only source" onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
          </label>
          <label className="cli-program-field">Executable
            <input value={draft.program} placeholder="/usr/local/bin/tool" onChange={(event) => setDraft({ ...draft, program: event.target.value })} />
          </label>
          <label>Result format
            <select value={draft.output} onChange={(event) => setDraft({ ...draft, output: event.target.value as CliOutputFormat })}>
              <option value="csv">CSV</option><option value="tsv">TSV</option><option value="parquet">Parquet</option>
            </select>
          </label>
          <label className="cli-arguments-field">Arguments — one per line
            <textarea value={argumentsText} placeholder="fetch\n{source}" onChange={(event) => setArgumentsText(event.target.value)} />
          </label>
          <label className="cli-source-field">{kind === "api" ? "Endpoint" : "Source name or address"}
            <input value={sourceLabel} placeholder={kind === "api" ? "https://api.example.com/orders" : "west-region"} onChange={(event) => setSourceLabel(event.target.value)} />
          </label>
          <div className="cli-result-mode cli-source-field">
            <span>Result</span>
            <strong>Cached result</strong>
          </div>
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
