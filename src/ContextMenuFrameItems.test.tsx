// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ContextMenuFrameActions } from "./ContextMenuFrameItems";
import type { FrameObject, Operation } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

function frameWithMonth() {
  const sales = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const month = {
    ...sales.columns[0],
    id: "month-column",
    name: "Month",
    dataType: "date",
  } as FrameObject["columns"][number];
  return { ...sales, columns: [...sales.columns, month] };
}

function menuProps(frame: FrameObject, run: (operation: Operation) => Promise<string | null>) {
  return {
    contextMenu: {
      screenX: 100,
      screenY: 200,
      canvasX: 100,
      canvasY: 200,
      frameId: frame.id,
    },
    contextFrame: frame,
    contextColumn: null,
    askOnImport: false,
    importMode: "stored" as const,
    runAppendImport: vi.fn(),
    run,
    setAppendImport: vi.fn(),
    setContextMenu: vi.fn(),
    setInspectorSection: vi.fn(),
    setJoin: vi.fn(),
    setSelection: vi.fn(),
  };
}

it("declares a date column as the period with one choice", () => {
  const frame = frameWithMonth();
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  fireEvent.change(screen.getByLabelText("Period column"), {
    target: { value: "month-column" },
  });
  expect(run).toHaveBeenCalledWith({
    type: "setFramePeriod",
    frameId: frame.id,
    period: { columnId: "month-column", partitionColumnIds: [] },
  });
});

it("clears the declaration back to none", () => {
  const frame = {
    ...frameWithMonth(),
    period: { columnId: "month-column", partitionColumnIds: [] },
  };
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  expect(
    (screen.getByLabelText("Period column") as HTMLSelectElement).value
  ).toBe("month-column");
  fireEvent.change(screen.getByLabelText("Period column"), {
    target: { value: "" },
  });
  expect(run).toHaveBeenCalledWith({
    type: "setFramePeriod",
    frameId: frame.id,
    period: null,
  });
});

it("offers no period choice without a date column", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const run = vi.fn(async (): Promise<string | null> => null);
  render(<ContextMenuFrameActions {...menuProps(frame, run)} />);
  expect(screen.queryByLabelText("Period column")).toBeNull();
});
