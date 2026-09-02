// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DatasetDialog } from "./DatasetDialog";
import { clearMocks, fixtures, serveInvoke } from "./test/support";

// The transport-seam exemplar: every command the dialog calls on mount is
// answered with explicit, inert data — lists that could never drift because
// they compute nothing — and the interaction claim is about what the dialog
// *does*: clicking a recent document invokes open_document with that
// document's path and hands the answer to onOpened.
describe("DatasetDialog", () => {
  afterEach(() => {
    cleanup();
    clearMocks();
  });

  it("opens a recent document by its path", async () => {
    const openDocument = vi.fn(() => ({
      document: fixtures.blank,
      path: "/somewhere/q3.fw",
    }));
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [
        { title: "Q3 forecast", path: "/somewhere/q3.fw" },
      ],
      list_tutorial_documents: () => ({
        directory: "/tmp/tutorials",
        documents: [
          {
            title: "Importing an Excel workbook — Start",
            lesson: "Importing an Excel workbook",
            kind: "Start",
            path: "/tmp/tutorials/excel/Start/Workbook.fw",
            exists: true,
          },
        ],
      }),
      open_document: openDocument,
    });

    const onOpened = vi.fn();
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={onOpened}
      />
    );

    await userEvent.click(await screen.findByText("Q3 forecast"));

    expect(openDocument).toHaveBeenCalledWith({
      path: "/somewhere/q3.fw",
      safeMode: false,
    });
    expect(onOpened).toHaveBeenCalledWith({
      document: fixtures.blank,
      path: "/somewhere/q3.fw",
    });
  });

  it("opens the explicit Excel-range flow separately from flat-file import", async () => {
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [],
      list_tutorial_documents: () => ({ directory: "/tmp/tutorials", documents: [] }),
    });
    const onImportExcelFile = vi.fn(async () => true);
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={onImportExcelFile}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    await userEvent.click(await screen.findByRole("button", { name: "Excel…" }));

    expect(onImportExcelFile).toHaveBeenCalledOnce();
  });

  it("puts import before document libraries", async () => {
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [
        { title: "Q3 forecast", path: "/somewhere/q3.fw" },
      ],
      list_tutorial_documents: () => ({ directory: "/tmp/tutorials", documents: [] }),
    });
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    const importAction = await screen.findByText("Add data");
    const documentPicker = screen.getByRole("button", {
      name: "Choose another FrameWork document",
    });
    const recentHeading = screen.getByText("Recent documents");
    const learningHeading = screen.getByRole("button", {
      name: /Tutorials and examples/,
    });

    expect(importAction.compareDocumentPosition(recentHeading)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(importAction.compareDocumentPosition(documentPicker)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(documentPicker.compareDocumentPosition(recentHeading)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(importAction.compareDocumentPosition(learningHeading)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(screen.queryByRole("button", { name: "Export Excel…" })).toBeNull();
    expect(screen.queryByText(/Open a data file/)).toBeNull();
    expect(screen.getByRole("button", { name: "Flat…" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Excel…" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "DB…" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "API…" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Script…" })).toBeTruthy();
  });

  it("keeps tutorials and examples together behind one collapsed heading", async () => {
    serveInvoke({
      list_sample_documents: () => [
        {
          fileName: "synthetic/demo.fw",
          title: "Demo workbook",
          path: "/samples/synthetic/demo.fw",
          frameCount: 3,
          category: "Synthetic",
        },
      ],
      list_recent_documents: () => [],
      list_tutorial_documents: () => ({
        directory: "/tmp/tutorials",
        documents: [
          {
            title: "Importing an Excel workbook — Start",
            lesson: "Importing an Excel workbook",
            kind: "Start",
            path: "/tmp/tutorials/excel/Start/Workbook.fw",
            exists: true,
          },
        ],
      }),
    });
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    const heading = await screen.findByRole("button", {
      name: /Tutorials and examples/,
    });
    expect(heading.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("button", { name: "Create tutorials" })).toBeNull();
    expect(screen.queryByText("Demo workbook")).toBeNull();
    expect(screen.queryByText("Importing an Excel workbook — Start")).toBeNull();

    await userEvent.click(heading);

    expect(heading.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("button", { name: "Create tutorials" })).toBeTruthy();
    expect(await screen.findByText("Demo workbook")).toBeTruthy();
    expect(await screen.findByText("Importing an Excel workbook — Start")).toBeTruthy();
  });

  // The regression this whole feature exists for: a tutorial workbook whose
  // file is on disk but that FrameWork cannot open (a stored macOS TCC deny
  // under `tauri dev`) used to look identical to any other tutorial, and
  // clicking it failed through to a bare "Operation not permitted" error
  // below the fold. It should instead stay listed, disabled, and say why.
  it("lists an unreadable tutorial as disabled and labelled, and refuses to open it", async () => {
    const openDocument = vi.fn();
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [],
      list_tutorial_documents: () => ({
        directory: "/tmp/tutorials",
        documents: [
          {
            title: "Importing an Excel workbook — Start",
            lesson: "Importing an Excel workbook",
            kind: "Start",
            path: "/tmp/tutorials/excel/Start/Workbook.fw",
            exists: true,
            readable: false,
          },
        ],
      }),
      open_document: openDocument,
    });
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    await userEvent.click(
      await screen.findByRole("button", { name: /Tutorials and examples/ })
    );
    const row = await screen.findByRole("button", {
      name: /Importing an Excel workbook — Start.*can't be read/,
    });
    expect((row as HTMLButtonElement).disabled).toBe(true);

    await userEvent.click(row);
    expect(openDocument).not.toHaveBeenCalled();
  });

  it("lists an unreadable recent document as disabled and labelled, and refuses to open it", async () => {
    const openDocument = vi.fn();
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [
        { title: "Q3 forecast", path: "/somewhere/q3.fw", readable: false },
      ],
      list_tutorial_documents: () => ({ directory: "/tmp/tutorials", documents: [] }),
      open_document: openDocument,
    });
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    const row = await screen.findByRole("button", {
      name: /Q3 forecast.*can't be read/,
    });
    expect((row as HTMLButtonElement).disabled).toBe(true);

    await userEvent.click(row);
    expect(openDocument).not.toHaveBeenCalled();
  });

  it("shows the library error directly under the header, above every list", async () => {
    serveInvoke({
      list_sample_documents: () => [],
      list_recent_documents: () => [
        { title: "Q3 forecast", path: "/somewhere/q3.fw" },
      ],
      list_tutorial_documents: () => ({ directory: "/tmp/tutorials", documents: [] }),
      open_document: () => {
        throw new Error("Could not load document: Operation not permitted (os error 1)");
      },
    });
    render(
      <DatasetDialog
        document={fixtures.blank}
        onClose={vi.fn()}
        onImportFile={vi.fn(async () => false)}
        onImportExcelFile={vi.fn(async () => false)}
        onImportCliSource={vi.fn(async () => {})}
        onImportDatabaseSource={vi.fn(async () => {})}
        onSourceChanged={vi.fn(async () => null)}
        onOpened={vi.fn()}
      />
    );

    await userEvent.click(await screen.findByText("Q3 forecast"));
    const error = await screen.findByText(
      "Could not load document: Operation not permitted (os error 1)"
    );
    const recentHeading = screen.getByText("Recent documents");

    expect(error.compareDocumentPosition(recentHeading)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
  });
});
