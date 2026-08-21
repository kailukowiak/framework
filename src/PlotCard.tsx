import { BarChart3, CircleAlert } from "lucide-react";
import embed, { type VisualizationSpec } from "vega-embed";
import { usePrefersDarkMode } from "./lib/palette";
import { useEffect, useMemo, useRef, useState } from "react";
import { getFramePage, type FramePage } from "./lib/api";
import type { OperationHandler } from "./lib/handlers";
import type { ComputedFrame, PlotObject, FrameObject } from "./lib/types";

function plotRows(
  frame: FrameObject,
  computed: ComputedFrame
): Array<Record<string, unknown>> {
  return frame.rows.map((row) =>
    Object.fromEntries(
      frame.columns.map((column) => {
        const value = computed.rows[row.id]?.[column.id]?.typedValue;
        if (!value || value.type === "null") return [column.id, null];
        return [column.id, value.value];
      })
    )
  );
}

function plotRowsFromPage(
  columns: FrameObject["columns"],
  page: FramePage
): Array<Record<string, unknown>> {
  return page.rows.map((row) =>
    Object.fromEntries(
      columns.map((column, index) => {
        const raw = row[index] ?? "";
        if (!raw) return [column.id, null];
        if (["integer", "number", "currency", "percentage"].includes(column.dataType)) {
          const number = Number(raw);
          return [column.id, Number.isFinite(number) ? number : null];
        }
        if (column.dataType === "boolean")
          return [column.id, raw.toLowerCase() === "true"];
        return [column.id, raw];
      })
    )
  );
}

// Vega-Lite draws its own SVG independent of the app's stylesheet, so a chart
// needs its own dark palette or it stays a bright white rectangle with
// unreadable black axis text against a dark card. Only the keys a plot
// doesn't already set are filled in, so a spec's own config/background --
// edited by hand in the Spec tab -- always wins.
const DARK_VEGA_CONFIG = {
  background: "transparent",
  title: { color: "#eeece2" },
  axis: {
    labelColor: "#b9b6a8",
    titleColor: "#eeece2",
    gridColor: "#3a3c30",
    domainColor: "#3a3c30",
    tickColor: "#3a3c30",
  },
  legend: { labelColor: "#b9b6a8", titleColor: "#eeece2" },
  view: { stroke: "#3a3c30" },
} as const;

function specObject(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function mergeVegaConfig(
  base: Record<string, unknown>,
  override: Record<string, unknown>
): Record<string, unknown> {
  const merged: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(override)) {
    const existing = merged[key];
    merged[key] =
      value && typeof value === "object" && !Array.isArray(value) &&
      existing && typeof existing === "object" && !Array.isArray(existing)
        ? { ...existing, ...value }
        : value;
  }
  return merged;
}

function VegaChart({
  plot,
  frame,
  computed,
}: {
  plot: PlotObject;
  frame: FrameObject;
  computed: ComputedFrame;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const isDark = usePrefersDarkMode();
  const [renderError, setRenderError] = useState<string | null>(null);
  const [fileRows, setFileRows] = useState<Array<Record<string, unknown>>>([]);
  const regularRows = useMemo(() => plotRows(frame, computed), [computed, frame]);
  const rows = computed.paged ? fileRows : regularRows;

  useEffect(() => {
    if (!computed.paged) return;
    let disposed = false;
    void getFramePage(frame.id, 0, 1000)
      .then((page) => {
        if (!disposed) setFileRows(plotRowsFromPage(frame.columns, page));
      })
      .catch((reason) => {
        if (!disposed) setRenderError(String(reason).replace(/^Error:\s*/, ""));
      });
    return () => {
      disposed = true;
    };
  }, [computed.paged, frame.id, frame.columns]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const host = window.document.createElement("div");
    host.className = "plot-embed-host";
    container.append(host);
    let disposed = false;
    let finalize: (() => void) | undefined;
    const spec = {
      ...plot.spec,
      data: { values: rows },
      width: plot.spec.width ?? "container",
      height: plot.spec.height ?? "container",
      autosize: plot.spec.autosize ?? {
        type: "fit",
        contains: "padding",
        resize: true,
      },
      ...(isDark
        ? {
            background: plot.spec.background ?? DARK_VEGA_CONFIG.background,
            config: mergeVegaConfig(
              DARK_VEGA_CONFIG,
              specObject(plot.spec.config)
            ),
          }
        : {}),
    } as VisualizationSpec;
    void embed(host, spec, {
      actions: {
        export: { png: true, svg: true },
        source: false,
        compiled: false,
        editor: false,
      },
      renderer: "svg",
      tooltip: true,
    })
      .then((result) => {
        if (disposed) result.finalize();
        else {
          finalize = result.finalize;
          setRenderError(null);
        }
      })
      .catch((reason) => {
        if (!disposed) setRenderError(String(reason).replace(/^Error:\s*/, ""));
      });
    return () => {
      disposed = true;
      finalize?.();
      host.remove();
    };
  }, [plot.spec, rows, isDark]);

  return (
    <div className="plot-visual-shell">
      <div className="plot-visual" ref={containerRef} />
      {renderError && (
        <div className="plot-render-error">
          <CircleAlert size={16} />
          {renderError}
        </div>
      )}
    </div>
  );
}

export function PlotCard({
  plot,
  frame,
  computed,
  onOperation,
}: {
  plot: PlotObject;
  frame: FrameObject;
  computed: ComputedFrame;
  onOperation: OperationHandler;
}) {
  return (
    <div className="plot-card">
      <div className="plot-title-row">
        <input
          className="frame-name"
          defaultValue={plot.name}
          key={plot.name}
          onBlur={(event) => {
            if (event.target.value !== plot.name)
              onOperation({
                type: "renameObject",
                objectId: plot.id,
                name: event.target.value,
              });
          }}
        />
        <span>
          <BarChart3 size={11} /> {frame.name} ·{" "}
          {(computed.totalRows ?? frame.rows.length).toLocaleString()} rows
        </span>
      </div>
      <VegaChart plot={plot} frame={frame} computed={computed} />
    </div>
  );
}
