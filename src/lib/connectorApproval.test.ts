// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import {
  approveConnectorRefresh,
  connectorApprovalSubject,
  isConnectorRefreshApproved,
} from "./connectorApproval";
import type { ConnectorRecipe } from "./types";

const stored = new Map<string, string>();
const localStorage = {
  get length() {
    return stored.size;
  },
  clear: () => stored.clear(),
  getItem: (key: string) => stored.get(key) ?? null,
  key: (index: number) => [...stored.keys()][index] ?? null,
  removeItem: (key: string) => stored.delete(key),
  setItem: (key: string, value: string) => stored.set(key, value),
};

beforeEach(() => {
  stored.clear();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: localStorage,
  });
});

const database: ConnectorRecipe = {
  kind: "database",
  connectionId: "conn-1",
  sourceName: "Warehouse",
  query: "select * from accounts",
};
const file: ConnectorRecipe = { kind: "file", sourcePath: "/data/accounts.csv" };
const cli: ConnectorRecipe = {
  kind: "cli",
  profileId: "profile-1",
  sourceLabel: "export-accounts",
};

describe("connectorApprovalSubject", () => {
  it("is the query for a database connector", () => {
    expect(connectorApprovalSubject(database)).toBe("select * from accounts");
  });

  it("is the path for a file connector", () => {
    expect(connectorApprovalSubject(file)).toBe("/data/accounts.csv");
  });

  it("is null for a CLI connector, whose program lives in a local profile", () => {
    expect(connectorApprovalSubject(cli)).toBeNull();
  });
});

describe("connector refresh approval", () => {
  it("is not approved before anyone approves it", () => {
    expect(isConnectorRefreshApproved("doc-1", "frame-1", "select 1")).toBe(false);
  });

  it("is approved for the same document, frame, and subject once approved", () => {
    approveConnectorRefresh("doc-1", "frame-1", "select 1");
    expect(isConnectorRefreshApproved("doc-1", "frame-1", "select 1")).toBe(true);
  });

  it("asks again when the subject changes, even for the same frame", () => {
    approveConnectorRefresh("doc-1", "frame-1", "select 1");
    expect(isConnectorRefreshApproved("doc-1", "frame-1", "select 2")).toBe(false);
  });

  it("keeps approvals separate per document", () => {
    approveConnectorRefresh("doc-1", "frame-1", "select 1");
    expect(isConnectorRefreshApproved("doc-2", "frame-1", "select 1")).toBe(false);
  });

  it("keeps approvals separate per frame within the same document", () => {
    approveConnectorRefresh("doc-1", "frame-1", "select 1");
    expect(isConnectorRefreshApproved("doc-1", "frame-2", "select 1")).toBe(false);
  });
});
