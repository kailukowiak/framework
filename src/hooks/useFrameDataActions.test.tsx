// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { clearMocks, fixtures, serveInvoke } from "../test/support";
import { useFrameDataActions } from "./useFrameDataActions";

const setters = () => ({
  setDocument: vi.fn(),
  setDataRefreshRevision: vi.fn(),
  setError: vi.fn(),
  setNotice: vi.fn(),
});
function Surface(props: ReturnType<typeof setters>) {
  const { updateOriginalFile, exportFrameFile } = useFrameDataActions(props);
  return (
    <>
      <button onClick={() => void exportFrameFile("selected-frame")}>Export frame as</button>
      <button onClick={() => void updateOriginalFile("selected-frame")}>Update original</button>
    </>
  );
}
afterEach(() => {
  cleanup();
  clearMocks();
});

it("adds the imported output and invalidates pages only after a successful write", async () => {
  const invoke = vi.fn(() => fixtures.salesBeforeFormula);
  serveInvoke({ export_frame_as: invoke });
  const props = setters();
  render(<Surface {...props} />);
  await userEvent.click(screen.getByRole("button", { name: "Export frame as" }));
  await waitFor(() =>
    expect(props.setDocument).toHaveBeenCalledWith(fixtures.salesBeforeFormula)
  );
  expect(invoke).toHaveBeenCalledWith({ frameId: "selected-frame" });
  expect(props.setDataRefreshRevision).toHaveBeenCalledOnce();
});

it("cancellation keeps the working document and does not announce an output", async () => {
  serveInvoke({ export_frame_as: () => null });
  const props = setters();
  render(<Surface {...props} />);
  await userEvent.click(screen.getByRole("button", { name: "Export frame as" }));
  await waitFor(() => expect(props.setError).toHaveBeenCalledWith(null));
  expect(props.setDocument).not.toHaveBeenCalled();
  expect(props.setDataRefreshRevision).not.toHaveBeenCalled();
  expect(props.setNotice).not.toHaveBeenCalled();
});

it("a refused write reports the reason without replacing the draft", async () => {
  serveInvoke({
    export_frame_as: () => {
      throw new Error("The source file changed outside FrameWork.");
    },
  });
  const props = setters();
  render(<Surface {...props} />);
  await userEvent.click(screen.getByRole("button", { name: "Export frame as" }));
  await waitFor(() =>
    expect(props.setError).toHaveBeenCalledWith(
      "The source file changed outside FrameWork."
    )
  );
  expect(props.setDocument).not.toHaveBeenCalled();
  expect(props.setNotice).not.toHaveBeenCalled();
});

it("adopts the baked table after updating its original file", async () => {
  const invoke = vi.fn(() => fixtures.salesBeforeFormula);
  serveInvoke({ update_original_delimited: invoke });
  const props = setters();
  render(<Surface {...props} />);
  await userEvent.click(screen.getByRole("button", { name: "Update original" }));
  await waitFor(() =>
    expect(props.setDocument).toHaveBeenCalledWith(fixtures.salesBeforeFormula)
  );
  expect(invoke).toHaveBeenCalledWith({ frameId: "selected-frame" });
  expect(props.setDataRefreshRevision).toHaveBeenCalledOnce();
  expect(props.setNotice).toHaveBeenCalledWith(
    "Original file updated and added as a direct read. Your transformation chain is still in this workbook."
  );
});
