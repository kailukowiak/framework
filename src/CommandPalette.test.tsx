// @vitest-environment jsdom

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { CommandPalette, type PaletteMode } from "./CommandPalette";
import type { QuickCommand } from "./QuickCommands";
import type { DocumentView } from "./lib/types";

// Nothing is paged unless a test says so, so the palette answers from the
// view it was handed and the engine is never asked anything.
const searchFrameRows = vi.fn();
vi.mock("./lib/api", () => ({ searchFrameRows: (...args: unknown[]) => searchFrameRows(...args) }));

// jsdom has no layout, so it has no scrollIntoView. Keeping the selection
// visible is real behaviour; there is simply nothing here for it to scroll.
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

afterEach(cleanup);

const commands = (run: () => void = vi.fn()): QuickCommand[] => [
  { id: "open", label: "Open…", group: "Document", shortcut: "⌘O", run },
  { id: "scratchwork", label: "Open Scratchwork", group: "Canvas", run: vi.fn() },
  { id: "disabled", label: "Unavailable action", group: "View", disabled: true, run: vi.fn() },
  { id: "pivot", label: "Pivot Selected Frame", group: "Canvas", run: vi.fn() },
];

/** A frame with `rows` rows whose Region cell reads "Northolt". */
const view = (rows = 1, paged = false): DocumentView =>
  ({
    objects: [
      {
        kind: "frame",
        id: "sales",
        name: "North Sales",
        columns: [{ id: "region", name: "Northing" }],
        rows: paged ? [] : Array.from({ length: rows }, (_, index) => ({ id: `row-${index}` })),
      },
    ],
    views: [{ id: "view-sales", objectId: "sales" }],
    computedFrames: {
      sales: {
        formulas: {},
        paged,
        rows: Object.fromEntries(
          Array.from({ length: paged ? 0 : rows }, (_, index) => [
            `row-${index}`,
            { region: { display: "Northolt" } },
          ])
        ),
      },
    },
    computedBlocks: {},
    formulaFunctions: [],
  }) as unknown as DocumentView;

function open(
  initialMode: PaletteMode,
  { document = view(), run = vi.fn() }: { document?: DocumentView; run?: () => void } = {}
) {
  const onJump = vi.fn();
  const onOpenHelp = vi.fn();
  const onClose = vi.fn();
  const result = render(
    <CommandPalette
      initialMode={initialMode}
      commands={commands(run)}
      document={document}
      onJump={onJump}
      onOpenHelp={onOpenHelp}
      onClose={onClose}
    />
  );
  const input = screen.getByRole("textbox");
  const type = (value: string) => fireEvent.change(input, { target: { value } });
  const options = () => screen.queryAllByRole("option");
  return { ...result, input, type, options, onJump, onOpenHelp, onClose };
}

describe("the palette as Quick Commands", () => {
  it("filters commands and runs the selected one from the keyboard", () => {
    const run = vi.fn();
    const { input, type, onClose } = open("commands", { run });
    expect(screen.getByLabelText("Search commands")).toBe(input);

    type("open");
    expect(screen.getByText("Open…")).toBeTruthy();
    expect(screen.queryByText("Unavailable action")).toBeNull();

    fireEvent.keyDown(input, { key: "Enter" });
    expect(onClose).toHaveBeenCalledOnce();
    expect(run).toHaveBeenCalledOnce();
  });

  it("closes when the press lands outside it", () => {
    const { container, onClose } = open("commands");
    fireEvent.pointerDown(container.querySelector(".quick-commands-backdrop")!);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("moves through the rows and closes with Escape", () => {
    const { input, onClose } = open("commands");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getByText("Open Scratchwork").closest("button")?.getAttribute("aria-selected"))
      .toBe("true");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  // The palette used to end a query with "Search document for…" and "Search
  // help for…" rows that handed the words to another box without saying
  // whether that box had an answer. The answers are rows here now.
  it("answers one query with commands, then document matches, then help", () => {
    const { type, options, onJump, onOpenHelp } = open("commands");
    type("north");
    expect(options().map((option) => option.textContent)).toEqual([
      "North SalesFrame",
      "North SalesNorthingColumn",
      "North SalesNorthingNortholt",
    ]);
    expect(screen.queryByText(/Search (help|document) for/)).toBeNull();

    fireEvent.click(options()[2]);
    expect(onJump.mock.calls[0][0]).toMatchObject({ kind: "cell", rowId: "row-0" });

    type("pivot");
    expect(options()[0].textContent).toBe("CanvasPivot Selected Frame");
    const answer = options().find((option) => option.textContent?.startsWith("HelpPivot"));
    expect(answer).toBeTruthy();
    // Rows of a different kind start after a rule rather than a heading.
    expect(answer!.className).toContain("section-start");
    fireEvent.click(answer!);
    expect(onOpenHelp).toHaveBeenCalledWith({ query: "pivot", entryId: "reference.pivot" });
  });

  it("does not search the document or help on a single letter", () => {
    const { type, options } = open("commands");
    type("n");
    expect(options().some((option) => option.textContent?.includes("North Sales"))).toBe(false);
  });

  it("shows a few matches and can list every one without retyping the query", () => {
    const { input, type, options, onClose } = open("commands", { document: view(8) });
    type("northolt");
    expect(options().filter((option) => option.textContent?.includes("Northolt"))).toHaveLength(5);

    fireEvent.click(screen.getByText("All 8 matches in the document"));
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Find in document")).toBe(input);
    expect((input as HTMLInputElement).value).toBe("northolt");
    expect(options()).toHaveLength(8);
  });
});

describe("the palette as Find", () => {
  it("lists every match in the document and nothing else", () => {
    const { type, options } = open("document");
    expect(screen.getByRole("dialog", { name: "Find" })).toBeTruthy();
    type("north");
    expect(options().map((option) => option.textContent)).toEqual([
      "North SalesFrame",
      "North SalesNorthingColumn",
      "North SalesNorthingNortholt",
    ]);
    expect(options()[0].getAttribute("aria-selected")).toBe("true");
  });

  it("sends Enter to the selected match and closes", () => {
    const { input, type, onJump, onClose } = open("document");
    type("north");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onJump).toHaveBeenCalledOnce();
    expect(onJump.mock.calls[0][0]).toMatchObject({ kind: "column", columnId: "region" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("says so plainly when nothing matches, and closes on Escape", () => {
    const { input, type, options, onClose } = open("document");
    type("zzz");
    expect(options()).toHaveLength(0);
    expect(screen.getByText("No matches")).toBeTruthy();

    fireEvent.keyDown(input, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("merges a paged frame's matches in once the engine answers", async () => {
    searchFrameRows.mockResolvedValue([
      { rowIndex: 900, rowId: "row-900", columnId: "region", snippet: "Northolt" },
    ]);
    const { type, options, onJump } = open("document", { document: view(0, true) });
    type("north");
    // Flush the search debounce and the resolved promise.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
    });

    const hit = options().find((option) => option.textContent?.endsWith("Northolt"));
    fireEvent.click(hit!);
    expect(onJump.mock.calls[0][0]).toMatchObject({ rowId: "row-900", rowIndex: 900 });
  });
});
