// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ConnectorRefreshConfirmDialog } from "./ConnectorRefreshConfirmDialog";
import type { ConnectorRecipe } from "./lib/types";

describe("ConnectorRefreshConfirmDialog", () => {
  afterEach(() => {
    cleanup();
  });

  it("shows the embedded query and connection for a database connector", () => {
    const connector: ConnectorRecipe = {
      kind: "database",
      connectionId: "conn-1",
      sourceName: "Warehouse",
      query: "select * from accounts",
    };
    const confirm = vi.fn();
    render(
      <ConnectorRefreshConfirmDialog
        frameName="Accounts"
        connector={connector}
        onConfirm={confirm}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByText("select * from accounts")).toBeTruthy();
    expect(screen.getByText("Against connection: Warehouse")).toBeTruthy();

    fireEvent.click(screen.getByText("Refresh"));
    expect(confirm).toHaveBeenCalledOnce();
  });

  it("shows the embedded path for a file connector, and closes on Escape", () => {
    const connector: ConnectorRecipe = { kind: "file", sourcePath: "/data/accounts.csv" };
    const cancel = vi.fn();
    render(
      <ConnectorRefreshConfirmDialog
        frameName="Accounts"
        connector={connector}
        onConfirm={vi.fn()}
        onCancel={cancel}
      />
    );

    expect(screen.getByText("/data/accounts.csv")).toBeTruthy();
    expect(screen.queryByText(/Against connection/)).toBeNull();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(cancel).toHaveBeenCalledOnce();
  });

  it("cancels from the Cancel button", () => {
    const connector: ConnectorRecipe = { kind: "file", sourcePath: "/data/accounts.csv" };
    const cancel = vi.fn();
    render(
      <ConnectorRefreshConfirmDialog
        frameName="Accounts"
        connector={connector}
        onConfirm={vi.fn()}
        onCancel={cancel}
      />
    );

    fireEvent.click(screen.getByText("Cancel"));
    expect(cancel).toHaveBeenCalledOnce();
  });
});
