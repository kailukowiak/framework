import { CircleAlert, GitMerge, KeyRound, X } from "lucide-react";
import { useState } from "react";
import { useJoinDiagnostics } from "./hooks/useJoinDiagnostics";
import type { OperationHandler } from "./lib/handlers";
import type { JoinDiagnostics } from "./lib/api";
import type { DocumentView, FrameObject } from "./lib/types";
import type { JoinState } from "./lib/joinState";
import { joinKeysCompatible, suggestedLookupKey } from "./lib/joinKeys";
export { suggestedLookupKey } from "./lib/joinKeys";

export function LookupJoinPrompt({
  state,
  document,
  onClose,
  onMoreOptions,
  onOperation,
  onCreated,
}: {
  state: NonNullable<JoinState>;
  document: DocumentView;
  onClose: () => void;
  onMoreOptions: () => void;
  onOperation: OperationHandler;
  onCreated: () => void;
}) {
  const frames = document.objects.filter(
    (object): object is FrameObject => object.kind === "frame"
  );
  const primary = frames.find((frame) => frame.id === state.primaryFrameId)!;
  const lookup = frames.find((frame) => frame.id === state.lookupFrameId)!;
  const outputIds = state.lookupOutputColumnIds ?? [];
  const outputColumns = lookup.columns.filter((column) => outputIds.includes(column.id));
  const [primaryKeyId, setPrimaryKeyId] = useState(
    state.primaryKeyId ?? primary.columns[0]?.id ?? ""
  );
  const primaryKey = primary.columns.find((column) => column.id === primaryKeyId);
  const [lookupKeyId, setLookupKeyId] = useState(
    state.lookupKeyId ?? suggestedLookupKey(primaryKey, lookup)
  );
  const lookupKey = lookup.columns.find((column) => column.id === lookupKeyId);
  const compatible = joinKeysCompatible(primaryKey, lookupKey);
  const unique = lookup.uniqueKeys.some(
    (key) => key.columnIds.length === 1 && key.columnIds[0] === lookupKeyId
  );
  const { diagnostics, error: diagnosticsError, loading } = useJoinDiagnostics(
    primary.id,
    lookup.id,
    primaryKeyId,
    lookupKeyId,
    compatible
  );
  const [joinError, setJoinError] = useState<string | null>(null);
  const duplicates = diagnostics?.lookupDuplicateKeyValues ?? 0;
  const outputInputs = lookupOutputInputs(primary, lookup, outputIds);
  const outputNames = outputColumns.map((column) => column.name).join(", ");

  const create = () =>
    void onOperation(
      {
        type: "addJoinFrame",
        primaryFrameId: primary.id,
        lookupFrameId: lookup.id,
        primaryKeyColumnIds: [primaryKeyId],
        lookupKeyColumnIds: [lookupKeyId],
        joinType: "left",
        columns: outputInputs,
        name: `${primary.name} + ${lookup.name}`,
        x: state.x,
        y: state.y,
      },
      { inlineError: true }
    ).then((failure) => {
      setJoinError(failure);
      if (!failure) onCreated();
    });

  return (
    <div className="dialog-backdrop" onPointerDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <div className="insert-dialog lookup-join-prompt">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">LOOK UP COLUMNS</span>
            <h2>Bring {outputNames || "columns"} into {primary.name}</h2>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close lookup">
            <X size={16} />
          </button>
        </div>
        <div className="lookup-equation">
          <span>Match</span>
          <strong>{primary.name}</strong>
          <select
            aria-label="Destination key"
            value={primaryKeyId}
            onChange={(event) => setPrimaryKeyId(event.target.value)}
          >
            {primary.columns.map((column) => (
              <option key={column.id} value={column.id}>{column.name}</option>
            ))}
          </select>
          <span>to</span>
          <strong>{lookup.name}</strong>
          <select
            aria-label="Lookup key"
            value={lookupKeyId}
            onChange={(event) => setLookupKeyId(event.target.value)}
          >
            {lookup.columns.map((column) => (
              <option key={column.id} value={column.id}>{column.name}</option>
            ))}
          </select>
        </div>
        <LookupDiagnosticsStatus
          compatible={compatible}
          diagnostics={diagnostics}
          error={diagnosticsError}
          loading={loading}
          unique={unique}
          lookupKeyName={lookupKey?.name ?? "key"}
          onMarkUnique={() => void onOperation({
            type: "setUniqueKey",
            frameId: lookup.id,
            columnIds: [lookupKeyId],
            enabled: true,
          }, { inlineError: true }).then(setJoinError)}
        />
        {joinError && <p className="formula-editor-error">{joinError}</p>}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onMoreOptions}>More options</button>
          <button
            className="primary-action"
            disabled={!compatible || !unique || !diagnostics || duplicates > 0}
            onClick={create}
          >
            Bring columns over <GitMerge size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}

function lookupOutputInputs(
  primary: FrameObject,
  lookup: FrameObject,
  outputIds: string[]
) {
  const primaryNames = new Set(primary.columns.map((column) => column.name));
  const input = (frame: FrameObject, column: FrameObject["columns"][number]) => ({
    sourceFrameId: frame.id,
    sourceColumnId: column.id,
    name:
      frame.id === lookup.id && primaryNames.has(column.name)
        ? `${lookup.name} ${column.name}`
        : column.name,
  });
  return [
    ...primary.columns.map((column) => input(primary, column)),
    ...lookup.columns
      .filter((column) => outputIds.includes(column.id))
      .map((column) => input(lookup, column)),
  ];
}

function LookupDiagnosticsStatus({
  compatible,
  diagnostics,
  error,
  loading,
  unique,
  lookupKeyName,
  onMarkUnique,
}: {
  compatible: boolean;
  diagnostics: JoinDiagnostics | null;
  error: string | null;
  loading: boolean;
  unique: boolean;
  lookupKeyName: string;
  onMarkUnique: () => void;
}) {
  const duplicates = diagnostics?.lookupDuplicateKeyValues ?? 0;
  return (
    <>
      {!compatible && <p className="formula-editor-error">Choose keys with compatible types.</p>}
      {error && <p className="formula-editor-error">{error}</p>}
      {compatible && (
        <div className="lookup-diagnostics" aria-live="polite">
          {loading || !diagnostics ? <span>Checking all rows…</span> : <>
            <span>{diagnostics.matchedRows.toLocaleString()} matched</span>
            <span>{diagnostics.unmatchedRows.toLocaleString()} missing</span>
            <span className={duplicates ? "invalid" : ""}>
              {duplicates.toLocaleString()} duplicate keys
            </span>
          </>}
        </div>
      )}
      {diagnostics && diagnostics.lookupNullKeyRows > 0 && (
        <p className="lookup-note">
          {diagnostics.lookupNullKeyRows.toLocaleString()} lookup rows have a blank key.
        </p>
      )}
      {!unique && diagnostics && duplicates === 0 && (
        <button className="secondary-action lookup-mark-key" onClick={onMarkUnique}>
          <KeyRound size={13} /> Mark {lookupKeyName} as unique
        </button>
      )}
      {duplicates > 0 && (
        <p className="formula-editor-error">
          <CircleAlert size={12} /> Choose a key without duplicate values.
        </p>
      )}
    </>
  );
}
