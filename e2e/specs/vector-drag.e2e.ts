import { $, browser } from "@wdio/globals";
import { resetAndOpenTutorial } from "../lib/helpers";

async function dragVectorToEmptyCanvas(name: string): Promise<string> {
  return browser.execute((sourceName: string) => {
    const input = Array.from(
      document.querySelectorAll<HTMLInputElement>(
        ".object-name-input, .variable-name-input"
      )
    ).find((candidate) => candidate.value === sourceName);
    const source = input
      ?.closest<HTMLElement>(".value-card, .variable-card")
      ?.querySelector<HTMLElement>('[data-vector-drag="true"]');
    if (!source) throw new Error(`the ${sourceName} vector source is missing`);
    const viewport = document.querySelector<HTMLElement>(".canvas-viewport");
    if (!viewport) throw new Error("the canvas viewport is missing");
    const bounds = viewport.getBoundingClientRect();
    let target: { x: number; y: number } | null = null;
    for (let y = bounds.top + 50; y < bounds.bottom - 40 && !target; y += 40) {
      for (let x = bounds.left + 50; x < bounds.right - 40; x += 40) {
        const under = document.elementFromPoint(x, y);
        if (
          under?.closest(".canvas-viewport") &&
          !under.closest(".canvas-object")
        ) {
          target = { x, y };
          break;
        }
      }
    }
    if (!target) throw new Error("no empty part of the visible canvas was found");
    const sourceBounds = source.getBoundingClientRect();
    const start = {
      x: sourceBounds.left + sourceBounds.width / 2,
      y: sourceBounds.top + sourceBounds.height / 2,
    };
    source.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: start.x,
        clientY: start.y,
      })
    );
    window.dispatchEvent(
      new PointerEvent("pointermove", {
        bubbles: true,
        cancelable: true,
        clientX: target.x,
        clientY: target.y,
      })
    );
    const preview =
      document.querySelector(".vector-drag-preview")?.textContent ?? "";
    window.dispatchEvent(
      new PointerEvent("pointerup", {
        bubbles: true,
        cancelable: true,
        clientX: target.x,
        clientY: target.y,
      })
    );
    return preview;
  }, name);
}

async function dragVectorToTable(sourceName: string, tableName: string) {
  return browser.execute((vectorName: string, frameName: string) => {
    const input = Array.from(
      document.querySelectorAll<HTMLInputElement>(
        ".object-name-input, .variable-name-input"
      )
    ).find((candidate) => candidate.value === vectorName);
    const source = input
      ?.closest<HTMLElement>(".value-card, .variable-card")
      ?.querySelector<HTMLElement>('[data-vector-drag="true"]');
    if (!source) throw new Error(`the ${vectorName} vector source is missing`);
    const table = Array.from(
      document.querySelectorAll<HTMLInputElement>(".frame-name")
    )
      .find((input) => input.value === frameName)
      ?.closest<HTMLElement>(".canvas-object");
    const header = table?.querySelector<HTMLElement>(".column-header");
    if (!header) throw new Error(`the ${frameName} table header is missing`);
    header.scrollIntoView({ block: "center", inline: "center" });
    const sourceBounds = source.getBoundingClientRect();
    const bounds = header.getBoundingClientRect();
    const start = {
      x: sourceBounds.left + sourceBounds.width / 2,
      y: sourceBounds.top + sourceBounds.height / 2,
    };
    const target = {
      x: bounds.left + bounds.width / 2,
      y: bounds.top + bounds.height / 2,
    };
    source.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: start.x,
        clientY: start.y,
      })
    );
    window.dispatchEvent(
      new PointerEvent("pointermove", {
        bubbles: true,
        cancelable: true,
        clientX: target.x,
        clientY: target.y,
      })
    );
    const preview =
      document.querySelector(".vector-drag-preview")?.textContent ?? "";
    const marked = header.classList.contains("vector-column-drop");
    window.dispatchEvent(
      new PointerEvent("pointerup", {
        bubbles: true,
        cancelable: true,
        clientX: target.x,
        clientY: target.y,
      })
    );
    return { preview, marked };
  }, sourceName, tableName);
}

async function dragVectorToMatrixAxis(
  sourceName: string,
  matrixName: string,
  axisName: "rows" | "columns"
) {
  return browser.execute((vectorName: string, calculationName: string, axis: string) => {
    const input = Array.from(
      document.querySelectorAll<HTMLInputElement>(
        ".object-name-input, .variable-name-input"
      )
    ).find((candidate) => candidate.value === vectorName);
    const source = input
      ?.closest<HTMLElement>(".value-card, .variable-card")
      ?.querySelector<HTMLElement>('[data-vector-drag="true"]');
    if (!source) throw new Error(`the ${vectorName} vector source is missing`);
    const matrix = Array.from(
      document.querySelectorAll<HTMLInputElement>(".calculation-matrix-name")
    )
      .find((candidate) => candidate.value === calculationName)
      ?.closest<HTMLElement>(".canvas-object");
    const target = matrix?.querySelector<HTMLElement>(`[data-matrix-axis="${axis}"]`);
    if (!target) throw new Error(`the ${calculationName} ${axis} axis is missing`);
    target.scrollIntoView({ block: "center", inline: "center" });
    const sourceBounds = source.getBoundingClientRect();
    const targetBounds = target.getBoundingClientRect();
    const start = {
      x: sourceBounds.left + sourceBounds.width / 2,
      y: sourceBounds.top + sourceBounds.height / 2,
    };
    const end = {
      x: targetBounds.left + targetBounds.width / 2,
      y: targetBounds.top + targetBounds.height / 2,
    };
    source.dispatchEvent(
      new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: start.x,
        clientY: start.y,
      })
    );
    window.dispatchEvent(
      new PointerEvent("pointermove", {
        bubbles: true,
        cancelable: true,
        clientX: end.x,
        clientY: end.y,
      })
    );
    const preview = document.querySelector(".vector-drag-preview")?.textContent ?? "";
    const marked = target.classList.contains("vector-column-drop");
    window.dispatchEvent(
      new PointerEvent("pointerup", {
        bubbles: true,
        cancelable: true,
        clientX: end.x,
        clientY: end.y,
      })
    );
    return { preview, marked };
  }, sourceName, matrixName, axisName);
}

// This gesture crosses the native webview, React drop routing, the explicit
// layout choice, and the Rust pipeline. The tutorial workbook supplies real
// vectors, while the first drop creates the one-column spreadsheet shape a
// person starts from.
describe("vector drag", () => {
  it("creates a vector table, previews it, and HStacks a second vector", async () => {
    await resetAndOpenTutorial("Vectors, dates, and visual joins — Answer key");
    await $("input.frame-name").waitForExist();

    expect(await dragVectorToEmptyCanvas("Scenario")).toContain("New table");
    await $('input.frame-name[value="Scenario table"]').waitForExist();

    const drop = await dragVectorToTable("Multiplier", "Scenario table");
    expect(drop.preview).toContain("Choose layout");
    expect(drop.marked).toBe(true);
    await $("button*=Beside · HStack").click();

    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const table = Array.from(
            document.querySelectorAll<HTMLInputElement>(".frame-name")
          )
            .find((input) => input.value === "Scenario table")
            ?.closest<HTMLElement>(".canvas-object");
          return table?.querySelectorAll(".column-header").length === 2;
        }),
      { timeoutMsg: "HStack did not add the paired vector column" }
    );
  });

  it("VStacks another vector below the first", async () => {
    expect(await dragVectorToEmptyCanvas("Multiplier")).toContain("New table");
    await $('input.frame-name[value="Multiplier table"]').waitForExist();

    const drop = await dragVectorToTable("Multiplier", "Multiplier table");
    expect(drop.preview).toContain("Choose layout");
    await $("button*=Below · VStack").click();

    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const table = Array.from(
            document.querySelectorAll<HTMLInputElement>(".frame-name")
          )
            .find((input) => input.value === "Multiplier table")
            ?.closest<HTMLElement>(".canvas-object");
          return table?.querySelector("table")?.getAttribute("aria-rowcount") === "9";
        }),
      { timeoutMsg: "VStack did not append the second vector's three rows" }
    );
  });

  it("builds a Calculation Matrix by dropping vectors into both axes", async () => {
    await $("button*=Matrix").click();
    const matrixName = "Calculation Matrix";
    await $(`input.calculation-matrix-name[value="${matrixName}"]`).waitForExist();

    const rowDrop = await dragVectorToMatrixAxis("Multiplier", matrixName, "rows");
    expect(rowDrop.preview).toContain("Use in rows");
    expect(rowDrop.marked).toBe(true);
    await browser.waitUntil(
      async () =>
        browser.execute((name: string) => {
          const card = Array.from(
            document.querySelectorAll<HTMLInputElement>(".calculation-matrix-name")
          )
            .find((candidate) => candidate.value === name)
            ?.closest<HTMLElement>(".canvas-object");
          return Array.from(
            card?.querySelectorAll<HTMLTextAreaElement>('[data-matrix-axis="rows"] textarea') ?? []
          ).some((editor) => editor.value === "`Multiplier`");
        }, matrixName),
      { timeoutMsg: "the row vector did not reach the Calculation Matrix" }
    );

    const columnDrop = await dragVectorToMatrixAxis(
      "Base revenue",
      matrixName,
      "columns"
    );
    expect(columnDrop.preview).toContain("Use in columns");
    expect(columnDrop.marked).toBe(true);

    const matrixInput = $(`input.calculation-matrix-name[value="${matrixName}"]`);
    await matrixInput.waitForExist();
    const formula = $(
      `.canvas-object:has(.calculation-matrix-name[value="${matrixName}"]) ` +
        `.calculation-matrix-card > .calculation-matrix-formula textarea`
    );
    await formula.waitForExist();
    await formula.setValue("Multiplier * `Base revenue`");
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());

    await browser.waitUntil(
      async () =>
        browser.execute((name: string) => {
          const card = Array.from(
            document.querySelectorAll<HTMLInputElement>(".calculation-matrix-name")
          )
            .find((candidate) => candidate.value === name)
            ?.closest<HTMLElement>(".canvas-object");
          const output = card?.querySelector(".calculation-matrix-output table");
          return (
            output?.querySelectorAll("tbody tr").length === 3 &&
            output.querySelectorAll("tbody td").length === 12 &&
            Array.from(output.querySelectorAll("tbody td")).at(-1)?.textContent ===
              "110.50"
          );
        }, matrixName),
      { timeoutMsg: "Calculation Matrix did not evaluate its 3 × 4 grid" }
    );
  });
});
