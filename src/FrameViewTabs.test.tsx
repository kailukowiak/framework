// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FrameViewTabs } from "./FrameViewTabs";
import type { CanvasView, FrameObject } from "./lib/types";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

// A card's name is carried by the title row's rename field. A single frame
// tab drawn as well is the same word on a second row, so the strip folds to
// its "+" alone; with two tabs the strip is the way between them and stays.

const view = { id: "view-1" } as CanvasView;
const frame = objectNamed(fixtures.salesWithMargin, "frame", "Monthly sales");
const sibling: FrameObject = { ...frame, id: "other", name: "Monthly sales 2" };

function mount(tabs: FrameObject[]) {
  return render(
    <FrameViewTabs
      view={view}
      tabs={tabs}
      activeObject={tabs[0]}
      sourceFrame={frame}
      computedFrames={{}}
      onOperation={vi.fn()}
      onActivate={vi.fn()}
      closableTabIds={new Set()}
      zoom={1}
      filterCount={() => 0}
      defaultPlotSpec={() => ({})}
    />
  );
}

describe("FrameViewTabs", () => {
  it("draws no tab for a single frame tab, only the add button", () => {
    mount([frame]);
    expect(screen.queryAllByRole("tab")).toHaveLength(0);
    expect(screen.queryByText("Monthly sales")).toBeNull();
    expect(screen.getByLabelText("Add a tab showing the same data")).not.toBeNull();
  });

  it("keeps the strip once there are two tabs", () => {
    mount([frame, sibling]);
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Monthly sales",
      "Monthly sales 2",
    ]);
  });
});
