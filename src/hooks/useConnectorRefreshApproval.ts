import { useCallback, useState } from "react";
import type { RefreshConnectorHandler } from "../FrameGrid";
import {
  approveConnectorRefresh,
  connectorApprovalSubject,
  isConnectorRefreshApproved,
} from "../lib/connectorApproval";
import type { ConnectorRecipe, DocumentView } from "../lib/types";

export type PendingConnectorRefresh = {
  frameId: string;
  frameName: string;
  connector: ConnectorRecipe;
  options?: { inlineError?: boolean };
};

/**
 * Wraps a raw refresh handler with a one-time confirmation for connectors
 * whose subject (a database query, a file path) travels inside the document
 * rather than being chosen locally — see lib/connectorApproval.ts for why.
 * A CLI connector's program lives in a local profile the document can't
 * reach, so it always passes straight through.
 */
export function useConnectorRefreshApproval({
  document,
  refreshConnector,
}: {
  document: DocumentView | null;
  refreshConnector: RefreshConnectorHandler;
}) {
  const [pendingConnectorRefresh, setPendingConnectorRefresh] =
    useState<PendingConnectorRefresh | null>(null);

  const requestConnectorRefresh: RefreshConnectorHandler = useCallback(
    async (frameId, options) => {
      const frame = document?.objects.find(
        (object) => object.kind === "frame" && object.id === frameId
      );
      const connector =
        frame && frame.kind === "frame" ? frame.connector : null;
      const subject = connector ? connectorApprovalSubject(connector) : null;
      if (
        document &&
        frame &&
        connector &&
        subject !== null &&
        !isConnectorRefreshApproved(document.id, frameId, subject)
      ) {
        setPendingConnectorRefresh({
          frameId,
          frameName: frame.name,
          connector,
          options,
        });
        return null;
      }
      return refreshConnector(frameId, options);
    },
    [document, refreshConnector]
  );

  const confirmPendingConnectorRefresh = useCallback(() => {
    const pending = pendingConnectorRefresh;
    if (!pending || !document) return;
    const subject = connectorApprovalSubject(pending.connector);
    if (subject !== null)
      approveConnectorRefresh(document.id, pending.frameId, subject);
    setPendingConnectorRefresh(null);
    void refreshConnector(pending.frameId, pending.options);
  }, [document, pendingConnectorRefresh, refreshConnector]);

  const cancelPendingConnectorRefresh = useCallback(
    () => setPendingConnectorRefresh(null),
    []
  );

  return {
    pendingConnectorRefresh,
    requestConnectorRefresh,
    confirmPendingConnectorRefresh,
    cancelPendingConnectorRefresh,
  };
}
