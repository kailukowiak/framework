import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";

// The smallest real connector crosses every important seam: the Data library
// saves a machine-local command connection, Rust launches that executable without
// a shell, stdout is staged as an artifact, and the engine renders its rows.
// `printf` is deliberately boring and local; the same contract is what lets a
// person's aws, psql, bq, or other authenticated CLI sit in its place.
describe("CLI connector", () => {
  it("imports CSV printed by a local command", async () => {
    const command = $("button*=Script…");
    await command.waitForClickable();
    await command.click();

    const dialog = $(".cli-connector-dialog");
    await dialog.waitForExist();
    await $("label*=Connection name").$("input").setValue("E2E printf");
    await $("label*=Executable").$("input").setValue("/usr/bin/printf");
    await $("label*=Arguments").$("textarea").setValue(
      "name,amount\\nAlpha,10\\nBeta,20\\n"
    );
    await $("label*=Source name or address").$("input").setValue("fixture.csv");
    await $("button*=Add").click();

    await dialog.waitForExist({ reverse: true });
    await $("div.cell-display*=Alpha").waitForExist();
    await $("div.cell-display*=10").waitForExist();
    await $("div.cell-display*=Beta").waitForExist();
    await $("div.cell-display*=20").waitForExist();
  });

  it("imports a ConnectorX SQL result into the Parquet-backed frame", async () => {
    await browser.keys([Key.Command, Key.Shift, "l"]);
    await $(".dataset-dialog").waitForExist();
    await $("button*=DB…").click();

    const dialog = $(".cli-connector-dialog");
    await dialog.waitForExist();
    await $("label*=Connection name").$("input").setValue("E2E SQLite");
    await $("label*=URI").$("input").setValue("sqlite:///tmp/framework-e2e-connector.sqlite");
    await $("label*=Table name").$("input").setValue("Database result");
    await $("label*=SQL").$("textarea").setValue(
      "select 'Gamma' as name, 30 as amount union all select 'Delta', 40"
    );
    await $("button*=Add").click();

    await dialog.waitForExist({ reverse: true });
    await $("div.cell-display*=Gamma").waitForExist();
    await $("div.cell-display*=30").waitForExist();
    await $("div.cell-display*=Delta").waitForExist();
    await $("div.cell-display*=40").waitForExist();
  });
});
