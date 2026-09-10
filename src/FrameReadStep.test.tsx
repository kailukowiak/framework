// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { FrameReadStep } from "./FrameReadStep";
import { fixtures, objectNamed } from "./test/support";

afterEach(cleanup);

it("offers source replacement on the frame's Read step", async () => {
  const document = fixtures.importedSales;
  const frame = objectNamed(document, "frame", "Imported sales");
  const onSourceChanged = vi.fn(async () => null);
  render(<FrameReadStep frame={frame} computed={document.computedFrames[frame.id]}
    onSourceChanged={onSourceChanged} onOperation={vi.fn(async () => null)} />);
  expect(screen.getByLabelText("Read source")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Replace source…" }));
  await waitFor(() => expect(onSourceChanged).toHaveBeenCalledWith(frame.id));
});
