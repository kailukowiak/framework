// @vitest-environment jsdom
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ModelWorkbench } from "./ModelWorkbench";
import { ModelCard } from "./ModelCard";
import { fixtures, objectNamed } from "../test/support";
import type { Operation } from "../lib/types";

afterEach(cleanup);
const view = fixtures.mlModels;
const model = objectNamed(view, "model", "Demand");

function setup() {
  const run = vi.fn(async (_operation: Operation): Promise<string | null> => null);
  render(<ModelWorkbench document={view} onOperation={run}><ModelCard model={model} /></ModelWorkbench>);
  return { run, user: userEvent.setup() };
}

describe("saved model card", () => {
  it("renders generated training-fit coefficients separately from holdout metrics", () => {
    setup();
    const coefficients = screen.getByRole("table", { name: "Model coefficients" });
    expect(coefficients.textContent).toContain("HC3");
    expect(within(coefficients).getAllByRole("row")).toHaveLength(model.fitted!.result.summary.coefficients.length + 1);
    expect(screen.getByRole("table", { name: "Holdout evaluation metrics" }).textContent).toContain("6 rows");
    expect(screen.getByRole("table", { name: "Training metrics" }).textContent).toContain("24 rows");
    expect(screen.getByRole("button", { name: "Retrain" })).toBeTruthy();
  });

  it("requests a new fit explicitly and retains the old summary after a failed fit", async () => {
    const { run, user } = setup();
    run.mockResolvedValueOnce("Could not fit these data");
    await user.click(screen.getByRole("button", { name: "Retrain" }));
    expect(run).toHaveBeenCalledWith({ type: "fitModel", modelId: model.id }, { inlineError: true });
    expect(screen.getByRole("alert").textContent).toContain("Could not fit these data");
    expect(screen.getByRole("table", { name: "Model coefficients" })).toBeTruthy();
  });

  it("opens the shared mapping editor and emits a live prediction-frame operation", async () => {
    const { run, user } = setup();
    await user.click(screen.getByRole("button", { name: "Predictions…" }));
    const dialog = screen.getByRole("dialog", { name: "Create live predictions" });
    expect(within(dialog).getByText(/Model order:/).textContent).toContain(model.fitted!.result.featureNames[0]);
    await user.click(within(dialog).getByRole("button", { name: "Create predictions" }));
    expect(run).toHaveBeenCalledWith(expect.objectContaining({ type: "addModelPredictions", modelId: model.id,
      sourceFrameId: model.fitted!.spec!.sourceFrameId, featureColumnIds: model.fitted!.spec!.featureColumnIds }), { inlineError: true });
  });

  it("copies core-owned coefficient results to a referenceable frame", async () => {
    const { run, user } = setup();
    await user.click(screen.getByText("Copy stats to frame"));
    await user.click(screen.getByRole("button", { name: "Coefficients" }));
    expect(run).toHaveBeenCalledWith(expect.objectContaining({ type: "addModelSummary", modelId: model.id, kind: "coefficients" }), { inlineError: true });
  });
});
