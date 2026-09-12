// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { ModelWorkbench } from "./ModelWorkbench";
import { ModelCreateButton } from "./ModelCreateButton";
import { fixtures, objectNamed } from "../test/support";

afterEach(cleanup);

it("emits a two-column statistical analysis as a saved table", async () => {
  const view = fixtures.salesBeforeFormula;
  const frame = objectNamed(view, "frame", "Monthly sales");
  const id = (name: string) => frame.columns.find(column => column.name === name)!.id;
  const run = vi.fn(async () => null);
  const user = userEvent.setup();
  render(<ModelWorkbench document={view} onOperation={run}>
    <ModelCreateButton statistics sourceFrameId={frame.id} x={100} y={200} />
  </ModelWorkbench>);
  await user.click(screen.getByRole("button", { name: "Statistics…" }));
  await user.selectOptions(screen.getByRole("combobox", { name: "Statistical method" }), "pearsonCorrelation");
  await user.selectOptions(screen.getByRole("combobox", { name: "First column" }), id("Revenue"));
  await user.selectOptions(screen.getByRole("combobox", { name: "Second column" }), id("Cost"));
  await user.click(screen.getByRole("button", { name: "Create statistics table" }));
  expect(run).toHaveBeenCalledWith({ type: "addStatisticalAnalysis", name: "Statistics", sourceFrameId: frame.id,
    xColumnId: id("Revenue"), yColumnId: id("Cost"), request: { method: "pearsonCorrelation", confidenceLevel: 0.95 },
    x: 100, y: 200 }, { inlineError: true });
});
