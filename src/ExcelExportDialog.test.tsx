// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExcelExportDialog } from "./ExcelExportDialog";
import { fixtures, objectNamed } from "./test/support";

describe("ExcelExportDialog", () => {
  afterEach(cleanup);

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
    expect(onExport).toHaveBeenLastCalledWith([frame.id]);

    await userEvent.click(table);
    await userEvent.click(screen.getByRole("button", { name: "Export .xlsx…" }));

    expect(onExport).toHaveBeenLastCalledWith([]);
  });
});
