import type { ConnectorRecipe } from "./types";

const STORAGE_KEY = "framework.approvedConnectorRefreshes";

/** The part of a connector that changes what refreshing it actually does —
 *  the query for a database connector, the path for a file connector. A CLI
 *  connector's program and argument templates live in a local profile the
 *  user manages themselves, never in the document, so it has nothing here to
 *  approve. */
export function connectorApprovalSubject(connector: ConnectorRecipe): string | null {
  switch (connector.kind) {
    case "database":
      return connector.query;
    case "file":
      return connector.sourcePath;
    case "cli":
      return null;
  }
}

type ApprovalStore = Record<string, Record<string, string>>;

function readStore(): ApprovalStore {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as ApprovalStore) : {};
  } catch {
    return {};
  }
}

/** Whether this exact subject (query or path) was already approved for this
 *  frame, in this document. Re-approval is required whenever it changes —
 *  including a document that was edited and reopened — since approving is
 *  about the specific thing that is about to run, not a standing grant. */
export function isConnectorRefreshApproved(
  documentId: string,
  frameId: string,
  subject: string
): boolean {
  return readStore()[documentId]?.[frameId] === subject;
}

export function approveConnectorRefresh(
  documentId: string,
  frameId: string,
  subject: string
): void {
  const store = readStore();
  store[documentId] = { ...store[documentId], [frameId]: subject };
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch {
    // The refresh still goes ahead; only the remembered approval is lost, so
    // the prompt reappears next time rather than silently skipping it.
  }
}
