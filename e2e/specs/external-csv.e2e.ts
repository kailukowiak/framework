import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { browser, $, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import { columnCellTexts } from "../lib/helpers";

describe("External CSV opening", () => {
  it("opens Finder data in a new window without changing the source", async () => {
    const directory = mkdtempSync(join(tmpdir(), "framework-finder-csv-"));
    const path = join(directory, "External orders.csv");
    const original = "SKU,Quantity\n0002,3\n0003,5\n";
    writeFileSync(path, original);
    try {
      const previous = await browser.getWindowHandles();
      const app = resolve("target/debug/bundle/macos/FrameWork.app");
      execFileSync("open", ["-a", app, path]);
      await browser.waitUntil(async () => (await browser.getWindowHandles()).length > previous.length);
      const handles = await browser.getWindowHandles();
      const opened = handles.find((handle) => !previous.includes(handle));
      if (!opened) throw new Error("CSV window did not open");
      await browser.switchToWindow(opened);
      await $('button[aria-label="Sort by SKU"]').waitForExist();
      await expect($("div.cell-display=0002")).toBeDisplayed();
      expect((await columnCellTexts("Quantity")).slice(0, 2).map(Number)).toEqual([3, 5]);
      expect(readFileSync(path, "utf8")).toBe(original);

      const output = join(directory, "Written orders.csv");
      await browser.execute((outputPath: string) => {
        // The save dialog cannot be driven by WebDriver. The command accepts
        // the path the dialog would return so this spec can still prove the
        // native boundary: computed values are written, the output is read
        // back, and the working recipe is not consumed.
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const w = window as any;
        w.__e2eWriteOut = undefined;
        w.__TAURI__.core
          .invoke("get_document")
          .then((view: { objects: Array<{ id: string; kind: string }> }) => {
            const frame = view.objects.find((object) => object.kind === "frame");
            if (!frame) throw new Error("Imported frame was missing");
            return w.__TAURI__.core
              .invoke("apply_operation", {
                operation: {
                  type: "setFramePipeline",
                  frameId: frame.id,
                  steps: [
                    {
                      kind: "filter",
                      predicates: ["`Quantity` > 3"],
                      matchAll: true,
                    },
                  ],
                },
              })
              .then(() =>
                w.__TAURI__.core.invoke("export_frame_as", {
                  frameId: frame.id,
                  path: outputPath,
                })
              )
              .then((written: unknown) => ({ frameId: frame.id, written }));
          })
          .then((result: unknown) => (w.__e2eWriteOut = result))
          .catch((reason: unknown) =>
            (w.__e2eWriteOut = { failure: String(reason) })
          );
      }, output);
      await browser.waitUntil(
        async () =>
          browser.execute(
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            () => (window as any).__e2eWriteOut !== undefined
          ),
        { timeoutMsg: "write out never answered" }
      );
      const written = await browser.execute(() => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const result = (window as any).__e2eWriteOut;
        const objects = result?.written?.objects ?? [];
        const originalFrame = objects.find(
          (object: { id: string }) => object.id === result.frameId
        );
        const outputFrame = objects.find(
          (object: { id: string; kind: string }) =>
            object.kind === "frame" && object.id !== result.frameId
        );
        return {
          failure: result?.failure ?? null,
          originalSteps: originalFrame?.steps?.length ?? -1,
          outputSteps: outputFrame ? outputFrame.steps?.length ?? 0 : -1,
          objects: objects.map(
            (object: { id: string; kind: string; name: string; steps?: unknown[] }) => ({
              id: object.id,
              kind: object.kind,
              name: object.name,
              steps: object.steps?.length,
            })
          ),
        };
      });
      expect(written.failure).toBeNull();
      expect(written.originalSteps).toBe(1);
      if (written.outputSteps === -1)
        throw new Error(`Output table missing: ${JSON.stringify(written.objects)}`);
      expect(written.outputSteps).toBe(0);
      expect(readFileSync(output, "utf8")).toBe("SKU,Quantity\n0003,5\n");
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  it("exports the same table as Parquet from one chooser", async () => {
    const directory = mkdtempSync(join(tmpdir(), "framework-parquet-"));
    const path = join(directory, "Parquet source.csv");
    writeFileSync(path, "SKU,Quantity\n0004,7\n");
    try {
      const previous = await browser.getWindowHandles();
      const app = resolve("target/debug/bundle/macos/FrameWork.app");
      execFileSync("open", ["-a", app, path]);
      await browser.waitUntil(async () => (await browser.getWindowHandles()).length > previous.length);
      const handles = await browser.getWindowHandles();
      const opened = handles.find((handle) => !previous.includes(handle));
      if (!opened) throw new Error("CSV window did not open");
      await browser.switchToWindow(opened);
      await $('button[aria-label="Sort by SKU"]').waitForExist();

      const output = join(directory, "Parquet output.parquet");
      await browser.execute((outputPath: string) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const w = window as any;
        w.__e2eParquet = undefined;
        w.__TAURI__.core
          .invoke("get_document")
          .then((view: { objects: Array<{ id: string; kind: string }> }) => {
            const frame = view.objects.find((object) => object.kind === "frame");
            if (!frame) throw new Error("Imported frame was missing");
            return w.__TAURI__.core
              .invoke("export_frame_as", { frameId: frame.id, path: outputPath })
              .then((written: unknown) => ({ frameId: frame.id, written }));
          })
          .then((result: unknown) => (w.__e2eParquet = result))
          .catch((reason: unknown) => (w.__e2eParquet = { failure: String(reason) }));
      }, output);
      await browser.waitUntil(
        async () =>
          browser.execute(
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            () => (window as any).__e2eParquet !== undefined
          ),
        { timeoutMsg: "Parquet export never answered" }
      );
      const written = await browser.execute(() => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const result = (window as any).__e2eParquet;
        const objects = result?.written?.objects ?? [];
        const outputFrame = objects.find(
          (object: { id: string; kind: string }) =>
            object.kind === "frame" && object.id !== result.frameId
        );
        return {
          failure: result?.failure ?? null,
          outputName: outputFrame?.name ?? null,
          outputSteps: outputFrame ? outputFrame.steps?.length ?? 0 : -1,
        };
      });
      expect(written.failure).toBeNull();
      // A real Parquet file, and on the canvas as a transformation-free table.
      expect(readFileSync(output).subarray(0, 4).toString("latin1")).toBe("PAR1");
      expect(written.outputSteps).toBe(0);
      expect(written.outputName).toBe("Parquet output");
      // The source is untouched by an export, whatever format it wrote.
      expect(readFileSync(path, "utf8")).toBe("SKU,Quantity\n0004,7\n");
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  // The seam this spec exists for. An opened file's rows are not in the
  // workbook any more — they are read from a staged copy of the file, and
  // typing into one records a correction against that read. Every layer has
  // to agree for the gesture to work at all: the grid must offer the cell,
  // the page must name the row by the ordinal it was read at, the operation
  // must land as a patch rather than a rewrite, and the recomputed page must
  // show the correction over the file's own value. A unit test can prove any
  // one of those; only the real app proves they agree.
  it("types into an opened file and shows the correction over it", async () => {
    const directory = mkdtempSync(join(tmpdir(), "framework-csv-edit-"));
    const path = join(directory, "Edited orders.csv");
    const original = "SKU,Quantity\n0005,11\n0006,12\n";
    writeFileSync(path, original);
    try {
      const previous = await browser.getWindowHandles();
      const app = resolve("target/debug/bundle/macos/FrameWork.app");
      execFileSync("open", ["-a", app, path]);
      await browser.waitUntil(
        async () => (await browser.getWindowHandles()).length > previous.length
      );
      const handles = await browser.getWindowHandles();
      const opened = handles.find((handle) => !previous.includes(handle));
      if (!opened) throw new Error("CSV window did not open");
      await browser.switchToWindow(opened);
      await $('button[aria-label="Sort by SKU"]').waitForExist();

      const cell = $("div.cell-display=12");
      await cell.waitForExist();
      await cell.click();
      await browser.keys(Key.F2);
      const editor = $(".cell-editor");
      await editor.waitForExist();
      await editor.setValue("99");
      await browser.keys(Key.Enter);
      await $("div.cell-display=99").waitForExist();
      expect((await columnCellTexts("Quantity")).slice(0, 2)).toEqual(["11", "99"]);

      // The correction is a note against the file, so the file itself has not
      // moved. Putting it back is a separate, confirmed gesture.
      expect(readFileSync(path, "utf8")).toBe(original);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});
