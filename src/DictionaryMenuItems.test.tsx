// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { DictionaryMenuItems } from "./DictionaryMenuItems";
import { fixtures, objectNamed } from "./test/support";

it("offers dictionary conversion using the selected key without copying the table", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const run = vi.fn().mockResolvedValue(null);
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    onMap={vi.fn()} run={run} close={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: /Use as dictionary/ }));
  expect(run).toHaveBeenCalledWith({ type: "setUniqueKey", frameId: frame.id, columnIds: [frame.columns[0].id], enabled: true });
});

it("authors a live exact-match formula in the existing transformation editor", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const onMap = vi.fn();
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    onMap={onMap} run={vi.fn()} close={vi.fn()} />);
  fireEvent.click(screen.getAllByText("Map values…").at(-1)!);
  fireEvent.click(screen.getAllByRole("button", { name: "Using Category fixes" }).at(-1)!);
  expect(onMap).toHaveBeenCalledWith('map_values(`Category`, `Category fixes`.`Key`, `Category fixes`.`Value`)');
});
