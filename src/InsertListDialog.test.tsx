// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InsertListDialog } from "./InsertListDialog";

describe("InsertListDialog", () => {
  afterEach(cleanup);

  it("leaves the document interactive while the list editor is open", async () => {
    const canvasAction = vi.fn();
    render(
      <>
        <button onClick={canvasAction}>Canvas value</button>
        <InsertListDialog
          state={{ containerId: "scenario-vectors" }}
          onClose={vi.fn()}
          onCreate={vi.fn()}
          onPickFile={vi.fn(async () => null)}
        />
      </>
    );

    expect(screen.getByRole("dialog").getAttribute("aria-modal")).toBe("false");
    expect(document.querySelector(".dialog-backdrop")).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: "Canvas value" }));
    expect(canvasAction).toHaveBeenCalledOnce();
  });

  it("creates a list from compact comma-separated input", async () => {
    const onCreate = vi.fn();
    render(
      <InsertListDialog
        state={{ containerId: "scenario-vectors" }}
        onClose={vi.fn()}
        onCreate={onCreate}
        onPickFile={vi.fn(async () => null)}
      />
    );

    await userEvent.type(
      screen.getByRole("textbox", { name: /Values/ }),
      "Base, Upside, Downside"
    );
    await userEvent.click(screen.getByRole("button", { name: /Create vector/ }));

    expect(onCreate).toHaveBeenCalledWith({
      type: "addSeries",
      name: "New vector",
      values: "Base, Upside, Downside",
      x: 0,
      y: 0,
      containerId: "scenario-vectors",
    });
  });
});
