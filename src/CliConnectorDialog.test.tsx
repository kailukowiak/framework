// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CliConnectorDialog } from "./CliConnectorDialog";
import { clearMocks, serveInvoke } from "./test/support";

describe("CliConnectorDialog", () => {
  afterEach(() => {
    cleanup();
    clearMocks();
  });

  it("saves the local command connection and emits a portable source recipe", async () => {
    const profile = {
      id: "aws-profile",
      kind: "script" as const,
      name: "AWS CLI",
      program: "/usr/local/bin/aws",
      arguments: ["s3", "cp", "{source}", "-"],
      output: "csv",
    };
    const save = vi.fn(() => profile);
    serveInvoke({
      list_cli_connector_profiles: () => [profile],
      save_cli_connector_profile: save,
    });
    const onImport = vi.fn(async () => {});
    render(<CliConnectorDialog kind="script" onClose={vi.fn()} onImport={onImport} />);

    expect(screen.getByLabelText("Connection")).toBeTruthy();
    expect(screen.getByLabelText("Connection name")).toBeTruthy();
    expect(screen.queryByLabelText("Base query (optional)")).toBeNull();

    await userEvent.type(
      await screen.findByLabelText("Source name or address"),
      "s3://finance/orders.csv"
    );
    expect(screen.queryByLabelText("Refresh")).toBeNull();
    expect(screen.getByText("Cached result")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Add" }));

    expect(save).toHaveBeenCalledWith({ profile });
    expect(onImport).toHaveBeenCalledWith({
      profileId: "aws-profile",
      sourceLabel: "s3://finance/orders.csv",
      query: null,
    });
  });
});
