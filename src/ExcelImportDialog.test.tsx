// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExcelImportDialog } from "./ExcelImportDialog";
import { clearMocks, serveInvoke } from "./test/support";

describe("ExcelImportDialog", () => {
  afterEach(() => {
    cleanup();
    clearMocks();
  });

  it("previews and imports the explicit range without formula semantics", async () => {
    const preview = vi.fn(() => ({
      columns: ["Account", "Amount"],
      rows: [["Sales", "12"]],
      rowCount: 1,
      formulaCellCount: 1,
      errorCellCount: 0,
    }));
    serveInvoke({ preview_excel_range: preview });
    const onImport = vi.fn(async () => undefined);
    render(
      <ExcelImportDialog
        workbook={{
          path: "/tmp/actuals.xlsx",
          fileName: "actuals.xlsx",
          sheets: [{ name: "Ledger", usedRange: "B3:C4", rowCount: 2, columnCount: 2 }],
          tables: [],
          suggestedRegions: [],
        }}
        onClose={vi.fn()}
        onImport={onImport}
      />
    );

    expect(await screen.findByText("Sales")).toBeTruthy();
    expect(screen.getByText(/last saved values/i)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Import range" }));

    expect(preview).toHaveBeenCalledWith({
      path: "/tmp/actuals.xlsx",
      sheetName: "Ledger",
      cellRange: "B3:C4",
      hasHeader: true,
      limit: 20,
    });
    expect(onImport).toHaveBeenCalledWith(
      {
        sheetName: "Ledger",
        cellRange: "B3:C4",
        hasHeader: true,
        name: "Ledger",
      },
      false
    );
  });

  it("cannot import a stale preview after the selection changes", async () => {
    serveInvoke({
      preview_excel_range: () => ({
        columns: ["Amount"],
        rows: [["12"]],
        rowCount: 1,
        formulaCellCount: 0,
        errorCellCount: 0,
      }),
    });
    render(
      <ExcelImportDialog
        workbook={{
          path: "/tmp/actuals.xlsx",
          fileName: "actuals.xlsx",
          sheets: [{ name: "Ledger", usedRange: "A1:A2", rowCount: 2, columnCount: 1 }],
          tables: [],
          suggestedRegions: [],
        }}
        onClose={vi.fn()}
        onImport={vi.fn(async () => undefined)}
      />
    );
    expect(await screen.findByText("12")).toBeTruthy();

    await userEvent.clear(screen.getByRole("textbox", { name: "Range" }));

    expect(
      (screen.getByRole("button", { name: "Import range" }) as HTMLButtonElement).disabled
    ).toBe(true);
  });

  it("offers a detected loose rectangle without treating it as an Excel table", async () => {
    const preview = vi.fn(() => ({
      columns: ["Date", "Amount"],
      rows: [["2026-01-01", "12"]],
      rowCount: 10,
      formulaCellCount: 0,
      errorCellCount: 0,
    }));
    serveInvoke({ preview_excel_range: preview });
    render(
      <ExcelImportDialog
        workbook={{
          path: "/tmp/actuals.xlsx",
          fileName: "actuals.xlsx",
          sheets: [{ name: "Ledger", usedRange: "A1:S25", rowCount: 25, columnCount: 19 }],
          tables: [{ name: "LedgerTable", sheetName: "Ledger", cellRange: "A1:M20" }],
          suggestedRegions: [{
            sheetName: "Ledger",
            cellRange: "P15:S25",
            rowCount: 11,
            columnCount: 4,
          }],
        }}
        onClose={vi.fn()}
        onImport={vi.fn(async () => undefined)}
      />
    );

    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Suggested region" }),
      "P15:S25"
    );

    expect(await screen.findByText("2026-01-01")).toBeTruthy();
    expect(preview).toHaveBeenLastCalledWith({
      path: "/tmp/actuals.xlsx",
      sheetName: "Ledger",
      cellRange: "P15:S25",
      hasHeader: true,
      limit: 20,
    });
  });
});
