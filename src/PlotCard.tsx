import { BarChart3, CircleAlert } from "lucide-react";
import embed, { type VisualizationSpec } from "vega-embed";
import { expressionInterpreter } from "vega-interpreter";
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

// A paged frame's rows only exist as pages, so a plot has to pull them itself.
// Pull them all, not one page: a chart aggregates (a bar total, a mean, a
// line) over the rows it is handed, and handing it only the thousand rows the
// grid happens to be showing makes every aggregate wrong for the frame behind
// them. Pages come in large chunks to keep the round trips down, and stop at a
// ceiling so a multi-million-row frame cannot lock the SVG renderer -- when
// that ceiling is hit the chart says so rather than quietly plotting a slice.
const PLOT_PAGE_SIZE = 10000;
// Above this many rows, the cost of shipping every row to the browser just so
// Vega can reduce them there is worth a word to the user. It is a warning, not
// a cap: the chart still loads the whole frame. A plot that draws all four
// million rows and hangs is an honest sharp edge -- the fix is a Summarize
// step, which this note names -- not a silently truncated chart that lies by
// showing a slice.
const PLOT_AGG_WARN_ROWS = 50000;
// Above this many rows a paged frame does not auto-load when the card mounts;
// it waits for a click. This is the recovery hatch, not a data cap: a plot
// bound to a pathological frame would otherwise re-crash the session on every
// open, before the user could reach the frame to fix it. Holding the fetch
// until asked keeps the document openable; clicking still loads every row and
// may still hang -- the sharp edge is intact, it just needs consent.
const PLOT_AUTORENDER_ROWS = 200000;

// Whether the Vega-Lite spec reduces its data in the browser -- an encoding
// channel or transform carrying `aggregate` or `bin`. When it does, every raw
// row has to reach the chart only to be collapsed there; a Summarize step on
// the source frame does the same reduction in the engine, over the whole
// frame, for a fraction of the cost. We don't forbid the Vega path -- an
// Excel-style `y: {aggregate: "sum"}` bar chart is one line of spec and the
// convenient way to draw one -- we just say so once the frame is big enough to
// feel it.
function specAggregates(spec: Record<string, unknown>): boolean {
  let found = false;
  const visit = (node: unknown) => {
    if (found || !node || typeof node !== "object") return;
    if (Array.isArray(node)) {
      node.forEach(visit);
      return;
    }
    for (const [key, value] of Object.entries(node)) {
      if ((key === "aggregate" || key === "bin") && value) {
        found = true;
        return;
      }
      visit(value);
    }
  };
  visit(spec);
  return found;
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
  const [confirmedRender, setConfirmedRender] = useState(false);
  const regularRows = useMemo(() => plotRows(frame, computed), [computed, frame]);
  const rows = computed.paged ? fileRows : regularRows;
  const aggregates = useMemo(() => specAggregates(plot.spec), [plot.spec]);

  // A paged frame this large does not draw itself on open -- it waits behind a
  // click so a poisoned plot cannot re-crash the session before the user can
  // reach the frame. The gate is per-mount state, never persisted, so a fresh
  // open always starts closed and there is always a way back in.
  const totalRows = computed.totalRows ?? frame.rows.length;
  const gated =
    computed.paged && totalRows > PLOT_AUTORENDER_ROWS && !confirmedRender;

  // Reset the gate whenever the bound frame changes: the new frame's size, not
  // the last one's, decides whether it draws on its own.
  useEffect(() => {
    setConfirmedRender(false);
  }, [frame.id]);

  // Advisory only -- the chart draws the whole frame regardless. When it is
  // reducing a large frame in the browser that the engine could reduce first,
  // point at the cheaper path before the row count makes the chart crawl.
  const note =
    computed.paged && aggregates && rows.length >= PLOT_AGG_WARN_ROWS
      ? `Aggregating ${rows.length.toLocaleString()} rows in the chart. A Summarize step on the frame is faster for data this size.`
      : null;

  useEffect(() => {
    if (!computed.paged || gated) return;
    let disposed = false;
    async function loadAllRows() {
      const collected: Array<Record<string, unknown>> = [];
      let offset = 0;
      let total = Infinity;
      while (!disposed && offset < total) {
        const page = await getFramePage(frame.id, offset, PLOT_PAGE_SIZE);
        total = page.totalRows;
        if (page.rows.length === 0) break;
        collected.push(...plotRowsFromPage(frame.columns, page));
        offset += page.rows.length;
      }
      if (disposed) return;
      setFileRows(collected);
    }
    void loadAllRows().catch((reason) => {
      if (!disposed) setRenderError(String(reason).replace(/^Error:\s*/, ""));
    });
    return () => {
      disposed = true;
    };
    // Keyed on frame.id so an unrelated canvas edit that only changes the
    // frame object's identity doesn't re-scan every row of a large import.
    // `gated` is here so clicking Render (which clears it) starts the load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [computed.paged, frame.id, gated]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || gated) return;
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
      // Vega's default runtime compiles expressions (scale/field accessors,
      // signal handlers, ...) through `new Function(...)`, which is eval and
      // is blocked by our bundled CSP's `script-src` (no unsafe-eval, on
      // purpose -- see src-tauri/tauri.conf.json). `tauri dev` doesn't
      // enforce CSP at all, so a regression here renders fine at the desk
      // and only breaks in a real build; that gap is exactly why this
      // AST-walking interpreter is wired in unconditionally rather than
      // left as an opt-in fallback.
      ast: true,
      expr: expressionInterpreter,
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
  }, [plot.spec, rows, isDark, gated]);

  if (gated) {
    return (
      <div className="plot-visual-shell">
        <div className="plot-gate">
          <p>This chart plots {totalRows.toLocaleString()} rows.</p>
          <button
            className="secondary-action"
            onClick={() => setConfirmedRender(true)}
          >
            Render chart
          </button>
          {aggregates && (
            <small>
              A Summarize step on the frame draws it faster, and for far more
              rows.
            </small>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="plot-visual-shell">
      <div className="plot-visual" ref={containerRef} />
      {note && <div className="plot-render-note">{note}</div>}
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
