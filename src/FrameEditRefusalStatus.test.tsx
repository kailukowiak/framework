// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { frameEditRefusalStatus } from "./FrameEditRefusalStatus";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it("offers ownership at the refused cell and explains loss of refresh before applying it", async () => {
  const frame = objectNamed(fixtures.importedSales, "frame", "Imported sales");
  const onTakeOwnership = vi.fn().mockResolvedValue(null);
  render(frameEditRefusalStatus(frame, fixtures.importedSales.computedFrames[frame.id], {
    rowId: "unloaded", columnId: frame.columns[0].id, editRefused: true,
  }, { rows: [], onTakeOwnership, onTransformColumn: vi.fn(), onEditCalculatedColumn: vi.fn() }));
  await userEvent.click(screen.getByRole("button", { name: "Make rows editable…" }));
  expect(onTakeOwnership).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: /stop source refresh/ }));
  await waitFor(() => expect(onTakeOwnership).toHaveBeenCalledWith(frame.id, { inlineError: true }));
});

it("routes a calculated cell to its declaration, and hides stale refusals on editable cells", async () => {
  const view = fixtures.salesWithMargin;
  const frame = objectNamed(view, "frame", "Monthly sales");
  const column = frame.columns.find((candidate) => candidate.name === "Margin")!;
  const onTransformColumn = vi.fn();
  render(frameEditRefusalStatus(frame, view.computedFrames[frame.id], {
    rowId: frame.rows[0].id, columnId: column.id, editRefused: true,
  }, { rows: frame.rows, onTransformColumn, onEditCalculatedColumn: onTransformColumn }));
  await userEvent.click(screen.getByRole("button", { name: "Edit column formula" }));
  expect(onTransformColumn).toHaveBeenCalledWith(frame, column, 0);
  expect(frameEditRefusalStatus(frame, view.computedFrames[frame.id], {
    rowId: frame.rows[0].id, columnId: frame.columns[0].id, editRefused: true,
  }, { rows: frame.rows, onTransformColumn, onEditCalculatedColumn: onTransformColumn })).toBeNull();
});
