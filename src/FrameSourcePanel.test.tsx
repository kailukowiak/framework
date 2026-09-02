// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FrameSourcePanel } from "./FrameSourcePanel";
import {
  forgetConnectorRefreshes,
  recordConnectorRefreshForTest,
} from "./hooks/useConnectorRefreshOutcome";
import type { FrameObject } from "./lib/types";

// What the panel is for after a refresh: saying what changed about the
// shape of the data, in the words of the person who has to act on it. The
// engine's claim — that the frame still computes with a column missing —
// belongs in Rust; this tier only proves the sentences reach the screen.
const frame: FrameObject = {
  kind: "frame",
  id: "frame-orders",
  name: "Orders",
  columns: [],
  rows: [],
  derivation: null,
  uniqueKeys: [],
  summaries: [],
  artifact: {
    id: "artifact-1",
    path: "/data/artifact-1.parquet",
    rowCount: 2,
    format: "parquet",
    sourceName: "orders.csv",
  },
  connector: { kind: "file", sourcePath: "/data/orders.csv" },
};

describe("FrameSourcePanel", () => {
  afterEach(() => {
    cleanup();
    forgetConnectorRefreshes();
  });

  it("reports what the last refresh did to the schema", () => {
    recordConnectorRefreshForTest(frame.id, {
      added: ["Region", "Channel"],
      removed: ["Notes"],
      keptMissing: [["Cost", "still read by the Margin formula"]],
      typeChanged: [["Qty", "integer", "number"]],
    });

    render(<FrameSourcePanel frame={frame} onSourceChanged={vi.fn(async () => null)} />);

    expect(screen.getByText("Added: Region, Channel")).toBeTruthy();
    expect(screen.getByText("Removed: Notes")).toBeTruthy();
    expect(
      screen.getByText("Kept, still read by the Margin formula: Cost (no data)")
    ).toBeTruthy();
    expect(screen.getByText("Type changed: Qty integer → number")).toBeTruthy();
  });

  it("says nothing when the refresh changed no schema", () => {
    recordConnectorRefreshForTest(frame.id, {
      added: [],
      removed: [],
      keptMissing: [],
      typeChanged: [],
    });

    render(<FrameSourcePanel frame={frame} onSourceChanged={vi.fn(async () => null)} />);

    expect(screen.queryByText(/^Added:/)).toBeNull();
    expect(screen.queryByText(/^Removed:/)).toBeNull();
    expect(screen.queryByText(/^Kept,/)).toBeNull();
    expect(screen.queryByText(/^Type changed:/)).toBeNull();
  });

  it("keeps one frame's report off another frame's panel", () => {
    recordConnectorRefreshForTest("some-other-frame", {
      added: ["Region"],
      removed: [],
      keptMissing: [],
      typeChanged: [],
    });

    render(<FrameSourcePanel frame={frame} onSourceChanged={vi.fn(async () => null)} />);

    expect(screen.queryByText(/^Added:/)).toBeNull();
  });
});
