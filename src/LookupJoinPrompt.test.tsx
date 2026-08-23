// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LookupJoinPrompt, suggestedLookupKey } from "./LookupJoinPrompt";
import type { Column, DocumentView, FrameObject } from "./lib/types";
import { clearMocks, serveInvoke } from "./test/support";

const column = (id: string, name: string): Column => ({
  id,
  name,
  dataType: "string",
  formula: null,
});

const orders = {
  kind: "frame",
  id: "orders",
  name: "Orders",
  columns: [column("order", "Order ID"), column("customer", "Customer ID")],
  rows: [],
  uniqueKeys: [],
  summaries: [],
} as unknown as FrameObject;

const customers = {
  kind: "frame",
  id: "customers",
  name: "Customers",
  columns: [column("customer-key", "Customer ID"), column("region", "Region")],
  rows: [],
  uniqueKeys: [{ id: "customer-unique", columnIds: ["customer-key"] }],
  summaries: [],
} as unknown as FrameObject;

const document = {
  objects: [orders, customers],
  computedFrames: {},
} as unknown as DocumentView;

afterEach(clearMocks);

describe("lookup join gesture", () => {
  it("suggests the compatible unique key with the same name", () => {
    expect(suggestedLookupKey(orders.columns[1], customers)).toBe("customer-key");
  });

  it("brings only the dragged lookup columns over after full-data diagnostics", async () => {
    serveInvoke({
      get_join_diagnostics: () => ({
        primaryRows: 20,
        matchedRows: 18,
        unmatchedRows: 2,
        lookupRows: 10,
        lookupNullKeyRows: 0,
        lookupDuplicateKeyValues: 0,
      }),
    });
    const onOperation = vi.fn().mockResolvedValue(null);
    const onCreated = vi.fn();
    render(
      <LookupJoinPrompt
        state={{
          primaryFrameId: orders.id,
          primaryKeyId: "customer",
          lookupFrameId: customers.id,
          lookupOutputColumnIds: ["region"],
          x: 700,
          y: 20,
        }}
        document={document}
        onClose={() => undefined}
        onMoreOptions={() => undefined}
        onOperation={onOperation}
        onCreated={onCreated}
      />
    );

    await screen.findByText("18 matched");
    fireEvent.click(screen.getByRole("button", { name: /Bring columns over/ }));

    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(onOperation).toHaveBeenCalledWith(
      {
        type: "addJoinFrame",
        primaryFrameId: "orders",
        lookupFrameId: "customers",
        primaryKeyColumnIds: ["customer"],
        lookupKeyColumnIds: ["customer-key"],
        joinType: "left",
        columns: [
          { sourceFrameId: "orders", sourceColumnId: "order", name: "Order ID" },
          {
            sourceFrameId: "orders",
            sourceColumnId: "customer",
            name: "Customer ID",
          },
          { sourceFrameId: "customers", sourceColumnId: "region", name: "Region" },
        ],
        name: "Orders + Customers",
        x: 700,
        y: 20,
      },
      { inlineError: true }
    );
  });
});
