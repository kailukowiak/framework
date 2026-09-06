import { browser, $ } from "@wdio/globals";
import { Key } from "webdriverio";
import { expect } from "expect-webdriverio";
import { closeDataLibrary } from "../lib/helpers";

// A container is a heading with things kept under it. At its default size
// the three add buttons once took every pixel and the member list rendered
// at zero height, so "+ Value" looked like it did nothing while the value
// quietly landed in the document. The member has to be on screen.
describe("container", () => {
  it("shows a value added with + Value at the card's default size", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();

    await browser.keys([Key.Command, Key.Alt, "g"]);
    const container = $(".container-object");
    await container.waitForExist();
    await $(".container-object .container-empty").waitForDisplayed();

    await container.$("button*=Value").click();
    const member = $(".container-object .container-member");
    await member.waitForDisplayed();
    const height = await browser.execute(
      () =>
        document.querySelector<HTMLElement>(".container-object .container-member")
          ?.getBoundingClientRect().height ?? 0
    );
    expect(height).toBeGreaterThan(40);
    expect(await member.$("input").getValue()).toContain("Value");
  });
});
