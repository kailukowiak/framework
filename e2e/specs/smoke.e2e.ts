import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import { closeDataLibrary, selectCard } from "../lib/helpers";

// A bare launch lands on a fresh scratch document with the Data library open
// over it — initial_session() in src-tauri guarantees the scratch is new and
// empty, so this spec can assert the launch state without cleaning anything.
describe("launch", () => {
  it("opens the scratch canvas behind the Data library", async () => {
    await expect(browser).toHaveTitle(expect.stringContaining("FrameWork"));

    const canvas = $('[aria-label="Free-form data canvas"]');
    await canvas.waitForExist();
  });

  // One real IPC round trip before any feature spec exists: the answer has
  // to come from the Rust store, not from anything the page could invent.
  // "Untitled" is BLANK_DOCUMENT_TITLE in src-tauri — the scratch document a
  // bare launch creates.
  it("answers get_document from the real engine", async () => {
    // Invoke-then-poll rather than an async script: the embedded server's
    // execute endpoint does not await a returned promise, so the answer is
    // parked on window and read back once it lands.
    await browser.execute(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      w.__e2eDocumentProbe = undefined;
      w.__TAURI__.core
        .invoke("get_document")
        .then((view: unknown) => (w.__e2eDocumentProbe = view))
        .catch((reason: unknown) => (w.__e2eDocumentProbe = { failure: String(reason) }));
    });
    await browser.waitUntil(
      async () =>
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        browser.execute(() => (window as any).__e2eDocumentProbe !== undefined),
      { timeoutMsg: "get_document never answered" }
    );
    // Explicit sentinels, because the WebDriver JSON bridge turns undefined
    // into null and an assertion on "no value" would pass for the wrong
    // reasons — or never pass at all.
    const document = await browser.execute(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const view = (window as any).__e2eDocumentProbe;
      return {
        failure: typeof view?.failure === "string" ? view.failure : "none",
        name: typeof view?.name === "string" ? view.name : "<missing>",
        objectCount: Array.isArray(view?.objects) ? view.objects.length : -1,
      };
    });
    expect(document.failure).toBe("none");
    expect(document.name).toBe("Untitled");
    expect(document.objectCount).toBe(0);
  });

  // ⌘⇧L mirrors the Data Library menu accelerator in menu-less shells. This
  // was a real gap the persistence spec ran into: with the dialog closed,
  // the launch state and the native menu used to be the only ways in.
  it("reopens the Data library with ⌘⇧L", async () => {
    await closeDataLibrary();
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist();
  });

  it("adds, arranges, collapses, and fits ordinary canvas cards", async () => {
    await closeDataLibrary();
    await browser.keys([Key.Command, Key.Alt, "g"]);
    await browser.keys([Key.Command, Key.Alt, "b"]);
    await browser.keys([Key.Command, Key.Alt, "f"]);
    const container = $(".container-object");
    await container.waitForExist();
    const frame = $(".frame-object");
    await frame.waitForExist();

    await browser.keys([Key.Command, Key.Shift, "a"]);
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const cards = Array.from(document.querySelectorAll<HTMLElement>(".canvas-object"));
          return new Set(cards.map((card) => `${card.offsetLeft}:${card.offsetTop}`)).size > 1;
        }),
      { timeoutMsg: "the Arrange shortcut did not lay out the cards" }
    );

    await selectCard(".container-object");
    await browser.waitUntil(
      async () => (await container.getAttribute("class"))?.includes("selected"),
      { timeoutMsg: "pointing at the container did not select it" }
    );
    await browser.keys([Key.Tab]);
    await browser.waitUntil(
      async () => !(await container.getAttribute("class"))?.includes("selected"),
      { timeoutMsg: "Tab did not cycle the selected canvas card" }
    );

    await selectCard(".frame-object");
    await browser.keys([Key.Command, "2"]);
    // By accessible name: the tab's visible text is capitalized by the
    // stylesheet, so matching on rendered text was matching the casing the
    // CSS happened to leave.
    await expect($('.inspector-nav button[aria-label="Format"]')).toHaveElementClass(
      "active"
    );
    await browser.keys([Key.Command, "3"]);
    await expect($('.inspector-nav button[aria-label="Wrangle"]')).toHaveElementClass(
      "active"
    );

    await $('[aria-label="Collapse Container"]').click();
    await $('[aria-label="Expand Container"]').waitForExist();
    await $('[aria-label="Expand Container"]').click();

    await selectCard(".container-object");
    await browser.keys([Key.Command, Key.Shift, "f"]);
    await browser.waitUntil(
      async () =>
        browser.execute(() => {
          const card = document.querySelector<HTMLElement>(".container-object");
          const viewport = document.querySelector<HTMLElement>(".canvas-viewport");
          return Boolean(card && viewport && card.offsetWidth >= viewport.clientWidth - 60);
        }),
      { timeoutMsg: "the Fit to Window shortcut did not resize the container" }
    );
  });
});
