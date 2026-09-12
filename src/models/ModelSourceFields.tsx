import type { FrameObject } from "../lib/types";

export function ModelSourceFields({ frames, sourceId, onSource, features, onFeatures, featureNames }: {
  frames: FrameObject[]; sourceId: string; onSource: (id: string) => void;
  features: string; onFeatures: (value: string) => void; featureNames?: string[];
}) {
  const source = frames.find(frame => frame.id === sourceId);
  return <>
    <label>Source frame<select aria-label="Model source frame" value={sourceId} onChange={event => onSource(event.target.value)}>
      <option value="">Choose a frame</option>
      {frames.map(frame => <option key={frame.id} value={frame.id}>{frame.name || "Unnamed frame"}</option>)}
    </select></label>
    <label>Features — one column name per line, in order
      <textarea aria-label="Model feature columns" value={features} rows={4} spellCheck={false}
        placeholder={source?.columns.slice(0, 3).map(column => column.name).join("\n") ?? "Revenue\nCost"}
        onChange={event => onFeatures(event.target.value)} />
    </label>
    {featureNames && <p className="model-mapping">Model order: {featureNames.join(" · ")}</p>}
    {source && <p className="model-mapping">Available: {source.columns.map(column => column.name).join(" · ")}</p>}
  </>;
}
