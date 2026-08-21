import { CircleAlert, GitMerge, KeyRound, X } from "lucide-react";
import { useState } from "react";
import type { OperationHandler } from "./lib/handlers";
import type {
  ComputedFrame,
  DataType,
  DocumentView,
  FrameJoinType,
  FrameObject,
} from "./lib/types";
import type { JoinState } from "./App";

function normalizedKeyName(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "")
    .replace(/identifier$/, "id");
}

function joinKeyValue(
  computed: ComputedFrame | undefined,
  rowId: string,
  columnId: string
): string | null {
  const value = computed?.rows[rowId]?.[columnId]?.typedValue;
  if (!value || value.type === "null") return null;
  return JSON.stringify(value);
}

function observedKeyStats(
  frame: FrameObject | undefined,
  computed: ComputedFrame | undefined,
  columnId: string
) {
  if (!frame || !columnId) return { distinct: 0, duplicates: 0, nulls: 0 };
  const seen = new Set<string>();
  let duplicates = 0;
  let nulls = 0;
  for (const row of frame.rows) {
    const value = joinKeyValue(computed, row.id, columnId);
    const key = value ?? "__framework_null_key__";
    if (value == null) nulls += 1;
    if (seen.has(key)) duplicates += 1;
    seen.add(key);
  }
  return { distinct: seen.size, duplicates, nulls };
}

export function JoinDialog({
  state,
  document,
  onClose,
  onOperation,
  onCreated,
}: {
  state: NonNullable<JoinState>;
  document: DocumentView;
  onClose: () => void;
  onOperation: OperationHandler;
  onCreated: () => void;
}) {
  const frames = document.objects.filter(
    (object): object is FrameObject => object.kind === "frame"
  );
  const primary = frames.find((frame) => frame.id === state.primaryFrameId)!;
  const candidates = frames.filter((frame) => frame.id !== primary.id);
  const initialLookup =
    candidates.find((frame) => frame.uniqueKeys.length > 0) ?? candidates[0];
  const initialLookupKey =
    initialLookup?.uniqueKeys[0]?.columnIds[0] ?? initialLookup?.columns[0]?.id ?? "";
  const initialLookupColumn = initialLookup?.columns.find(
    (column) => column.id === initialLookupKey
  );
  const initialPrimaryKey =
    primary.columns.find(
      (column) =>
        normalizedKeyName(column.name) ===
        normalizedKeyName(initialLookupColumn?.name ?? "")
    )?.id ??
    primary.columns[0]?.id ??
    "";
  const [lookupFrameId, setLookupFrameId] = useState(initialLookup?.id ?? "");
  const [primaryKeyId, setPrimaryKeyId] = useState(initialPrimaryKey);
  const [lookupKeyId, setLookupKeyId] = useState(initialLookupKey);
  const [joinType, setJoinType] = useState<FrameJoinType>("left");
  const [name, setName] = useState(
    initialLookup ? `${primary.name} + ${initialLookup.name}` : `${primary.name} joined`
  );
  const [selected, setSelected] = useState<Set<string>>(
    () =>
      new Set([
        ...primary.columns.map((column) => `${primary.id}:${column.id}`),
        ...(initialLookup?.columns
          .filter((column) => column.id !== initialLookupKey)
          .map((column) => `${initialLookup.id}:${column.id}`) ?? []),
      ])
  );
  const [joinError, setJoinError] = useState<string | null>(null);
  const membershipOnly = joinType === "anti" || joinType === "semi";
  const lookup = frames.find((frame) => frame.id === lookupFrameId);
  const primaryKey = primary.columns.find((column) => column.id === primaryKeyId);
  const lookupKey = lookup?.columns.find((column) => column.id === lookupKeyId);
  const lookupIsExplicitlyUnique = Boolean(
    lookup?.uniqueKeys.some(
      (key) => key.columnIds.length === 1 && key.columnIds[0] === lookupKeyId
    )
  );
  const lookupStats = observedKeyStats(
    lookup,
    lookup ? document.computedFrames[lookup.id] : undefined,
    lookupKeyId
  );
  const numericTypes = new Set<DataType>([
    "integer",
    "number",
    "currency",
    "percentage",
  ]);
  const compatible = Boolean(
    primaryKey &&
      lookupKey &&
      (primaryKey.dataType === lookupKey.dataType ||
        (numericTypes.has(primaryKey.dataType) && numericTypes.has(lookupKey.dataType)))
  );
  const lookupValues = new Set(
    (lookup?.rows ?? [])
      .map((row) =>
        joinKeyValue(document.computedFrames[lookup!.id], row.id, lookupKeyId)
      )
      .filter((value): value is string => value != null)
  );
  let matched = 0;
  let unmatched = 0;
  for (const row of primary.rows) {
    const value = joinKeyValue(
      document.computedFrames[primary.id],
      row.id,
      primaryKeyId
    );
    if (value != null && lookupValues.has(value)) matched += 1;
    else unmatched += 1;
  }
  const outputInputs = (membershipOnly ? [primary] : [primary, lookup])
    .filter((frame): frame is FrameObject => Boolean(frame))
    .flatMap((frame) =>
      frame.columns
        .filter((column) => selected.has(`${frame.id}:${column.id}`))
        .map((column) => {
          const duplicateName =
            frame.id === lookup?.id &&
            primary.columns.some(
              (candidate) =>
                candidate.name === column.name &&
                selected.has(`${primary.id}:${candidate.id}`)
            );
          return {
            sourceFrameId: frame.id,
            sourceColumnId: column.id,
            name: duplicateName ? `${frame.name} ${column.name}` : column.name,
          };
        })
    );

  const chooseLookup = (frameId: string) => {
    const next = frames.find((frame) => frame.id === frameId)!;
    const nextLookupKey = next.uniqueKeys[0]?.columnIds[0] ?? next.columns[0]?.id ?? "";
    const nextLookupColumn = next.columns.find((column) => column.id === nextLookupKey);
    const nextPrimaryKey =
      primary.columns.find(
        (column) =>
          normalizedKeyName(column.name) ===
          normalizedKeyName(nextLookupColumn?.name ?? "")
      )?.id ??
      primary.columns[0]?.id ??
      "";
    setLookupFrameId(frameId);
    setLookupKeyId(nextLookupKey);
    setPrimaryKeyId(nextPrimaryKey);
    setName(`${primary.name} + ${next.name}`);
    setSelected(
      new Set([
        ...primary.columns.map((column) => `${primary.id}:${column.id}`),
        ...(membershipOnly
          ? []
          : next.columns
              .filter((column) => column.id !== nextLookupKey)
              .map((column) => `${next.id}:${column.id}`)),
      ])
    );
    setJoinError(null);
  };

  if (candidates.length === 0) {
    return (
      <div className="dialog-backdrop">
        <div className="insert-dialog join-dialog">
          <div className="dialog-header">
            <div>
              <span className="eyebrow">JOIN FRAMES</span>
              <h2>Add another frame first</h2>
            </div>
            <button className="icon-button" onClick={onClose}>
              <X size={18} />
            </button>
          </div>
          <p className="empty-transform-note">A join needs two frames on the canvas.</p>
          <div className="dialog-actions">
            <button className="secondary-action" onClick={onClose}>
              Close
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className="dialog-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="insert-dialog join-dialog">
        <div className="dialog-header">
          <div>
            <span className="eyebrow">JOIN FRAMES</span>
            <h2>Bring columns into {primary.name}</h2>
          </div>
          <button className="icon-button" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <div className="join-equation">
          <div>
            <small>Starting frame</small>
            <strong>{primary.name}</strong>
            <select
              value={primaryKeyId}
              onChange={(event) => setPrimaryKeyId(event.target.value)}
            >
              {primary.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
            </select>
          </div>
          <GitMerge size={20} />
          <div>
            <small>Lookup frame</small>
            <select
              value={lookupFrameId}
              onChange={(event) => chooseLookup(event.target.value)}
            >
              {candidates.map((frame) => (
                <option key={frame.id} value={frame.id}>
                  {frame.name}
                </option>
              ))}
            </select>
            <select
              value={lookupKeyId}
              onChange={(event) => {
                setLookupKeyId(event.target.value);
                setJoinError(null);
              }}
            >
              {lookup?.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                  {lookup?.uniqueKeys.some(
                    (key) =>
                      key.columnIds.length === 1 && key.columnIds[0] === column.id
                  )
                    ? " · Unique"
                    : ""}
                </option>
              ))}
            </select>
          </div>
        </div>
        {!compatible && (
          <div className="join-warning">
            <CircleAlert size={14} /> Choose columns with compatible types.
          </div>
        )}
        {!membershipOnly && !lookupIsExplicitlyUnique && (
          <div
            className={`join-key-callout ${lookupStats.duplicates ? "invalid" : ""}`}
          >
            <KeyRound size={16} />
            <div>
              <strong>
                {lookupStats.duplicates
                  ? `${lookupStats.duplicates} duplicate key value${
                      lookupStats.duplicates === 1 ? "" : "s"
                    }`
                  : `${lookupKey?.name ?? "Column"} appears unique`}
              </strong>
              <span>
                FrameWork requires the lookup side to be an enforced unique key.
              </span>
            </div>
            <button
              disabled={lookupStats.duplicates > 0}
              onClick={() =>
                void onOperation(
                  {
                    type: "setUniqueKey",
                    frameId: lookup!.id,
                    columnIds: [lookupKeyId],
                    enabled: true,
                  },
                  { inlineError: true }
                ).then((failure) => setJoinError(failure))
              }
            >
              Mark unique
            </button>
          </div>
        )}
        {!membershipOnly && lookupIsExplicitlyUnique && (
          <div className="join-key-callout valid">
            <KeyRound size={16} />
            <div>
              <strong>Unique lookup key</strong>
              <span>Duplicate matches cannot silently multiply rows.</span>
            </div>
          </div>
        )}
        <div className="join-preview">
          <div>
            <strong>{matched.toLocaleString()}</strong>
            <span>matched</span>
          </div>
          <div>
            <strong>{unmatched.toLocaleString()}</strong>
            <span>unmatched</span>
          </div>
          <div>
            <strong>{lookupStats.duplicates.toLocaleString()}</strong>
            <span>duplicates</span>
          </div>
        </div>
        <label className="join-keep-mode">
          Keep{" "}
          <select
            value={joinType}
            onChange={(event) => {
              const next = event.target.value as FrameJoinType;
              setJoinType(next);
              if (next === "anti" || next === "semi")
                setSelected(
                  (current) =>
                    new Set(
                      Array.from(current).filter((key) =>
                        key.startsWith(`${primary.id}:`)
                      )
                    )
                );
            }}
          >
            <option value="left">every {primary.name} row</option>
            <option value="inner">only matched rows</option>
            <option value="anti">rows without a match (anti)</option>
            <option value="semi">rows with a match (semi)</option>
          </select>
        </label>
        <div className="join-output-heading">
          <strong>Columns in the new frame</strong>
          <span>
            {outputInputs.length} selected
            {membershipOnly ? ` · ${primary.name} columns only` : ""}
          </span>
        </div>
        <div className="join-columns">
          {(membershipOnly ? [primary] : [primary, lookup!]).map((frame) => (
            <div key={frame.id}>
              <strong>{frame.name}</strong>
              {frame.columns.map((column) => {
                const key = `${frame.id}:${column.id}`;
                return (
                  <label key={column.id}>
                    <input
                      type="checkbox"
                      checked={selected.has(key)}
                      onChange={(event) =>
                        setSelected((current) => {
                          const next = new Set(current);
                          if (event.target.checked) next.add(key);
                          else next.delete(key);
                          return next;
                        })
                      }
                    />
                    <span>{column.name}</span>
                    <small>{column.dataType}</small>
                  </label>
                );
              })}
            </div>
          ))}
        </div>
        <label>
          Result name
          <input value={name} onChange={(event) => setName(event.target.value)} />
        </label>
        {joinError && (
          <div className="formula-editor-error">
            <CircleAlert size={12} />
            <span>{joinError}</span>
          </div>
        )}
        <div className="dialog-actions">
          <button className="secondary-action" onClick={onClose}>
            Cancel
          </button>
          <button
            className="primary-action"
            disabled={
              !name.trim() ||
              !lookup ||
              !compatible ||
              (!membershipOnly &&
                (!lookupIsExplicitlyUnique || lookupStats.duplicates > 0)) ||
              outputInputs.length === 0
            }
            onClick={() =>
              void onOperation(
                {
                  type: "addJoinFrame",
                  primaryFrameId: primary.id,
                  lookupFrameId: lookup!.id,
                  primaryKeyColumnIds: [primaryKeyId],
                  lookupKeyColumnIds: [lookupKeyId],
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
          >
            Create joined frame <GitMerge size={15} />
          </button>
        </div>
      </div>
    </div>
  );
}
