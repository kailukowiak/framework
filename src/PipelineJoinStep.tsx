import { GitMerge, KeyRound } from "lucide-react";
import { useState } from "react";
import { useJoinDiagnostics } from "./hooks/useJoinDiagnostics";
import type { OperationHandler } from "./lib/handlers";
import { joinKeysCompatible, sameNamedKeyId } from "./lib/joinKeys";
import type { FrameObject } from "./lib/types";

export function PipelineJoinStep({
  frame,
  frames,
  onOperation,
}: {
  frame: FrameObject;
  frames: FrameObject[];
  onOperation: OperationHandler;
}) {
  const join = frame.derivation?.join;
  const primary = frames.find(
    (candidate) => candidate.id === frame.derivation?.sourceFrameId
  );
  const lookup = frames.find((candidate) => candidate.id === join?.lookupFrameId);
  const [primaryKeyId, setPrimaryKeyId] = useState(
    join?.primaryKeyColumnIds[0] ?? ""
  );
  const [lookupKeyId, setLookupKeyId] = useState(join?.lookupKeyColumnIds[0] ?? "");
  const [error, setError] = useState<string | null>(null);
  const primaryKey = primary?.columns.find((column) => column.id === primaryKeyId);
  const lookupKey = lookup?.columns.find((column) => column.id === lookupKeyId);
  const compatible = joinKeysCompatible(primaryKey, lookupKey);
  const unique = Boolean(lookup?.uniqueKeys.some(
    (key) => key.columnIds.length === 1 && key.columnIds[0] === lookupKeyId
  ));
  const { diagnostics, loading } = useJoinDiagnostics(
    primary?.id,
    lookup?.id,
    primaryKeyId,
    lookupKeyId,
    compatible
  );
  const outputNames = (join?.outputs ?? [])
    .filter((output) => output.sourceFrameId === lookup?.id)
    .map(
      (output) =>
        frame.baseColumns?.find((column) => column.id === output.outputColumnId)?.name ??
        lookup?.columns.find((column) => column.id === output.sourceColumnId)?.name
    )
    .filter((name): name is string => Boolean(name));

  if (!join || !primary || !lookup) return null;

  const save = (nextPrimary: string, nextLookup: string) => {
    setPrimaryKeyId(nextPrimary);
    setLookupKeyId(nextLookup);
    setError(null);
    void onOperation(
      {
        type: "setFrameJoinKeys",
        frameId: frame.id,
        primaryKeyColumnIds: [nextPrimary],
        lookupKeyColumnIds: [nextLookup],
      },
      { inlineError: true }
    ).then(setError);
  };

  return (
    <section className="pipeline-step pipeline-join-step">
      <div className="pipeline-step-heading">
        <span className="pipeline-step-index">1</span>
        <strong>Look up columns</strong>
      </div>
      <div className="pipeline-join-reading">
        <GitMerge size={13} />
        <span>Bring {outputNames.join(", ") || `${lookup.name} columns`} from {lookup.name}</span>
      </div>
      <div className="pipeline-join-keys">
        <select
          aria-label="Destination join key"
          value={primaryKeyId}
          onChange={(event) => {
            const nextPrimary = primary.columns.find(
              (column) => column.id === event.target.value
            );
            const paired = sameNamedKeyId(lookup.columns, nextPrimary?.name ?? "");
            save(event.target.value, paired ?? lookupKeyId);
          }}
        >
          {primary.columns.map((column) => (
            <option key={column.id} value={column.id}>{column.name}</option>
          ))}
        </select>
        <span>=</span>
        <select
          aria-label="Lookup join key"
          value={lookupKeyId}
          onChange={(event) => {
            const nextLookup = lookup.columns.find(
              (column) => column.id === event.target.value
            );
            const paired = sameNamedKeyId(primary.columns, nextLookup?.name ?? "");
            save(paired ?? primaryKeyId, event.target.value);
          }}
        >
          {lookup.columns.map((column) => (
            <option key={column.id} value={column.id}>{column.name}</option>
          ))}
        </select>
      </div>
      <PipelineJoinStatus
        compatible={compatible}
        diagnostics={diagnostics}
        loading={loading}
        unique={unique}
        error={error}
      />
    </section>
  );
}

function PipelineJoinStatus({
  compatible,
  diagnostics,
  loading,
  unique,
  error,
}: {
  compatible: boolean;
  diagnostics: ReturnType<typeof useJoinDiagnostics>["diagnostics"];
  loading: boolean;
  unique: boolean;
  error: string | null;
}) {
  return <>
    <div className="pipeline-join-status">
      <span>{loading || !diagnostics
        ? "Checking all rows…"
        : `${diagnostics.matchedRows.toLocaleString()} matched · ${diagnostics.unmatchedRows.toLocaleString()} missing`}</span>
      {unique && <KeyRound size={11} aria-label="Unique lookup key" />}
    </div>
    {!compatible && <p className="formula-editor-error">Choose compatible key types.</p>}
    {!unique && <p className="formula-editor-error">The lookup key must be marked unique.</p>}
    {error && <p className="formula-editor-error">{error}</p>}
  </>;
}
