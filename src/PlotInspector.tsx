import { BarChart3, Braces, CircleAlert, Play } from "lucide-react";
import { useEffect, useState } from "react";
import { Field } from "./Field";
import type { OperationHandler } from "./lib/handlers";
import type { Column, FrameObject, PlotObject, ValueObject } from "./lib/types";

type PlotEditorTab = "build" | "style" | "spec";

function vegaType(column: Column | undefined): "nominal" | "quantitative" | "temporal" {
  if (column?.dataType === "date") return "temporal";
  if (
    column &&
    ["integer", "number", "currency", "percentage"].includes(column.dataType)
  )
    return "quantitative";
  return "nominal";
}

function specObject(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function updatePlotSpec(
  plot: PlotObject,
  onOperation: OperationHandler,
  mutate: (spec: Record<string, unknown>) => void
) {
  const spec = structuredClone(plot.spec);
  mutate(spec);
  void onOperation({ type: "setPlotSpec", plotId: plot.id, spec });
}

export function PlotInspector({
  plot,
  frame,
  onOperation,
}: {
  plot: PlotObject;
  frame: FrameObject;
  onOperation: OperationHandler;
}) {
  const [tab, setTab] = useState<PlotEditorTab>("build");
  const [draft, setDraft] = useState(() => JSON.stringify(plot.spec, null, 2));
  const [specError, setSpecError] = useState<string | null>(null);
  useEffect(() => setDraft(JSON.stringify(plot.spec, null, 2)), [plot.spec]);
  const mark = specObject(plot.spec.mark);
  const markType =
    typeof plot.spec.mark === "string" ? plot.spec.mark : String(mark.type ?? "point");
  const encoding = specObject(plot.spec.encoding);
  const x = specObject(encoding.x);
  const y = specObject(encoding.y);
  const color = specObject(encoding.color);
  const config = specObject(plot.spec.config);
  const axis = specObject(config.axis);

  const setEncoding = (channel: "x" | "y" | "color", field: string) =>
    updatePlotSpec(plot, onOperation, (spec) => {
      const nextEncoding = specObject(spec.encoding);
      if (!field && channel === "color") delete nextEncoding.color;
      else {
        const column = frame.columns.find((candidate) => candidate.id === field);
        nextEncoding[channel] = {
          ...specObject(nextEncoding[channel]),
          field,
          type: vegaType(column),
          title: column?.name ?? field,
        };
      }
      spec.encoding = nextEncoding;
    });

  return (
    <div className="inspector-content plot-inspector">
      <div className="plot-editor-tabs" role="tablist">
        {(["build", "style", "spec"] as PlotEditorTab[]).map((candidate) => (
          <button
            key={candidate}
            className={tab === candidate ? "active" : ""}
            onClick={() => setTab(candidate)}
          >
            {candidate}
          </button>
        ))}
      </div>

      {tab === "build" && (
        <>
          <label className="inspector-field">
            Chart type
            <select
              value={markType}
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  spec.mark = {
                    ...specObject(spec.mark),
                    type: event.target.value,
                    tooltip: mark.tooltip ?? true,
                  };
                })
              }
            >
              <option value="bar">Bar</option>
              <option value="line">Line</option>
              <option value="area">Area</option>
              <option value="point">Scatter</option>
              <option value="rect">Heatmap</option>
              <option value="tick">Tick</option>
              <option value="boxplot">Box plot</option>
              <option value="rule">Rule</option>
            </select>
          </label>
          <label className="inspector-field">
            X axis
            <select
              value={String(x.field ?? "")}
              onChange={(event) => setEncoding("x", event.target.value)}
            >
              {frame.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
            </select>
          </label>
          <label className="inspector-field">
            Y axis
            <select
              value={String(y.field ?? "")}
              onChange={(event) => setEncoding("y", event.target.value)}
            >
              {frame.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
            </select>
          </label>
          <label className="inspector-field">
            Aggregate Y
            <select
              value={String(y.aggregate ?? "")}
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  const nextEncoding = specObject(spec.encoding);
                  const nextY = specObject(nextEncoding.y);
                  if (event.target.value) nextY.aggregate = event.target.value;
                  else delete nextY.aggregate;
                  nextEncoding.y = nextY;
                  spec.encoding = nextEncoding;
                })
              }
            >
              <option value="">None</option>
              <option value="sum">Sum</option>
              <option value="mean">Mean</option>
              <option value="median">Median</option>
              <option value="min">Minimum</option>
              <option value="max">Maximum</option>
              <option value="count">Count</option>
            </select>
          </label>
          <label className="inspector-field">
            Color / group
            <select
              value={String(color.field ?? "")}
              onChange={(event) => setEncoding("color", event.target.value)}
            >
              <option value="">None</option>
              {frame.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.name}
                </option>
              ))}
            </select>
          </label>
          <label className="plot-toggle">
            <input
              type="checkbox"
              checked={mark.tooltip !== false}
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  spec.mark = {
                    ...specObject(spec.mark),
                    type: markType,
                    tooltip: event.target.checked,
                  };
                })
              }
            />{" "}
            Tooltips
          </label>
          <label className="plot-toggle">
            <input
              type="checkbox"
              checked={
                Array.isArray(plot.spec.params) &&
                plot.spec.params.some(
                  (parameter) => specObject(parameter).name === "framework_zoom"
                )
              }
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  const params = Array.isArray(spec.params) ? [...spec.params] : [];
                  spec.params = event.target.checked
                    ? [
                        ...params.filter(
                          (parameter) => specObject(parameter).name !== "framework_zoom"
                        ),
                        { name: "framework_zoom", select: "interval", bind: "scales" },
                      ]
                    : params.filter(
                        (parameter) => specObject(parameter).name !== "framework_zoom"
                      );
                })
              }
            />{" "}
            Zoom and pan
          </label>
        </>
      )}

      {tab === "style" && (
        <>
          <Field
            label="Plot title"
            initial={typeof plot.spec.title === "string" ? plot.spec.title : ""}
            onCommit={(title) =>
              updatePlotSpec(plot, onOperation, (spec) => {
                if (title) spec.title = title;
                else delete spec.title;
              })
            }
          />
          <label className="inspector-field">
            Mark color
            <input
              type="color"
              value={typeof mark.color === "string" ? mark.color : "#26734d"}
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  spec.mark = {
                    ...specObject(spec.mark),
                    type: markType,
                    color: event.target.value,
                  };
                })
              }
            />
          </label>
          <label className="plot-toggle">
            <input
              type="checkbox"
              checked={axis.grid !== false}
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  const nextConfig = specObject(spec.config);
                  nextConfig.axis = {
                    ...specObject(nextConfig.axis),
                    grid: event.target.checked,
                  };
                  spec.config = nextConfig;
                })
              }
            />{" "}
            Grid lines
          </label>
          <label className="inspector-field">
            Background
            <input
              type="color"
              value={
                typeof plot.spec.background === "string"
                  ? plot.spec.background
                  : "#ffffff"
              }
              onChange={(event) =>
                updatePlotSpec(plot, onOperation, (spec) => {
                  spec.background = event.target.value;
                })
              }
            />
          </label>
          <div className="info-panel">
            <BarChart3 size={16} />
            <p>
              Use the Specification tab for layers, facets, scales, conditional
              encodings, and the complete Vega-Lite API.
            </p>
          </div>
        </>
      )}

      {tab === "spec" && (
        <>
          <label className="inspector-field">
            Vega-Lite JSON
            <textarea
              className="plot-spec-editor"
              value={draft}
              spellCheck={false}
              onChange={(event) => setDraft(event.target.value)}
            />
          </label>
          {specError && (
            <div className="formula-error">
              <CircleAlert size={14} />
              {specError}
            </div>
          )}
          <button
            className="primary-action"
            onClick={() => {
              try {
                const parsed = JSON.parse(draft);
                if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
                  throw new Error("The specification must be a JSON object");
                setSpecError(null);
                void onOperation(
                  { type: "setPlotSpec", plotId: plot.id, spec: parsed },
                  { inlineError: true }
                ).then((failure) => failure && setSpecError(failure));
              } catch (reason) {
                setSpecError(String(reason).replace(/^SyntaxError:\s*/, ""));
              }
            }}
          >
            <Play size={14} /> Apply specification
          </button>
          <p className="plot-spec-note">
            FrameWork supplies the source frame at render time, so the top-level data
            property is not persisted into the chart preview.
          </p>
        </>
      )}
    </div>
  );
}

export function ValueInspector({
  value,
  onOperation,
}: {
  value: ValueObject;
  onOperation: OperationHandler;
}) {
  return (
    <div className="inspector-content">
      <Field
        label="Name"
        initial={value.name}
        onCommit={(name) =>
          onOperation({ type: "renameObject", objectId: value.id, name })
        }
      />
      <Field
        label="Value"
        initial={value.raw}
        onCommit={(raw) => onOperation({ type: "setValue", objectId: value.id, raw })}
      />
      <div className="info-panel">
        <Braces size={16} />
        <p>
          This is a standalone scalar object. Frame formulas reference its stable ID, so
          renaming it is safe.
        </p>
      </div>
    </div>
  );
}
