// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExcelExportDialog } from "./ExcelExportDialog";
import { clearMocks, fixtures, objectNamed, serveInvoke } from "./test/support";

describe("ExcelExportDialog", () => {
  beforeEach(() => serveInvoke({ export_row_counts: () => ({}) }));
  afterEach(() => { cleanup(); clearMocks(); });

  it("selects every table initially and exports only tables left checked", async () => {
    const frame = objectNamed(fixtures.salesWithFormula, "frame", "Monthly sales");
    const onExport = vi.fn(async () => true);
    render(
      <ExcelExportDialog
        document={fixtures.salesWithFormula}
        onClose={vi.fn()}
        onExport={onExport}
      />
    );

    const table = screen.getByRole("checkbox", { name: /Monthly sales/ });
    expect((table as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText(/named constants and current formula results/)).toBeTruthy();

    await userEvent.click(screen.getByRole("button", { name: "Export .xlsx…" }));
    expect(onExport).toHaveBeenLastCalledWith([frame.id], true, true);

    await userEvent.click(table);
    await userEvent.click(screen.getByRole("button", { name: "Export .xlsx…" }));

    expect(onExport).toHaveBeenLastCalledWith([], true, true);
  });

  it("includes the lineage sheet by default, and omits it once unchecked", async () => {
    const frame = objectNamed(fixtures.salesWithFormula, "frame", "Monthly sales");
    const onExport = vi.fn(async () => true);
    render(
      <ExcelExportDialog
        document={fixtures.salesWithFormula}
        onClose={vi.fn()}
        onExport={onExport}
      />
    );

    const lineage = screen.getByRole("checkbox", {
      name: "Include a sheet explaining how each column was computed",
    });
    expect((lineage as HTMLInputElement).checked).toBe(true);

    await userEvent.click(screen.getByRole("button", { name: "Export .xlsx…" }));
    expect(onExport).toHaveBeenLastCalledWith([frame.id], true, true);

    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Rows to export" }), "all");
    await userEvent.click(lineage);
    await userEvent.click(screen.getByRole("button", { name: "Export .xlsx…" }));
    expect(onExport).toHaveBeenLastCalledWith([frame.id], false, false);
  });
});
