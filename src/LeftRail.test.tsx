// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { LeftRail } from "./LeftRail";
import { acceleratorSymbols, menuCommand } from "./lib/menuCommands";

// The claim under test: a modifier hint on the rail is the menu's own
// accelerator, spelled the menu's way. Nothing here proves the badge is
// drawn — that is one CSS rule — it proves the badge cannot start promising
// a key `src-tauri/src/menu.rs` no longer binds.

/** Rail button, by its visible name, and the menu command it stands for. */
const railCommands: Array<[string, string]> = [
  ["Data", "toggle-sources"],
  ["Library", "data-library"],
  ["Block", "add-block"],
  ["Text", "add-text"],
  ["Frame", "add-frame"],
  ["Container", "add-container"],
  ["Arrange", "tidy-layout"],
];

/** Rail buttons the menu gives no accelerator, which must claim none. */
const unbound = ["Canvas", "Project", "Variable", "Matrix"];

afterEach(cleanup);

const renderRail = () =>
  render(
    <LeftRail
      leftPanel={null}
      setLeftPanel={() => {}}
      toggleLeftPanel={() => {}}
      onOpenLibrary={() => {}}
      addBlock={() => undefined}
      addVariable={() => undefined}
      addText={() => undefined}
      addCalculationMatrix={() => undefined}
      addEmptyFrame={() => undefined}
      addContainer={() => undefined}
      viewCount={3}
      onOperation={async () => null}
    />
  );

describe("LeftRail modifier hints", () => {
  it.each(railCommands)("%s wears the menu's accelerator for %s", (name, id) => {
    renderRail();
    const accelerator = menuCommand(id)?.accelerator;
    expect(accelerator).toBeTruthy();
    expect(screen.getByRole("button", { name }).getAttribute("data-shortcut")).toBe(
      acceleratorSymbols(accelerator as string)
    );
  });

  it.each(unbound)("%s claims no shortcut, because the menu binds none", (name) => {
    renderRail();
    expect(
      screen.getByRole("button", { name }).hasAttribute("data-shortcut")
    ).toBe(false);
  });
});
