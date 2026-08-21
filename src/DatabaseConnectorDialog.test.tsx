// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DatabaseConnectorDialog } from "./DatabaseConnectorDialog";
import { clearMocks, serveInvoke } from "./test/support";

describe("DatabaseConnectorDialog", () => {
  afterEach(() => { cleanup(); clearMocks(); });

  it("keeps the URI local and emits the query as the table recipe", async () => {
    const connection = {
      id: "warehouse",
      name: "Warehouse",
      uri: "postgresql://reader:secret@server/finance",
    };
    const save = vi.fn(() => connection);
    serveInvoke({
      list_database_connections: () => [connection],
      save_database_connection: save,
    });
    const onImport = vi.fn(async () => {});
    render(<DatabaseConnectorDialog onClose={vi.fn()} onImport={onImport} />);

    await userEvent.type(await screen.findByLabelText("Table name"), "Ledger");
    await userEvent.type(screen.getByLabelText("SQL"), "select * from finance.ledger");
    await userEvent.click(screen.getByRole("button", { name: "Add" }));

    expect(save).toHaveBeenCalledWith({ connection });
    expect(onImport).toHaveBeenCalledWith({
      connectionId: "warehouse",
      sourceName: "Ledger",
      query: "select * from finance.ledger",
    });
    expect(screen.queryByLabelText("Refresh")).toBeNull();
    expect(screen.getByText("Cached result")).toBeTruthy();
  });
});
