import { CircleAlert } from "lucide-react";
import { useState } from "react";
import {
  frameRowCount,
  pairColumnSteps,
  pairFillRefusal,
  stackMapping,
} from "./lib/combineChain";
import type {
  ComputedFrame,
  FrameObject,
  FrameStepInput,
  ZipFill,
} from "./lib/types";

/**
 * The three relationships that append a step to the frame's own chain rather
 * than creating a new frame. They share the same skeleton — pick the other
 * table, see what the step would do, Create — so they share the frame picker
 * and the save path, and differ only in the preview between them.
 */
function OtherFramePicker({
  value,
  candidates,
  onChange,
}: {
  value: string;
  candidates: FrameObject[];
  onChange: (frameId: string) => void;
}) {
  return (
    <label className="combine-line">
      With
      <select
        aria-label="Other table"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        {candidates.map((frame) => (
          <option key={frame.id} value={frame.id}>
            {frame.name}
          </option>
        ))}
      </select>
    </label>
  );
}

function CreateRow({
  label,
  refusal,
  error,
  onCreate,
}: {
  label: string;
  refusal: string | null;
  error: string | null;
  onCreate: () => void;
}) {
  return (
    <>
      {refusal && <p className="combine-refusal">{refusal}</p>}
      {error && (
        <p className="formula-editor-error">
          <CircleAlert size={12} /> {error}
        </p>
      )}
      <div className="dialog-actions">
        <button className="primary-action" disabled={Boolean(refusal)} onClick={onCreate}>
          {label}
        </button>
      </div>
    </>
  );
}

const rowText = (count: number | undefined) =>
  count === undefined ? "—" : `${count.toLocaleString()} rows`;

/**
 * *Same rows in order*: one `zipVector` step per column brought across.
 *
 * The fill choice is the whole decision the pair step needed and never had.
 * `Exact` is the honest default — one value per row, and a mismatch is a
 * mistake worth refusing. `Repeat to fill` is the thing Kai asked for by
 * name: five rates against sixty months, tiled.
 */
export function CombinePairForm({
  primary,
  primaryComputed,
  candidates,
  computedFrames,
  onSave,
}: {
  primary: FrameObject;
  primaryComputed: ComputedFrame | undefined;
  candidates: FrameObject[];
  computedFrames: Record<string, ComputedFrame | undefined>;
  onSave: (steps: FrameStepInput[]) => Promise<string | null>;
}) {
  const [otherId, setOtherId] = useState(candidates[0]?.id ?? "");
  const other = candidates.find((frame) => frame.id === otherId);
  const [chosen, setChosen] = useState<Set<string>>(
    () => new Set(candidates[0]?.columns.map((column) => column.id) ?? [])
  );
  const [fill, setFill] = useState<ZipFill>("exact");
  const [error, setError] = useState<string | null>(null);
  const primaryRows = frameRowCount(primary, primaryComputed);
  const otherRows = other
    ? frameRowCount(other, computedFrames[other.id])
    : undefined;
  const refusal =
    !other || chosen.size === 0
      ? "Choose at least one column to bring across."
      : pairFillRefusal(primaryRows, otherRows, fill);

  return (
    <>
      <OtherFramePicker
        value={otherId}
        candidates={candidates}
        onChange={(frameId) => {
          setOtherId(frameId);
          setChosen(
            new Set(
              candidates
                .find((frame) => frame.id === frameId)
                ?.columns.map((column) => column.id) ?? []
            )
          );
          setError(null);
        }}
      />
      <div className="combine-counts">
        <span>
          {primary.name} {rowText(primaryRows)}
        </span>
        <span>
          {other?.name ?? "—"} {rowText(otherRows)}
        </span>
      </div>
      <label className="combine-line">
        Fill
        <select
          aria-label="Fill"
          value={fill}
          onChange={(event) => setFill(event.target.value as ZipFill)}
        >
          <option value="exact">Exact</option>
          <option value="repeat">Repeat to fill</option>
        </select>
      </label>
      <div className="combine-checklist">
        {other?.columns.map((column) => (
          <label key={column.id}>
            <input
              type="checkbox"
              checked={chosen.has(column.id)}
              onChange={(event) =>
                setChosen((current) => {
                  const next = new Set(current);
                  if (event.target.checked) next.add(column.id);
                  else next.delete(column.id);
                  return next;
                })
              }
            />
            <span>{column.name}</span>
            <small>{column.dataType}</small>
          </label>
        ))}
      </div>
      <CreateRow
        label="Add columns"
        refusal={refusal}
        error={error}
        onCreate={() =>
          void onSave(
            pairColumnSteps(
              other!,
              other!.columns
                .filter((column) => chosen.has(column.id))
                .map((column) => column.id),
              fill,
              primary.columns.map((column) => column.name),
              otherRows ?? 1
            )
          ).then(setError)
        }
      />
    </>
  );
}

/**
 * *Rows under rows*: a `union` step. The core resolves the column mapping by
 * name at save time and keeps ids afterwards, so the only useful thing the
 * dialog can do is show that resolution before it is frozen — which columns
 * line up, which take nulls, which are dropped, and where the types disagree.
 */
export function CombineStackForm({
  primary,
  candidates,
  onSave,
}: {
  primary: FrameObject;
  candidates: FrameObject[];
  onSave: (steps: FrameStepInput[]) => Promise<string | null>;
}) {
  const [otherId, setOtherId] = useState(candidates[0]?.id ?? "");
  const other = candidates.find((frame) => frame.id === otherId);
  const [error, setError] = useState<string | null>(null);
  const { pairs, notCarried } = stackMapping(
    primary.columns,
    other?.columns ?? []
  );

  return (
    <>
      <OtherFramePicker
        value={otherId}
        candidates={candidates}
        onChange={(frameId) => {
          setOtherId(frameId);
          setError(null);
        }}
      />
      <div className="combine-mapping">
        {pairs.map((pair) => (
          <div key={pair.columnId}>
            <span>{pair.name}</span>
            <span className={pair.sourceName ? "" : "combine-blank"}>
              {pair.sourceName ?? "— (blank)"}
              {pair.mismatch && <em className="combine-mismatch"> mismatch</em>}
            </span>
          </div>
        ))}
      </div>
      {notCarried.length > 0 && (
        <p className="combine-note">not carried: {notCarried.join(", ")}</p>
      )}
      <CreateRow
        label="Stack rows"
        refusal={other ? null : "Choose a table to stack."}
        error={error}
        onCreate={() =>
          void onSave([{ kind: "union", frameId: other!.id }]).then(setError)
        }
      />
    </>
  );
}

/** *Every row with every row*: an `expand` step, with the count it produces. */
export function CombineExpandForm({
  primary,
  primaryComputed,
  candidates,
  computedFrames,
  onSave,
}: {
  primary: FrameObject;
  primaryComputed: ComputedFrame | undefined;
  candidates: FrameObject[];
  computedFrames: Record<string, ComputedFrame | undefined>;
  onSave: (steps: FrameStepInput[]) => Promise<string | null>;
}) {
  const [otherId, setOtherId] = useState(candidates[0]?.id ?? "");
  const other = candidates.find((frame) => frame.id === otherId);
  const [error, setError] = useState<string | null>(null);
  const primaryRows = frameRowCount(primary, primaryComputed);
  const otherRows = other
    ? frameRowCount(other, computedFrames[other.id])
    : undefined;

  return (
    <>
      <OtherFramePicker
        value={otherId}
        candidates={candidates}
        onChange={(frameId) => {
          setOtherId(frameId);
          setError(null);
        }}
      />
      <p className="combine-product">
        {primaryRows === undefined || otherRows === undefined
          ? "Row count unknown until both tables are read"
          : `${primaryRows.toLocaleString()} × ${otherRows.toLocaleString()} = ${(
              primaryRows * otherRows
            ).toLocaleString()} rows`}
      </p>
      <CreateRow
        label="Expand rows"
        refusal={other ? null : "Choose a table to expand with."}
        error={error}
        onCreate={() =>
          void onSave([{ kind: "expand", frameId: other!.id }]).then(setError)
        }
      />
    </>
  );
}
