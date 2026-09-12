type Spec = Record<string, unknown>;
export const object = (value: unknown): Spec =>
  value && typeof value === "object" && !Array.isArray(value) ? value as Spec : {};

export function intervalSettings(spec: Spec) {
  return object(object(spec.usermeta).frameworkInterval);
}

// Only unwrap layers authored by this editor. Arbitrary specifications belong
// to the Spec tab; interpreting their layer order would silently destroy work.
export function editablePlotSpec(source: Spec): Spec {
  const spec = structuredClone(source);
  if (intervalSettings(spec).version === 1 && Array.isArray(spec.layer)) {
    const main = object(spec.layer.at(-1));
    delete spec.layer;
    Object.assign(spec, main);
  }
  return spec;
}

export function rebuildIntervalSpec(spec: Spec) {
  const settings = intervalSettings(spec);
  if (!settings.lower || !settings.upper || settings.style === "none" || !settings.style) return;
  const axis = settings.axis === "x" ? "x" : "y";
  const encoding = object(spec.encoding);
  const main = { mark: spec.mark, encoding };
  // Bounds are observations already calculated by the frame. Aggregating only
  // the estimate would compare unlike rows and give the interval a false meaning.
  for (const channel of ["x", "y"]) {
    const entry = { ...object(encoding[channel]) };
    delete entry.aggregate;
    encoding[channel] = entry;
  }
  const bounds = {
    ...encoding,
    [axis]: { field: settings.lower, type: "quantitative", title: object(encoding[axis]).title },
    [`${axis}2`]: { field: settings.upper },
  };
  delete spec.mark;
  delete spec.encoding;
  const color = object(main.mark).color;
  spec.layer = [{
    mark: settings.style === "band"
      ? { type: "area", opacity: 0.18, orient: axis === "y" ? "vertical" : "horizontal", ...(color ? { color } : {}) }
      : { type: "errorbar", ticks: true, ...(color ? { color } : {}) },
    encoding: bounds,
  }, main];
}
