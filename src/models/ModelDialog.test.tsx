// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ModelWorkbench } from "./ModelWorkbench";
import { ModelCreateButton } from "./ModelCreateButton";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "../test/support";
import type { Operation } from "../lib/types";

afterEach(() => { cleanup(); clearMocks(); });
const view = fixtures.salesBeforeFormula;
const frame = objectNamed(view, "frame", "Monthly sales");
const id = (name: string) => frame.columns.find(column => column.name === name)!.id;

function setup() {
  const run = vi.fn(async (_operation: Operation): Promise<string | null> => null);
  render(<ModelWorkbench document={view} onOperation={run}>
    <ModelCreateButton sourceFrameId={frame.id} x={100} y={200} />
  </ModelWorkbench>);
  return { run, user: userEvent.setup() };
}

describe("model authoring", () => {
  it("requires explicit features and emits the selected statistical specification", async () => {
    const { run, user } = setup();
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    expect((screen.getByRole("textbox", { name: "Model feature columns" }) as HTMLTextAreaElement).value).toBe("");
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Revenue");
    await user.selectOptions(screen.getByRole("combobox", { name: "Model target column" }), id("Cost"));
    await user.selectOptions(screen.getByRole("combobox", { name: "Model standard errors" }), "hc3");
    await user.click(screen.getByRole("button", { name: "Create model" }));
    expect(run).toHaveBeenCalledWith({ type: "addModel", name: "Regression", x: 100, y: 200,
      spec: { sourceFrameId: frame.id, targetColumnId: id("Cost"), featureColumnIds: [id("Revenue")],
        method: "ols", covariance: "hc3", confidenceLevel: 0.95, holdoutFraction: 0.2, seed: 42 } }, { inlineError: true });
  });

  it("refuses using the target as a feature before emitting an operation", async () => {
    const { run, user } = setup();
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Revenue");
    await user.selectOptions(screen.getByRole("combobox", { name: "Model target column" }), id("Revenue"));
    await user.click(screen.getByRole("button", { name: "Create model" }));
    expect(screen.getByRole("alert").textContent).toContain("target cannot also be a feature");
    expect(run).not.toHaveBeenCalled();
  });

  it("routes file and pasted XGBoost JSON through the same public operation", async () => {
    const { run, user } = setup();
    const contents = '{"learner":{"fixture":"operation payload only"}}';
    serveInvoke({ pick_model_file: () => ({ name: "trained.json", contents, bytes: null }) });
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Model method" }), "xgboost");
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Cost\nRevenue");
    await user.click(screen.getByRole("button", { name: "Choose model file…" }));
    await waitFor(() => expect((screen.getByRole("textbox", { name: "XGBoost model JSON" }) as HTMLTextAreaElement).value).toBe(contents));
    await user.click(screen.getByRole("button", { name: "Import model" }));
    expect(run).toHaveBeenCalledWith({ type: "importModel", name: "trained", sourceFrameId: frame.id,
      featureColumnIds: [id("Cost"), id("Revenue")], json: contents, x: 100, y: 200 }, { inlineError: true });
  });

  it("retains ONNX bytes and ordered mixed input mapping", async () => {
    const { run, user } = setup();
    const bytes = [8, 9, 18, 0]; // Opaque picker response; parsing belongs to native tests.
    serveInvoke({ pick_model_file: () => ({ name: "pipeline.onnx", contents: "", bytes }) });
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Model method" }), "onnx");
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Revenue\nCost\nRegion");
    await user.click(screen.getByRole("button", { name: "Choose model file…" }));
    await screen.findByText("pipeline.onnx");
    await user.click(screen.getByRole("button", { name: "Import model" }));
    expect(run).toHaveBeenCalledWith({ type: "importOnnxModel", name: "pipeline", sourceFrameId: frame.id,
      featureColumnIds: [id("Revenue"), id("Cost"), id("Region")], bytes, x: 100, y: 200 }, { inlineError: true });
  });

  it("uses the shared specification for a seeded random forest", async () => {
    const { run, user } = setup();
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Model method" }), "randomForestRegressor");
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Revenue");
    await user.selectOptions(screen.getByRole("combobox", { name: "Model target column" }), id("Cost"));
    await user.click(screen.getByRole("button", { name: "Create model" }));
    expect(run).toHaveBeenCalledWith(expect.objectContaining({ type: "addModel", spec: expect.objectContaining({
      method: "randomForestRegressor", forest: { trees: 100, maxDepth: 8, minSamplesLeaf: 2, maxFeatures: null, seed: 0 },
    }) }), { inlineError: true });
  });

  it("keeps an inline backend error and the editable draft after rejection", async () => {
    const { run, user } = setup();
    run.mockImplementationOnce(async () => "That model version is not supported");
    await user.click(screen.getByRole("button", { name: "Create model…" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Model method" }), "xgboost");
    await user.type(screen.getByRole("textbox", { name: "Model feature columns" }), "Revenue");
    await user.type(screen.getByRole("textbox", { name: "XGBoost model JSON" }), "unsupported");
    await user.click(screen.getByRole("button", { name: "Import model" }));
    expect(screen.getByRole("alert").textContent).toContain("model version is not supported");
    expect((screen.getByRole("textbox", { name: "XGBoost model JSON" }) as HTMLTextAreaElement).value).toBe("unsupported");
  });
});
