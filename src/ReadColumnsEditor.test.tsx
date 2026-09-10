// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ReadColumnsEditor } from "./ReadColumnsEditor";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it("pastes a batch of header names as one operation", async () => {
  const document = fixtures.salesBeforeFormula;
  const frame = objectNamed(document, "frame", "Monthly sales");
  const onOperation = vi.fn(async () => null);
  render(<ReadColumnsEditor frame={frame} onOperation={onOperation} />);
  const names = frame.columns.map((_, index) => `Header ${index + 1}`);
  const editor = screen.getByLabelText("Column names");
  fireEvent.change(editor, { target: { value: names.join("\t") } });
  fireEvent.blur(editor);
  await waitFor(() => expect(onOperation).toHaveBeenCalledWith({
    type: "renameColumns", frameId: frame.id,
    names: frame.columns.map((column, index) => [column.id, names[index]]),
  }, { inlineError: true }));
});

it("keeps a mismatched paste inline without emitting a partial rename", () => {
  const frame = objectNamed(fixtures.salesBeforeFormula, "frame", "Monthly sales");
  const onOperation = vi.fn(async () => null);
  render(<ReadColumnsEditor frame={frame} onOperation={onOperation} />);
  fireEvent.change(screen.getByLabelText("Column names"), { target: { value: "Only one" } });
  fireEvent.blur(screen.getByLabelText("Column names"));
  expect(screen.getByRole("alert").textContent).toContain("Expected");
  expect(onOperation).not.toHaveBeenCalled();
});
