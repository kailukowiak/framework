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

  // The dialog used to lose a row when the key was marked unique, which
  // recentred it and slid the create button up under the pointer. The row
  // now stays and states what it knows.
  it("confirms a unique key in place of the button that marked it", async () => {
    serveInvoke({
      get_join_diagnostics: () => ({
        primaryRows: 20,
        matchedRows: 20,
        unmatchedRows: 0,
        lookupRows: 10,
        lookupNullKeyRows: 0,
        lookupDuplicateKeyValues: 0,
      }),
    });
    const prompt = (lookupFrame: FrameObject) => (
      <LookupJoinPrompt
        state={{
          primaryFrameId: orders.id,
          primaryKeyId: "customer",
          lookupFrameId: lookupFrame.id,
          lookupOutputColumnIds: ["region"],
          x: 0,
          y: 0,
        }}
        document={
          {
            objects: [orders, lookupFrame],
            computedFrames: {},
          } as unknown as DocumentView
        }
        onClose={() => undefined}
        onMoreOptions={() => undefined}
        onOperation={vi.fn().mockResolvedValue(null)}
        onCreated={() => undefined}
      />
    );
    const notUnique = { ...customers, uniqueKeys: [] } as unknown as FrameObject;
    const view = render(prompt(notUnique));
    await screen.findByText("20 matched");
    expect(
      screen.getByRole("button", { name: /Mark Customer ID as unique/ })
    ).toBeTruthy();

    view.rerender(prompt(customers));
    await screen.findByText(/Customer ID is unique/);
    expect(screen.queryByRole("button", { name: /Mark Customer ID as unique/ })).toBe(
      null
    );
    view.unmount();
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
