import type { FrameObject, PlotObject } from "../lib/types";
import type { OperationHandler } from "../lib/handlers";
import { editablePlotSpec, intervalSettings, object, rebuildIntervalSpec } from "./intervalSpec";

export function IntervalControls({ plot, frame, onOperation }: {
  plot: PlotObject; frame: FrameObject; onOperation: OperationHandler;
}) {
  const settings = intervalSettings(plot.spec);
  const customLayers = Array.isArray(plot.spec.layer) && settings.version !== 1;
  const update = (key: string, value: string) => {
    const spec = editablePlotSpec(plot.spec);
    spec.usermeta = { ...object(spec.usermeta), frameworkInterval: {
      ...settings, version: 1, [key]: value,
    } };
    rebuildIntervalSpec(spec);
    void onOperation({ type: "setPlotSpec", plotId: plot.id, spec });
  };
  const numeric = frame.columns.filter(column =>
    ["integer", "number", "currency", "percentage"].includes(column.dataType));
  return <>
    <label className="inspector-field">Interval
      <select value={String(settings.style ?? "none")} disabled={customLayers}
        onChange={event => update("style", event.target.value)}>
        <option value="none">None</option><option value="band">Shaded band</option>
        <option value="whiskers">Whiskers</option>
      </select>
    </label>
    {customLayers && <p>Edit this layered chart in the Spec tab.</p>}
    {settings.style && settings.style !== "none" && <>
      <label className="inspector-field">Interval direction
        <select value={String(settings.axis ?? "y")} onChange={event => update("axis", event.target.value)}>
          <option value="y">Vertical (Y)</option><option value="x">Horizontal (X)</option>
        </select>
      </label>
      {(["lower", "upper"] as const).map(key => <label className="inspector-field" key={key}>
        {key === "lower" ? "Lower bound" : "Upper bound"}
        <select value={String(settings[key] ?? "")} onChange={event => update(key, event.target.value)}>
          <option value="">Choose column…</option>
          {numeric.map(column => <option key={column.id} value={column.id}>{column.name}</option>)}
        </select>
      </label>)}
    </>}
  </>;
}
