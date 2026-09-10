// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { DictionaryMenuItems } from "./DictionaryMenuItems";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it("offers dictionary conversion using the selected key without copying the table", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const run = vi.fn().mockResolvedValue(null);
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={vi.fn()} run={run} close={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: /Use for mapping/ }));
  expect(run).toHaveBeenCalledWith({ type: "setUniqueKey", frameId: frame.id, columnIds: [frame.columns[0].id], enabled: true });
});

it("authors a live exact-match formula in the existing transformation editor", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const onMap = vi.fn();
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={onMap} run={vi.fn()} close={vi.fn()} />);
  fireEvent.click(screen.getAllByText("Map values…").at(-1)!);
  fireEvent.click(screen.getAllByRole("button", { name: "Using Category fixes" }).at(0)!);
  expect(onMap).toHaveBeenCalledWith('map_values(`Category`, `Category fixes`.`Key`, `Category fixes`.`Value`)');
});

it.each([false, true])("offers dictionary creation with existing dictionaries: %s", (existing) => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const document = existing ? fixtures.dictionaries : { ...fixtures.dictionaries, objects: [frame] };
  const run = vi.fn().mockResolvedValue(null);
  const close = vi.fn();
  const onMap = vi.fn();
  render(<DictionaryMenuItems document={document} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={onMap} run={run} close={close} />);
  fireEvent.click(screen.getAllByText("Map values…").at(-1)!);
  fireEvent.click(screen.getAllByRole("button", { name: "Create mapping frame…" }).at(-2)!);
  expect(run).toHaveBeenCalledWith({ type: "addDictionary", name: "Category map", x: 100, y: 200 });
  expect(close).toHaveBeenCalledOnce();
  expect(onMap).not.toHaveBeenCalled();
});

it("renames headers through the complete mapping frame in one operation", () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const mapping = objectNamed(fixtures.dictionaries, "frame", "Category fixes");
  const run = vi.fn().mockResolvedValue(null);
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={vi.fn()} run={run} close={vi.fn()} />);
  fireEvent.click(screen.getAllByText("Rename columns…").at(-1)!);
  fireEvent.click(screen.getAllByRole("button", { name: "Using Category fixes" }).at(-1)!);
  expect(run).toHaveBeenCalledWith({ type: "renameColumnsUsingMapping", frameId: frame.id,
    mappingFrameId: mapping.id, keyColumnId: mapping.columns[0].id, valueColumnId: mapping.columns[1].id }, { inlineError: true });
});

it.each([['none', '', 'None'], ['custom', '', 'None'], ['custom', 'OTHER', '"OTHER"']])("authors the %s fallback", (fallback, value, expression) => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const onMap = vi.fn();
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={onMap} run={vi.fn()} close={vi.fn()} />);
  fireEvent.click(screen.getAllByText("Map values…").at(-1)!);
  fireEvent.change(screen.getAllByLabelText("Unmatched values").at(-1)!, { target: { value: fallback } });
  if (fallback === 'custom') fireEvent.change(screen.getAllByLabelText("Unmatched replacement").at(-1)!, { target: { value } });
  fireEvent.click(screen.getAllByRole("button", { name: "Using Category fixes" }).at(-2)!);
  expect(onMap).toHaveBeenCalledWith(`map_values(\`Category\`, \`Category fixes\`.\`Key\`, \`Category fixes\`.\`Value\`, ${expression})`);
});

it("keeps a failed rename beside the mapping choice", async () => {
  const frame = objectNamed(fixtures.dictionaries, "frame", "Sales");
  const close = vi.fn();
  render(<DictionaryMenuItems document={fixtures.dictionaries} frame={frame} column={frame.columns[0]}
    position={{ x: 100, y: 200 }} onMap={vi.fn()} run={vi.fn().mockResolvedValue("Column names must be unique")} close={close} />);
  fireEvent.click(screen.getByText("Rename columns…"));
  fireEvent.click(screen.getAllByRole("button", { name: "Using Category fixes" }).at(-1)!);
  expect((await screen.findByRole("alert")).textContent).toBe("Column names must be unique");
  expect(close).not.toHaveBeenCalled();
});
