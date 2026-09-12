import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";
import { closeDataLibrary, waitForGutterAnswer } from "../lib/helpers";

describe("financial Scratchwork", () => {
  it("prices a loan and recomputes its payment after an assumption changes", async () => {
    await $('[aria-label="Free-form data canvas"]').waitForExist();
    await closeDataLibrary();
    await browser.keys([Key.Command, "j"]);
    const source = $('textarea[aria-label$=" lines"]');
    await source.waitForExist();
    const loan = (principal: number) =>
      `principal = ${principal}\npayment = (-principal).finance.pmt(0.06/12, 60).round(2)\ninterest = finance.ipmt(0.06/12, 1, 60, -principal)\nprincipal paid = ppmt(0.06/12, 1, 60, -principal).round(2)`;
    await source.setValue(loan(400000));
    await waitForGutterAnswer("7733.12");
    await waitForGutterAnswer("2000");
    await waitForGutterAnswer("5733.12");
    await source.setValue(loan(200000));
    await waitForGutterAnswer("3866.56");
    await waitForGutterAnswer("1000");
  });

  it("discounts dated flows using the first date as time zero", async () => {
    const source = $('textarea[aria-label$=" lines"]');
    await source.setValue(
      "value = [-100, 121].finance.xnpv(0.1, [date(2025,1,1), date(2027,1,1)]).round(2)"
    );
    await waitForGutterAnswer("0");
  });

  it("solves periodic and dated returns and follows a changed cash flow", async () => {
    const source = $('textarea[aria-label$=" lines"]');
    const returns = (amount: number) =>
      `proceeds = ${amount}\nperiodic = (finance.irr([-100, proceeds]) * 100).round(2)\nannual = ([-100, proceeds].finance.xirr([date(2025,1,1), date(2027,1,1)]) * 100).round(2)`;
    await source.setValue(returns(121));
    await waitForGutterAnswer("21");
    await waitForGutterAnswer("10");
    await source.setValue(returns(144));
    await waitForGutterAnswer("44");
    await waitForGutterAnswer("20");
  });
});
