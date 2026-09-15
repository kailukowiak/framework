import { CircleAlert, GitMerge, KeyRound } from "lucide-react";
import { useState } from "react";
import { useJoinDiagnostics } from "./hooks/useJoinDiagnostics";
import type { JoinDiagnostics } from "./lib/api";
import type { OperationHandler } from "./lib/handlers";
import { joinKeysCompatible, normalizedKeyName } from "./lib/joinKeys";
import type { JoinState } from "./lib/joinState";
import type { Column, FrameJoinType, FrameObject } from "./lib/types";

/**
 * One row of the key equation. Composite keys are ordinary in a ledger — a
 * period and an account, an entity and a currency — and the core has always
 * taken parallel arrays of key columns. Only the dialog was single-pair.
 */
type KeyPair = { id: string; primaryColumnId: string; lookupColumnId: string };

/**
 * *Match on a key*: the join, which creates a new frame rather than
 * appending to a chain. Kept as its own file because the three chain
 * relationships beside it share almost nothing with it — this one asks
 * about keys, uniqueness and diagnostics, and they ask about row counts.
 */
export function CombineMatchForm({
  state,
  primary,
  candidates,
  onOperation,
  onCreated,
}: {
  state: NonNullable<JoinState>;
  primary: FrameObject;
  candidates: FrameObject[];
  onOperation: OperationHandler;
  onCreated: () => void;
}) {
  const initialLookup =
    candidates.find((frame) => frame.id === state.lookupFrameId) ??
    candidates.find((frame) => frame.uniqueKeys.length > 0) ??
    candidates[0];
  const [lookupFrameId, setLookupFrameId] = useState(initialLookup?.id ?? "");
  const [keys, setKeys] = useState<KeyPair[]>(() => [
    initialKeyPair(primary, initialLookup, state),
  ]);
  const [joinType, setJoinType] = useState<FrameJoinType>("left");
  const [name, setName] = useState(
    initialLookup
      ? `${primary.name} + ${initialLookup.name}`
      : `${primary.name} joined`
  );
  const [selected, setSelected] = useState<Set<string>>(() =>
    defaultOutputSelection(primary, initialLookup, state)
  );
  const [joinError, setJoinError] = useState<string | null>(null);

  const membershipOnly = joinType === "anti" || joinType === "semi";
  const lookup = candidates.find((frame) => frame.id === lookupFrameId);
  const primaryKeyIds = keys.map((pair) => pair.primaryColumnId);
  const lookupKeyIds = keys.map((pair) => pair.lookupColumnId);
  const compatible =
    keys.length > 0 &&
    keys.every((pair) =>
      joinKeysCompatible(
        columnOf(primary, pair.primaryColumnId),
        columnOf(lookup, pair.lookupColumnId)
      )
    );
  const lookupIsExplicitlyUnique = Boolean(
    lookup?.uniqueKeys.some((key) => sameIdSet(key.columnIds, lookupKeyIds))
  );
  const { diagnostics, error: diagnosticsError, loading } = useJoinDiagnostics(
    primary.id,
    lookup?.id,
    primaryKeyIds,
    lookupKeyIds,
    compatible
  );
  const duplicateKeys = diagnostics?.lookupDuplicateKeyValues ?? 0;
  const outputInputs = joinOutputInputs(
    primary,
    lookup,
    selected,
    membershipOnly
  );

  const chooseLookup = (frameId: string) => {
    const next = candidates.find((frame) => frame.id === frameId)!;
    setLookupFrameId(frameId);
    setKeys([initialKeyPair(primary, next, {})]);
    setName(`${primary.name} + ${next.name}`);
    setSelected(defaultOutputSelection(primary, next, {}));
    setJoinError(null);
  };

  // A membership join answers "which of my rows", so it produces only this
  // frame's columns. Dropping the other side's ticks here rather than
  // filtering at save keeps the checklist honest about what will come out.
  const chooseJoinType = (next: FrameJoinType) => {
    setJoinType(next);
    if (next === "anti" || next === "semi")
      setSelected(
        (current) =>
          new Set(
            Array.from(current).filter((key) => key.startsWith(`${primary.id}:`))
          )
      );
  };

  return (
    <>
      <MatchEquation
        primary={primary}
        lookup={lookup}
        candidates={candidates}
        lookupFrameId={lookupFrameId}
        keys={keys}
        compatible={compatible}
        explicitlyUnique={lookupIsExplicitlyUnique}
        membershipOnly={membershipOnly}
        diagnostics={diagnostics}
        loading={loading}
        error={diagnosticsError}
        onChooseLookup={chooseLookup}
        onChangeKeys={(next) => {
          setKeys(next);
          setJoinError(null);
        }}
        onMarkUnique={() =>
          void onOperation(
            {
              type: "setUniqueKey",
              frameId: lookup!.id,
              columnIds: lookupKeyIds,
              enabled: true,
            },
            { inlineError: true }
          ).then(setJoinError)
        }
      />
      <MatchKeepMode
        primaryName={primary.name}
        joinType={joinType}
        columnsOut={outputInputs.length}
        onChange={chooseJoinType}
      />
      <MatchOutputColumns
        frames={membershipOnly ? [primary] : [primary, lookup]}
        selected={selected}
        onToggle={(key, on) =>
          setSelected((current) => {
            const next = new Set(current);
            if (on) next.add(key);
            else next.delete(key);
            return next;
          })
        }
      />
      <MatchResult
        name={name}
        onName={setName}
        error={joinError}
        disabled={Boolean(
          !name.trim() ||
            !lookup ||
            !compatible ||
            outputInputs.length === 0 ||
            (!membershipOnly &&
              (!lookupIsExplicitlyUnique || !diagnostics || duplicateKeys > 0))
        )}
        onCreate={() =>
          void onOperation(
            {
              type: "addJoinFrame",
              primaryFrameId: primary.id,
              lookupFrameId: lookup!.id,
              primaryKeyColumnIds: primaryKeyIds,
              lookupKeyColumnIds: lookupKeyIds,
              joinType,
              columns: outputInputs,
              name: name.trim(),
              x: state.x,
              y: state.y,
            },
            { inlineError: true }
          ).then((failure) => {
            setJoinError(failure);
            if (!failure) onCreated();
          })
        }
      />
    </>
  );
}

function columnOf(frame: FrameObject | undefined, columnId: string) {
  return frame?.columns.find((column) => column.id === columnId);
}

function sameIdSet(left: string[], right: string[]): boolean {
  return (
    left.length === right.length &&
    left.every((id) => right.includes(id)) &&
    new Set(right).size === right.length
  );
}

/** The pair a fresh row opens on: the gesture's keys, then same-name guesses. */
function initialKeyPair(
  primary: FrameObject,
  lookup: FrameObject | undefined,
  state: Partial<NonNullable<JoinState>>
): KeyPair {
  const lookupColumnId =
    (lookup?.columns.some((column) => column.id === state.lookupKeyId)
      ? state.lookupKeyId
      : lookup?.uniqueKeys[0]?.columnIds[0]) ??
    lookup?.columns[0]?.id ??
    "";
  const lookupColumn = columnOf(lookup, lookupColumnId);
  const primaryColumnId =
    (primary.columns.some((column) => column.id === state.primaryKeyId)
      ? state.primaryKeyId
      : sameNamed(primary.columns, lookupColumn?.name)) ??
    primary.columns[0]?.id ??
    "";
  return { id: crypto.randomUUID(), primaryColumnId, lookupColumnId };
}

function sameNamed(columns: Column[], name: string | undefined) {
  return columns.find(
    (column) => normalizedKeyName(column.name) === normalizedKeyName(name ?? "")
  )?.id;
}

function defaultOutputSelection(
  primary: FrameObject,
  lookup: FrameObject | undefined,
  state: Partial<NonNullable<JoinState>>
): Set<string> {
  const keyId = initialKeyPair(primary, lookup, state).lookupColumnId;
  return new Set([
    ...primary.columns.map((column) => `${primary.id}:${column.id}`),
    ...(lookup?.columns
      .filter((column) =>
        state.lookupOutputColumnIds?.length
          ? state.lookupOutputColumnIds.includes(column.id)
          : column.id !== keyId
      )
      .map((column) => `${lookup.id}:${column.id}`) ?? []),
  ]);
}

function joinOutputInputs(
  primary: FrameObject,
  lookup: FrameObject | undefined,
  selected: Set<string>,
  membershipOnly: boolean
) {
  return (membershipOnly ? [primary] : [primary, lookup])
    .filter((frame): frame is FrameObject => Boolean(frame))
    .flatMap((frame) =>
      frame.columns
        .filter((column) => selected.has(`${frame.id}:${column.id}`))
        .map((column) => ({
          sourceFrameId: frame.id,
          sourceColumnId: column.id,
          name:
            frame.id === lookup?.id &&
            primary.columns.some(
              (candidate) =>
                candidate.name === column.name &&
                selected.has(`${primary.id}:${candidate.id}`)
            )
              ? `${frame.name} ${column.name}`
              : column.name,
        }))
    );
}

function MatchKeyStatus({
  compatible,
  membershipOnly,
  explicitlyUnique,
  diagnostics,
  loading,
  error,
  keyNames,
  onMarkUnique,
}: {
  compatible: boolean;
  membershipOnly: boolean;
  explicitlyUnique: boolean;
  diagnostics: JoinDiagnostics | null;
  loading: boolean;
  error: string | null;
  keyNames: string;
  onMarkUnique: () => void;
}) {
  const duplicates = diagnostics?.lookupDuplicateKeyValues ?? 0;
  if (!compatible)
    return (
      <p className="formula-editor-error">
        <CircleAlert size={12} /> Choose columns with compatible types.
      </p>
    );
  return (
    <>
      <div className="combine-diagnostics" aria-live="polite">
        {loading || !diagnostics ? (
          <span>Checking all rows…</span>
        ) : (
          <>
            <span>{diagnostics.matchedRows.toLocaleString()} matched</span>
            <span>{diagnostics.unmatchedRows.toLocaleString()} unmatched</span>
            <span className={duplicates ? "invalid" : ""}>
              {duplicates.toLocaleString()} duplicate keys
            </span>
          </>
        )}
      </div>
      {!membershipOnly &&
        diagnostics &&
        duplicates === 0 &&
        (explicitlyUnique ? (
          <p className="combine-line lookup-key-unique">
            <KeyRound size={13} /> {keyNames} is unique
          </p>
        ) : (
          <button className="secondary-action combine-line" onClick={onMarkUnique}>
            <KeyRound size={13} /> Mark {keyNames} as unique
          </button>
        ))}
      {duplicates > 0 && (
        <p className="formula-editor-error">
          <CircleAlert size={12} /> Choose a key without duplicate values.
        </p>
      )}
      {error && <p className="formula-editor-error">{error}</p>}
    </>
  );
}

/**
 * The key equation, one row per pair. A composite key is several columns
 * matched together, not several joins, so the rows sit in one block under a
 * single "another key" — the same shape a unique-key constraint has.
 */
function MatchKeys({
  primary,
  lookup,
  keys,
  onChange,
}: {
  primary: FrameObject;
  lookup: FrameObject | undefined;
  keys: KeyPair[];
  onChange: (keys: KeyPair[]) => void;
}) {
  const replace = (id: string, change: Partial<KeyPair>) =>
    onChange(
      keys.map((pair) => (pair.id === id ? { ...pair, ...change } : pair))
    );
  return (
    <div className="combine-keys">
      {keys.map((pair, index) => (
        <div className="combine-key-pair" key={pair.id}>
          <select
            aria-label={
              index === 0 ? "Destination key" : `Destination key ${index + 1}`
            }
            value={pair.primaryColumnId}
            onChange={(event) =>
              replace(pair.id, { primaryColumnId: event.target.value })
            }
          >
            {primary.columns.map((column) => (
              <option key={column.id} value={column.id}>
                {column.name}
              </option>
            ))}
          </select>
          <span>=</span>
          <select
            aria-label={index === 0 ? "Lookup key" : `Lookup key ${index + 1}`}
            value={pair.lookupColumnId}
            onChange={(event) =>
              replace(pair.id, { lookupColumnId: event.target.value })
            }
          >
            {lookup?.columns.map((column) => (
              <option key={column.id} value={column.id}>
                {column.name}
              </option>
            ))}
          </select>
          {keys.length > 1 && (
            <button
              className="combine-drop-key"
              aria-label={`Remove key ${index + 1}`}
              onClick={() =>
                onChange(keys.filter((candidate) => candidate.id !== pair.id))
              }
            >
              ×
            </button>
          )}
        </div>
      ))}
      <button
        className="combine-add-key"
        disabled={!lookup}
        onClick={() => onChange([...keys, initialKeyPair(primary, lookup, {})])}
      >
        another key
      </button>
    </div>
  );
}

function MatchOutputColumns({
  frames,
  selected,
  onToggle,
}: {
  frames: Array<FrameObject | undefined>;
  selected: Set<string>;
  onToggle: (key: string, on: boolean) => void;
}) {
  return (
    <div className="join-columns">
      {frames.map(
        (frame) =>
          frame && (
            <div key={frame.id}>
              <strong>{frame.name}</strong>
              {frame.columns.map((column) => {
                const key = `${frame.id}:${column.id}`;
                return (
                  <label key={column.id}>
                    <input
                      type="checkbox"
                      checked={selected.has(key)}
                      onChange={(event) => onToggle(key, event.target.checked)}
                    />
                    <span>{column.name}</span>
                    <small>{column.dataType}</small>
                  </label>
                );
              })}
            </div>
          )
      )}
    </div>
  );
}

function MatchResult({
  name,
  onName,
  error,
  disabled,
  onCreate,
}: {
  name: string;
  onName: (name: string) => void;
  error: string | null;
  disabled: boolean;
  onCreate: () => void;
}) {
  return (
    <>
      <label className="combine-line">
        Result name
        <input
          aria-label="Result name"
          value={name}
          onChange={(event) => onName(event.target.value)}
        />
      </label>
      {error && (
        <p className="formula-editor-error">
          <CircleAlert size={12} /> {error}
        </p>
      )}
      <div className="dialog-actions">
        <button className="primary-action" disabled={disabled} onClick={onCreate}>
          Create joined frame <GitMerge size={15} />
        </button>
      </div>
    </>
  );
}

/**
 * Which two tables, matched on which columns, and what the engine makes of
 * those keys over the whole plan. One block, because it is one question.
 */
function MatchEquation({
  primary,
  lookup,
  candidates,
  lookupFrameId,
  keys,
  compatible,
  explicitlyUnique,
  membershipOnly,
  diagnostics,
  loading,
  error,
  onChooseLookup,
  onChangeKeys,
  onMarkUnique,
}: {
  primary: FrameObject;
  lookup: FrameObject | undefined;
  candidates: FrameObject[];
  lookupFrameId: string;
  keys: KeyPair[];
  compatible: boolean;
  explicitlyUnique: boolean;
  membershipOnly: boolean;
  diagnostics: JoinDiagnostics | null;
  loading: boolean;
  error: string | null;
  onChooseLookup: (frameId: string) => void;
  onChangeKeys: (keys: KeyPair[]) => void;
  onMarkUnique: () => void;
}) {
  return (
    <>
      <div className="combine-row">
        <label>
          Match
          <strong>{primary.name}</strong>
        </label>
        <label>
          to
          <select
            aria-label="Other table"
            value={lookupFrameId}
            onChange={(event) => onChooseLookup(event.target.value)}
          >
            {candidates.map((frame) => (
              <option key={frame.id} value={frame.id}>
                {frame.name}
              </option>
            ))}
          </select>
        </label>
      </div>
      <MatchKeys
        primary={primary}
        lookup={lookup}
        keys={keys}
        onChange={onChangeKeys}
      />
      <MatchKeyStatus
        compatible={compatible}
        membershipOnly={membershipOnly}
        explicitlyUnique={explicitlyUnique}
        diagnostics={diagnostics}
        loading={loading}
        error={error}
        keyNames={keys
          .map((pair) => columnOf(lookup, pair.lookupColumnId)?.name ?? "Column")
          .join(" + ")}
        onMarkUnique={onMarkUnique}
      />
    </>
  );
}

function MatchKeepMode({
  primaryName,
  joinType,
  columnsOut,
  onChange,
}: {
  primaryName: string;
  joinType: FrameJoinType;
  columnsOut: number;
  onChange: (joinType: FrameJoinType) => void;
}) {
  return (
    <label className="combine-line">
      Keep
      <select
        aria-label="Keep mode"
        value={joinType}
        onChange={(event) => onChange(event.target.value as FrameJoinType)}
      >
        <option value="left">every {primaryName} row</option>
        <option value="inner">only matched rows</option>
        <option value="anti">rows without a match (anti)</option>
        <option value="semi">rows with a match (semi)</option>
      </select>
      <span className="combine-note">{columnsOut} columns out</span>
    </label>
  );
}
