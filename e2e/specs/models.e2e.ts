import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { readFileSync } from "node:fs";
import { documentJson, openTutorialsAndExamples, pointAtColumnCell, pressAndRelease, resetAndOpenTutorial } from "../lib/helpers";
import type { DocumentView } from "../../src/lib/types";

const data = "MLinput\tMLtarget\tMLaux\n1\t3.1\t0\n2\t4.8\t1\n3\t7.2\t0\n4\t8.9\t1\n5\t11.1\t0\n6\t12.8\t1\n7\t15.2\t0\n8\t16.9\t1\n9\t19.1\t0\n10\t20.8\t1\n11\t23.2\t0\n12\t24.9\t1";
const trainingFrameName = "ML Training Data";
const readDocument = async () => JSON.parse(await documentJson()) as DocumentView;
const fittedId = async () => {
  const model = (await readDocument()).objects.find(object => object.kind === "model" && object.name === "Simple regression");
  return model?.kind === "model" ? model.fitted?.id : undefined;
};
const firstPrediction = async (frameName = "ScoredData") => browser.execute((name: string) => {
  const field = Array.from(document.querySelectorAll<HTMLInputElement>('input[aria-label="Frame name"]')).find(input => input.value === name);
  const card = field?.closest(".frame-object");
  const column = card?.querySelector('[aria-label="Sort by Prediction"]')?.closest("th")?.getAttribute("data-column-id");
  return column ? card?.querySelector(`td[data-column-id="${column}"]`)?.textContent?.trim() ?? "" : "";
}, frameName);

async function waitForSelectOption(label: string, optionText: string): Promise<void> {
  const select = $(`[aria-label="${label}"]`);
  try {
    await browser.waitUntil(async () => {
      const options = await select.$$("option");
      return (await options.map(option => option.getText())).includes(optionText);
    }, { timeoutMsg: `${label} did not offer ${optionText}` });
  } catch (error) {
    const options = await select.$$("option");
    const available = await options.map(option => option.getText());
    throw new Error(`${label} did not offer ${optionText}; available options: ${available.join(" | ") || "<none>"}`, { cause: error });
  }
}

// The embedded WKWebView driver changes a select's DOM value without delivering
// the bubbling change event React listens for. Dispatch the events a native
// select gesture produces so the dialog state follows the visible control.
async function chooseSelectOption(label: string, match: { text?: string; value?: string }): Promise<void> {
  await browser.execute((accessibleName: string, wanted: { text?: string; value?: string }) => {
    const select = document.querySelector<HTMLSelectElement>(`select[aria-label="${accessibleName}"]`);
    const option = Array.from(select?.options ?? []).find(candidate => wanted.text !== undefined
      ? candidate.text === wanted.text : candidate.value === wanted.value);
    if (!select || !option) throw new Error(`${accessibleName} has no matching option`);
    select.value = option.value;
    select.dispatchEvent(new Event("input", { bubbles: true }));
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }, label, match);
}

async function waitForModelSourceColumns(columnNames: string[]): Promise<void> {
  try {
    await browser.waitUntil(() => browser.execute((names: string[]) => {
      const mappings = Array.from(document.querySelectorAll<HTMLElement>(".model-dialog .model-mapping"));
      const available = mappings.find(mapping => mapping.textContent?.startsWith("Available:"))?.textContent ?? "";
      return names.every(name => available.includes(name));
    }, columnNames), { timeoutMsg: `model dialog did not show source columns ${columnNames.join(", ")}` });
  } catch (error) {
    const available = await browser.execute(() => Array.from(document.querySelectorAll<HTMLElement>(".model-dialog .model-mapping"))
      .map(mapping => mapping.textContent ?? "").join(" | "));
    throw new Error(`model dialog did not show source columns ${columnNames.join(", ")}; mappings: ${available || "<none>"}`, { cause: error });
  }
}

describe("fitted models and live prediction frames", function () {
  this.bail(true);
  let fitId: string;
  let prediction: string;

  it("authors numerical training data through a normal frame paste", async () => {
    await resetAndOpenTutorial("The FrameWork tour — Start");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await pressAndRelease('[aria-label="Free-form data canvas"]');
    await browser.execute((text: string) => {
      const transfer = new DataTransfer(); transfer.setData("text/plain", text);
      (document.activeElement ?? window).dispatchEvent(new ClipboardEvent("paste", { clipboardData: transfer, bubbles: true, cancelable: true }));
    }, data);
    const inputHeader = $('[aria-label="Sort by MLinput"]');
    await inputHeader.waitForExist();
    await $('[aria-label="Sort by MLtarget"]').waitForExist();
    await $('[aria-label="Sort by MLaux"]').waitForExist();
    const name = $('//button[@aria-label="Sort by MLinput"]/ancestor::*[contains(concat(" ", normalize-space(@class), " "), " frame-object ")]//input[@aria-label="Frame name"]');
    await name.setValue(trainingFrameName);
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => {
      const frame = (await readDocument()).objects.find(object => object.kind === "frame" && object.name === trainingFrameName);
      return frame?.kind === "frame" && ["MLinput", "MLtarget", "MLaux"].every(columnName =>
        frame.columns.some(column => column.name === columnName));
    }, { timeoutMsg: `${trainingFrameName} document view did not contain all three pasted columns` });
  });

  it("creates an OLS model, explicitly fits HC3 and shows holdout evaluation", async () => {
    await $(".left-rail").$("button*=Model").click();
    await $('[role="dialog"][aria-label="Create model"]').waitForExist();
    await $('.model-dialog [aria-label="Model name"]').setValue("Simple regression");
    await chooseSelectOption("Model source frame", { text: trainingFrameName });
    await waitForModelSourceColumns(["MLinput", "MLtarget", "MLaux"]);
    await waitForSelectOption("Model target column", "MLtarget");
    expect(await $('[aria-label="Model feature columns"]').isExisting()).toBe(false);
    await chooseSelectOption("Model target column", { text: "MLtarget" });
    await chooseSelectOption("Model standard errors", { value: "hc3" });
    await $(".model-dialog").$("button=Create model").click();
    await $(".model-card").$("button=Fit model").waitForExist();
    await $(".model-card").$("button=Fit model").click();
    await $('table[aria-label="Model coefficients"]').waitForExist();
    await $('table[aria-label="Holdout evaluation metrics"]').waitForExist();
    await browser.waitUntil(async () => Boolean(await fittedId()));
    fitId = (await fittedId())!;
  });

  it("creates a live prediction frame from the saved fitted model", async () => {
    await $(".model-card").$("button=Predictions…").click();
    await $('[role="dialog"][aria-label="Create live predictions"]').waitForExist();
    await $('[aria-label="Prediction frame name"]').setValue("ScoredData");
    await $(".model-dialog").$("button=Create predictions").click();
    await $('[aria-label="Sort by Prediction"]').waitForExist();
    await browser.waitUntil(async () => Boolean(await firstPrediction()));
    prediction = await firstPrediction();
    expect(Number(prediction.replace(/[^\d.-]/g, ""))).toBeGreaterThan(1);
  });

  it("updates predictions after a source edit without retraining, then undoes the edit", async () => {
    await pointAtColumnCell("MLinput", 0);
    await browser.keys(Key.F2);
    const editor = $(".cell-editor"); await editor.waitForExist();
    await editor.setValue("20"); await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await firstPrediction()) !== prediction, { timeoutMsg: "prediction did not update after changing scoring input" });
    expect(await fittedId()).toBe(fitId);
    await $('.model-card [role="status"]').waitForExist();
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await firstPrediction()) === prediction);
    expect(await fittedId()).toBe(fitId);
  });

  it("reopens the saved tutorial with the same fitted revision and predictions", async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist();
    await openTutorialsAndExamples();
    await $(".dataset-dialog").$("button*=The FrameWork tour — Start").click();
    await $('table[aria-label="Model coefficients"]').waitForExist();
    await browser.waitUntil(async () => (await firstPrediction()) === prediction);
    expect(await fittedId()).toBe(fitId);
  });

  it("imports XGBoost JSON through the visible paste editor and creates predictions", async () => {
    await $(".left-rail").$("button*=Model").click();
    await $('.model-dialog [aria-label="Model name"]').setValue("Imported XGBoost");
    await chooseSelectOption("Model method", { value: "xgboost" });
    await chooseSelectOption("Model source frame", { text: trainingFrameName });
    await waitForModelSourceColumns(["MLinput", "MLtarget", "MLaux"]);
    await $('[aria-label="Model feature columns"]').setValue("MLinput\nMLtarget\nMLaux");
    const json = readFileSync(new URL("../../tools/ml-spikes/imports/fixtures/xgboost-regression.model.json", import.meta.url), "utf8");
    await $('[aria-label="XGBoost model JSON"]').setValue(json);
    await $(".model-dialog").$("button=Import model").click();
    const imported = $('.model-object:has(input[aria-label="Model name"][value="Imported XGBoost"])');
    await imported.waitForExist();
    await imported.$('button=Predictions…').click();
    await $('[aria-label="Prediction frame name"]').setValue("ScoredXgb");
    await $(".model-dialog").$("button=Create predictions").click();
    await browser.waitUntil(async () => Boolean(await firstPrediction("ScoredXgb")));
  });

  it("keeps imported weights fixed while scoring changes, then persists them on reopen", async () => {
    const importedId = async () => {
      const model = (await readDocument()).objects.find(object => object.kind === "model" && object.name === "Imported XGBoost");
      return model?.kind === "model" ? model.fitted?.id : undefined;
    };
    const revision = await importedId();
    expect(revision).toBeTruthy();
    const original = await firstPrediction("ScoredXgb");
    await pointAtColumnCell("MLinput", 0);
    await browser.keys(Key.F2);
    const editor = $(".cell-editor"); await editor.waitForExist();
    await editor.setValue("-20"); await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await firstPrediction("ScoredXgb")) !== original);
    expect(await importedId()).toBe(revision);
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await firstPrediction("ScoredXgb")) === original);
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist(); await openTutorialsAndExamples();
    await $(".dataset-dialog").$("button*=The FrameWork tour — Start").click();
    await browser.waitUntil(async () => (await firstPrediction("ScoredXgb")) === original);
    expect(await importedId()).toBe(revision);
  });

  it("allows deleting a model input, shows an inline error, and recovers on undo", async () => {
    const before = await firstPrediction();
    await browser.execute(() => {
      const header = document.querySelector('[aria-label="Sort by MLinput"]')?.closest("th");
      const column = header?.getAttribute("data-column-id");
      const cell = document.querySelector(`td[data-column-id="${column}"]`);
      if (!cell) throw new Error("Missing input column");
      cell.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
    });
    await $(".framework-context-menu").$("button=Delete column").click();
    await browser.waitUntil(async () => !(await $('[aria-label="Sort by MLinput"]').isExisting()));
    await $('.model-card [role="alert"]').waitForExist();
    expect(await fittedId()).toBe(fitId);
    await browser.keys([Key.Command, "z"]);
    await $('[aria-label="Sort by MLinput"]').waitForExist();
    await browser.waitUntil(async () => (await firstPrediction()) === before);
    await browser.waitUntil(async () => !(await $('.model-card [role="alert"]').isExisting()));
    expect(await fittedId()).toBe(fitId);
  });

  it("trains native XGBoost and retains its live predictions after undo and reopen", async () => {
    await $(".left-rail").$("button*=Model").click();
    await $('.model-dialog [aria-label="Model name"]').setValue("Native XGBoost");
    await chooseSelectOption("Model method", { value: "xgboostRegression" });
    await chooseSelectOption("Model source frame", { text: trainingFrameName });
    await waitForModelSourceColumns(["MLinput", "MLtarget", "MLaux"]);
    expect(await $('[aria-label="Model feature columns"]').isExisting()).toBe(false);
    await chooseSelectOption("Model target column", { text: "MLtarget" });
    await $(".model-dialog").$("button=Create model").click();
    const card = $('.model-object:has(input[aria-label="Model name"][value="Native XGBoost"])');
    await card.$("button=Fit model").click();
    await card.$('table[aria-label="Holdout evaluation metrics"]').waitForExist();
    const nativeFit = async () => {
      const model = (await readDocument()).objects.find(object => object.kind === "model" && object.name === "Native XGBoost");
      return model?.kind === "model" ? model.fitted?.id : undefined;
    };
    const revision = await nativeFit();
    expect(revision).toBeTruthy();
    await card.$("button=Predictions…").click();
    await $('[aria-label="Prediction frame name"]').setValue("NativeScores");
    await $(".model-dialog").$("button=Create predictions").click();
    await browser.waitUntil(async () => Boolean(await firstPrediction("NativeScores")));
    const original = await firstPrediction("NativeScores");
    await pointAtColumnCell("MLinput", 0);
    await browser.keys(Key.F2);
    const editor = $(".cell-editor"); await editor.waitForExist();
    await editor.setValue("12"); await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await firstPrediction("NativeScores")) !== original);
    expect(await nativeFit()).toBe(revision);
    await browser.keys([Key.Command, "z"]);
    await browser.waitUntil(async () => (await firstPrediction("NativeScores")) === original);
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist(); await openTutorialsAndExamples();
    await $(".dataset-dialog").$("button*=The FrameWork tour — Start").click();
    await browser.waitUntil(async () => (await firstPrediction("NativeScores")) === original);
    expect(await nativeFit()).toBe(revision);
  });

});
