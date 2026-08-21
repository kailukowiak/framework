import type { CanvasView, DocumentView, FrameObject } from "./types";

/** A first, reasonable chart for a frame nobody has plotted before. */
export function defaultPlotSpec(frame: FrameObject): Record<string, unknown> {
  const quantitative = frame.columns.filter((column) =>
    ["integer", "number", "currency", "percentage"].includes(column.dataType)
  );
  const temporal = frame.columns.find((column) => column.dataType === "date");
  const categorical = frame.columns.find((column) =>
    ["string", "categorical", "boolean"].includes(column.dataType)
  );
  const x = temporal ?? categorical ?? frame.columns[0];
  const y = quantitative.find((column) => column.id !== x?.id) ?? quantitative[0];
  if (!x || !y) {
    const field = frame.columns[0]?.id;
    return {
      $schema: "https://vega.github.io/schema/vega-lite/v6.json",
      mark: { type: "bar", tooltip: true },
      encoding: field
        ? {
            x: { field, type: "nominal", title: frame.columns[0].name },
            y: { aggregate: "count", type: "quantitative", title: "Count" },
          }
        : {},
    };
  }
  const isTemporal = x.dataType === "date";
  const isScatter = !temporal && !categorical && quantitative.length >= 2;
  return {
    $schema: "https://vega.github.io/schema/vega-lite/v6.json",
    mark: isTemporal
      ? { type: "line", tooltip: true, point: true }
      : { type: isScatter ? "point" : "bar", tooltip: true },
    encoding: {
      x: {
        field: x.id,
        type: isTemporal ? "temporal" : isScatter ? "quantitative" : "nominal",
        title: x.name,
        sort: isTemporal || isScatter ? undefined : "-y",
      },
      y: {
        field: y.id,
        type: "quantitative",
        title: y.name,
        aggregate: isTemporal || isScatter ? undefined : "sum",
      },
      tooltip: [
        {
          field: x.id,
          type: isTemporal ? "temporal" : isScatter ? "quantitative" : "nominal",
          title: x.name,
        },
        { field: y.id, type: "quantitative", title: y.name },
      ],
    },
  };
}

/**
 * The card an object sits on, whether or not that card is showing it.
 *
 * A frame on a background tab is still on the canvas and its lineage is still
 * true, so anything asking "where is this" wants the card that holds it — not
 * the narrower question of which card currently has it selected.
 */
export function viewHolding(
  document: DocumentView,
  objectId: string
): CanvasView | undefined {
  return (
    document.views.find((view) => view.objectId === objectId) ??
    document.views.find((view) => view.tabObjectIds?.includes(objectId))
  );
}
