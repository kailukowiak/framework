import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary, waitForGutterAnswer } from "../lib/helpers";

// The scratchpad on a blank canvas: ⌘J summons a Scratchwork block, lines
// evaluate as they are typed, and editing a line upstream moves the answers
// downstream. This is the shortest path through the whole stack — keyboard,
// parser, engine, live recompute — with no document fixture at all.
//
// Multi-line entry goes through setValue rather than typed Enter keys: the
// embedded WebDriver server inserts printable characters as text but does
// not perform Enter's default newline insertion, and a literal "\n" in a
// key action crashes its script interpolation. setValue is still the
// element send-keys endpoint — the same input a person's typing amounts to
// on the wire — so nothing here bypasses the interface.
describe("scratchpad", () => {
  it("summons with ⌘J and evaluates lines live", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await browser.keys([Key.Command, "j"]);
    const source = $(".block-source");
    await source.waitForExist();

    await source.setValue("x = 10\ny = 30\nx + y");

    await waitForGutterAnswer("40");
  });

  it("recomputes dependents when an upstream line changes", async () => {
    const source = $(".block-source");
    await source.setValue("x = 20\ny = 30\nx + y");

    await waitForGutterAnswer("50");
  });
});
