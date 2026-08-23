// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PipelineJoinStep } from "./PipelineJoinStep";
import type { Column, FrameObject } from "./lib/types";
import { clearMocks, serveInvoke } from "./test/support";

const column = (id: string, name: string): Column => ({
  id,
  name,
  dataType: "string",
  formula: null,
});

const primary = {
  kind: "frame",
  id: "orders",
  name: "Orders",
  columns: [column("customer", "Customer ID"), column("account", "Account ID")],
  uniqueKeys: [],
} as unknown as FrameObject;
const lookup = {
  kind: "frame",
  id: "customers",
  name: "Customers",
  columns: [column("customer-key", "Customer ID"), column("account-key", "Account ID")],
  uniqueKeys: [
    { id: "customer-unique", columnIds: ["customer-key"] },
    { id: "account-unique", columnIds: ["account-key"] },
  ],
} as unknown as FrameObject;
const joined = {
  kind: "frame",
  id: "joined",
  name: "Orders + Customers",
  columns: [column("region-output", "Region")],
  baseColumns: [column("region-output", "Region")],
  derivation: {
    sourceFrameId: primary.id,
    join: {
      lookupFrameId: lookup.id,
      primaryKeyColumnIds: ["customer"],
      lookupKeyColumnIds: ["customer-key"],
      joinType: "left",
      outputs: [
        {
          outputColumnId: "region-output",
          sourceFrameId: lookup.id,
          sourceColumnId: "region-source",
        },
      ],
    },
  },
} as unknown as FrameObject;

afterEach(clearMocks);

describe("join step in Wrangle", () => {
  it("shows the relationship and saves a changed key pair", async () => {
    serveInvoke({
      get_join_diagnostics: () => ({
        primaryRows: 10,
        matchedRows: 8,
        unmatchedRows: 2,
        lookupRows: 8,
        lookupNullKeyRows: 0,
        lookupDuplicateKeyValues: 0,
      }),
    });
    const onOperation = vi.fn().mockResolvedValue(null);
    render(
      <PipelineJoinStep
        frame={joined}
        frames={[primary, lookup, joined]}
        onOperation={onOperation}
      />
    );

    expect(screen.getByText(/Bring Region from Customers/)).toBeTruthy();
    await screen.findByText(/8 matched/);
    fireEvent.change(screen.getByRole("combobox", { name: "Destination join key" }), {
      target: { value: "account" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "Lookup join key" }), {
      target: { value: "account-key" },
    });

    await waitFor(() =>
      expect(onOperation).toHaveBeenLastCalledWith(
        {
          type: "setFrameJoinKeys",
          frameId: "joined",
          primaryKeyColumnIds: ["account"],
          lookupKeyColumnIds: ["account-key"],
        },
        { inlineError: true }
      )
    );
  });
});
