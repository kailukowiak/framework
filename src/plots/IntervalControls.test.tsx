// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { compile } from "vega-lite";
import { PlotInspector } from "../PlotInspector";
import { fixtures, objectNamed } from "../test/support";
import type { PlotObject } from "../lib/types";
import { editablePlotSpec, object } from "./intervalSpec";

afterEach(cleanup);
it("authors bounds, preserves grouping and style edits, and removes the overlay", async () => {
  const frame = objectNamed(fixtures.salesBeforeFormula, "frame", "Monthly sales");
  const column = (name: string) => frame.columns.find(c => c.name === name)!.id;
  let plot: PlotObject = { kind: "plot", id: frame.id, name: "Plot", sourceFrameId: frame.id, spec: {
    mark: "line", encoding: { x: { field: column("Revenue"), type: "quantitative" },
      y: { field: column("Cost"), type: "quantitative" },
      color: { field: frame.columns[0].id, type: "nominal" } },
  } };
  const run = vi.fn(async (operation) => {
    plot = { ...plot, spec: operation.spec };
    mounted.rerender(<PlotInspector plot={plot} frame={frame} onOperation={run} />);
    return null;
  });
  const mounted = render(<PlotInspector plot={plot} frame={frame} onOperation={run} />);
  const user = userEvent.setup();
  const select = (name: string, value: string) => user.selectOptions(screen.getByRole("combobox", { name }), value);
  await select("Interval", "band");
  await select("Lower bound", column("Cost"));
  await select("Upper bound", column("Revenue"));
  const layers = () => plot.spec.layer as Array<Record<string, unknown>>;
  expect(object(layers()[0].mark).type).toBe("area");
  expect(object(layers()[0].encoding).color).toEqual(object(layers()[1].encoding).color);
  expect(() => compile(plot.spec as unknown as Parameters<typeof compile>[0])).not.toThrow();
  await select("Chart type", "point");
  expect(object(layers()[1].mark).type).toBe("point");
  await select("Interval", "whiskers");
  await select("Interval direction", "x");
  expect(object(object(layers()[0].encoding).x2).field).toBe(column("Revenue"));
  expect(() => compile(plot.spec as unknown as Parameters<typeof compile>[0])).not.toThrow();
  await user.click(screen.getByRole("button", { name: "style" }));
  await user.click(screen.getByRole("button", { name: "build" }));
  await select("Interval", "none");
  expect(plot.spec.layer).toBeUndefined();
  expect(object(editablePlotSpec(plot.spec).mark).type).toBe("point");
});
