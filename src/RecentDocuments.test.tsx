// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RecentDocuments } from "./RecentDocuments";

// The regression: two recent documents that share a filename (the tour's
// "Start" and "Answer key" copies) used to be told apart only by however
// much of the shared absolute path survived a left-anchored CSS truncation
// — often nothing, since both paths agree right up to the last folder or
// two. The secondary line now shows those differing folders directly.
describe("RecentDocuments", () => {
  afterEach(cleanup);

  it("shows the last two enclosing folders instead of the full path", () => {
    render(
      <RecentDocuments
        loading={false}
        recents={[
          {
            title: "The FrameWork tour",
            path: "/Users/kai/Documents/FrameWork Tutorials/The FrameWork tour/Start/Tour.fw",
            readable: true,
          },
          {
            title: "The FrameWork tour",
            path: "/Users/kai/Documents/FrameWork Tutorials/The FrameWork tour/Answer key/Tour.fw",
            readable: true,
          },
        ]}
        opening={null}
        onOpen={vi.fn()}
      />
    );

    expect(screen.getByText("The FrameWork tour › Start")).toBeTruthy();
    expect(screen.getByText("The FrameWork tour › Answer key")).toBeTruthy();
  });

  it("keeps the full path available as a title attribute", () => {
    const fullPath = "/Users/kai/Documents/FrameWork Tutorials/The FrameWork tour/Start/Tour.fw";
    render(
      <RecentDocuments
        loading={false}
        recents={[{ title: "The FrameWork tour", path: fullPath, readable: true }]}
        opening={null}
        onOpen={vi.fn()}
      />
    );

    expect(screen.getByText("The FrameWork tour › Start").getAttribute("title")).toBe(
      fullPath
    );
  });
});
