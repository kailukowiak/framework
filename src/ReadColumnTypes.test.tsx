// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ReadColumnTypes } from "./ReadColumnTypes";
import { fixtures, objectNamed } from "./test/support";
import { readColumnTypes } from "./lib/readColumnTypes";
import { formulaToken } from "./lib/formulaReferences";
import { formattedFormula } from "./PipelineFormulaFormatting";

afterEach(cleanup);

it("authors a source cast on an imported frame and keeps failures inline", async () => {
  const document = fixtures.importedSales;
  const frame = objectNamed(document, "frame", "Imported sales");
  const computed = document.computedFrames[frame.id];
  const columns = frame.baseColumns?.length ? frame.baseColumns : frame.columns;
  const onOperation = vi.fn(async () => "Cannot read this column as a date.");
  render(<ReadColumnTypes frame={frame} computed={computed} columns={columns} onOperation={onOperation} />);
  fireEvent.change(screen.getByLabelText("Read column type"), { target: { value: "date" } });
  await waitFor(() => expect(onOperation).toHaveBeenCalledWith({ type: "setFramePipeline",
    frameId: frame.id, steps: [{ kind: "withColumns", columns: [{
      outputColumnId: columns[0].id, name: columns[0].name,
      formula: formattedFormula(`${formulaToken(columns[0].name)}.cast("date")`),
    }] }] }, { inlineError: true }));
  expect(screen.getByRole("alert").textContent).toContain("Cannot read");
});

it("puts the read cast before existing calculations and retains their output identities", () => {
  const document = fixtures.salesWithMargin;
  const frame = objectNamed(document, "frame", "Monthly sales");
  const columns = frame.baseColumns?.length ? frame.baseColumns : frame.columns;
  const model = readColumnTypes(frame, document.computedFrames[frame.id], columns);
  const before = readColumnTypes(frame, document.computedFrames[frame.id], columns).change(columns[0].id, "");
  const after = model.change(columns[0].id, "string");
  expect(after.slice(1)).toEqual(before);
  expect(after[0]).toMatchObject({ kind: "withColumns", columns: [{ outputColumnId: columns[0].id }] });
  expect(model.columns.map((column) => column.name)).not.toContain("Margin");
});

it("authors the cast in the chain's own formatted spelling so it round-trips", () => {
  const document = fixtures.importedSales;
  const frame = objectNamed(document, "frame", "Imported sales");
  const columns = frame.baseColumns?.length ? frame.baseColumns : frame.columns;
  const model = readColumnTypes(frame, document.computedFrames[frame.id], columns);
  const first = model.change(columns[0].id, "integer")[0];
  if (first.kind !== "withColumns") throw new Error("expected a leading cast step");
  // The engine echoes chain formulas in this form. Authoring any other
  // spelling reseeds the chain drafts on save and leaves the type dropdown
  // unable to recognize the cast it just wrote.
  expect(first.columns[0].formula).toBe(formattedFormula(first.columns[0].formula));
});
