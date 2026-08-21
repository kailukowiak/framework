import { browser, $ } from "@wdio/globals";
import { expect } from "expect-webdriverio";
import { pointAtCell, resetAndOpenTutorial } from "../lib/helpers";

type StoredScale = {
  text?: { low?: string; high?: string } | null;
  fill?: { low?: string; high?: string } | null;
} | undefined;

// Conditional formatting end to end: the Format inspector writes a rule, the
// core compiles it into a hidden column over the rows being read, and the
// grid repaints from the answer. Every link in that chain is somewhere a
// unit test cannot reach — the operation is validated in Rust against the
// frame's real schema, the rule runs in Polars, and the styles arrive on the
// page the cells were rendered from. The spec asserts the painted cells,
// because a rule that stores correctly and colors nothing is the exact bug
// this feature already had.
describe("conditional formatting", () => {
  it("paints the rows a rule answers true for", async () => {
    await resetAndOpenTutorial("Month-over-month formulas by pointing — Start");
    await $("div.cell-display*=142,000").waitForExist();

    // Selecting a cell is what gives the inspector a frame and a column to
    // seed the rule from — the same gesture that aims the direct format
    // controls.
    await pointAtCell("142,000");
    // The inspector is what a selection opens; if it did not, say what is on
    // screen instead, because "button not clickable" names the symptom and
    // not the cause.
    try {
      await $(".inspector").waitForExist({ timeout: 8000 });
    } catch {
      const onScreen = await browser.execute(() => ({
        dialog: Boolean(document.querySelector(".dataset-dialog")),
        cells: document.querySelectorAll("td.styled-frame-cell").length,
        buttons: Array.from(document.querySelectorAll("button"))
          .map((button) => button.getAttribute("aria-label") ?? button.textContent ?? "")
          .slice(0, 40),
      }));
      throw new Error(`no inspector after selecting a cell: ${JSON.stringify(onScreen)}`);
    }
    const format = $('button[aria-label="Format"]');
    await format.waitForClickable();
    await format.click();

    const addRule = $('button[aria-label="Add rule"]');
    await addRule.waitForClickable();
    // The control opens on the press, like every other menu here, so a
    // WebDriver click's mousedown is not enough to reach it.
    await browser.execute(() => {
      document
        .querySelector<HTMLElement>('button[aria-label="Add rule"]')
        ?.dispatchEvent(
          new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
        );
    });
    const heatmap = $("button*=Heatmap");
    await heatmap.waitForClickable();
    await heatmap.click();

    // A ramp paints a position between 0 and 1, and the preset writes the
    // mapping into the formula rather than leaving numbers on the rule. That
    // formula has to survive the round trip through the engine and come back
    // as the text somebody could edit -- if it renders differently from what
    // was sent, every rule in the document quietly rewrites itself on read.
    await browser.waitUntil(
      async () =>
        browser.execute(
          () =>
            document.querySelector<HTMLTextAreaElement>(".style-rule-head textarea")
              ?.value ?? ""
        ).then((value) => value.includes(".normalize()")),
      { timeout: 8000, timeoutMsg: "the heatmap preset never wrote a normalized formula" }
    );

    // The heatmap preset is a ramp over the selected column: every row takes
    // a fill somewhere between the two ends. That is the assertion — a
    // colored column, not a stored rule.
    try {
      await browser.waitUntil(
        async () =>
          browser.execute(() => {
            const painted = Array.from(
              document.querySelectorAll<HTMLElement>("td.styled-frame-cell")
            ).filter((cell) => cell.style.backgroundColor !== "");
            return painted.length > 1;
          }),
        { timeout: 8000 }
      );
    } catch {
      // The panel reports a refused rule inline, and that message is the
      // whole diagnosis — worth carrying into the failure rather than
      // leaving somebody to reproduce this by hand.
      const refusals = await browser.execute(() =>
        Array.from(document.querySelectorAll(".style-rule-error")).map(
          (node) => node.textContent
        )
      );
      throw new Error(
        `no cell was ever painted by the rule. Panel said: ${refusals.join(" / ") || "nothing"}`
      );
    }

    // Rewriting the formula as a question rewrites the reading: a yes-or-no
    // answer picks rows rather than shading all of them, and the panel takes
    // it without anybody choosing a rule kind. The workbook's six months run
    // 118,000 to 168,000, so this threshold has rows on both sides of it.
    const formula = $(".style-rule-head textarea");
    await formula.waitForExist();
    await formula.click();
    await formula.setValue("`Revenue` > 140000");
    // Leaving the field is the commit, which is the gesture a person makes
    // without thinking about it. ⌘↵ commits too; this asserts the one that
    // has to work for someone who never learned the other.
    await browser.execute(() => {
      const editor = document.querySelector<HTMLTextAreaElement>(
        ".style-rule-head textarea"
      );
      editor?.blur();
    });

    // The assertion has to separate a condition from the ramp it replaced,
    // and "some cells painted" cannot: a ramp scoped to one column already
    // leaves every other column bare. Within the ruled column, though, a
    // ramp paints every row and a condition paints only the rows it matches.
    const paintedInRuledColumn = () =>
      browser.execute(() => {
        const column = document.querySelector<HTMLElement>(
          "td.styled-frame-cell[style*='background']"
        )?.dataset.columnId;
        const cells = Array.from(
          document.querySelectorAll<HTMLElement>(
            `td.styled-frame-cell[data-column-id="${column}"]`
          )
        );
        return {
          total: cells.length,
          painted: cells.filter((cell) => cell.style.backgroundColor !== "").length,
        };
      });
    try {
      await browser.waitUntil(
        async () => {
          const { total, painted } = await paintedInRuledColumn();
          return total > 1 && painted > 0 && painted < total;
        },
        { timeout: 8000 }
      );
    } catch {
      const refusals = await browser.execute(() =>
        Array.from(document.querySelectorAll(".style-rule-error")).map(
          (node) => node.textContent
        )
      );
      const editorState = await browser.execute(() => {
        const editor = document.querySelector<HTMLTextAreaElement>(
          ".style-rule-head textarea"
        );
        return {
          value: editor?.value ?? "<no editor>",
          focused: document.activeElement === editor,
          active: document.activeElement?.tagName ?? "?",
        };
      });
      throw new Error(
        `the rewritten rule never narrowed to the rows it matches (${JSON.stringify(
          await paintedInRuledColumn()
        )}, editor ${JSON.stringify(editorState)}). Panel said: ${
          refusals.join(" / ") || "nothing"
        }`
      );
    }
  });

  // The headline of "auto": a rule over text arrives already holding the
  // values the column has, each with a color, and every square of that
  // mapping is editable afterwards. Nothing about it can be checked without
  // the engine — the panel asks what the formula answers, and only Polars
  // over the real rows can say — so the whole thing lives here.
  it("fills a category rule with the values the column actually has", async () => {
    // Region is East on four rows and West on two, so a filled rule has
    // exactly two values in it and they are those two.
    await pointAtCell("West");
    const format = $('button[aria-label="Format"]');
    await format.waitForClickable();
    await format.click();
    await browser.execute(() => {
      document
        .querySelector<HTMLElement>('button[aria-label="Add rule"]')
        ?.dispatchEvent(
          new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
        );
    });
    const byValue = $("button*=A color per value");
    await byValue.waitForClickable();
    await byValue.click();

    // The stops are the mapping. Two values plus the catch-all, each value
    // wearing a fill of its own — that is what nobody had to type.
    const stops = () =>
      browser.execute(() =>
        Array.from(document.querySelectorAll<HTMLElement>(".style-rule-stop")).map(
          (stop) => ({
            label: stop.textContent?.replace("123", "").trim() ?? "",
            fill:
              stop.querySelector<HTMLElement>(".style-rule-swatch")?.style
                .backgroundColor ?? "",
          })
        )
      );
    try {
      await browser.waitUntil(
        async () => {
          const filled = await stops();
          return (
            filled.filter((stop) => stop.label === "East" || stop.label === "West")
              .length === 2
          );
        },
        { timeout: 8000 }
      );
    } catch {
      const refusals = await browser.execute(() =>
        Array.from(document.querySelectorAll(".style-rule-error")).map(
          (node) => node.textContent
        )
      );
      throw new Error(
        `the rule never filled itself from the data (stops ${JSON.stringify(
          await stops()
        )}). Panel said: ${refusals.join(" / ") || "nothing"}`
      );
    }
    const filled = await stops();
    const east = filled.find((stop) => stop.label === "East");
    const west = filled.find((stop) => stop.label === "West");
    // Two values, two colors: a mapping that paints both the same is not one.
    expect(east?.fill).toBeTruthy();
    expect(west?.fill).toBeTruthy();
    expect(east?.fill).not.toEqual(west?.fill);

    // And the grid is painted from it. Found by what the cells say rather
    // than by column id: the earlier rule is still on this frame and still
    // painting Revenue, so "the first painted column" would be the wrong
    // column. Two values, two fills, and the same value the same fill --
    // which is the whole claim a mapping makes.
    const byLabel = await browser.execute(() => {
      const painted: Record<string, string[]> = { East: [], West: [] };
      for (const cell of Array.from(
        document.querySelectorAll<HTMLElement>("td.styled-frame-cell")
      )) {
        const label = cell.querySelector("div.cell-display")?.textContent?.trim() ?? "";
        if (label in painted) painted[label].push(cell.style.backgroundColor);
      }
      return painted;
    });
    expect(new Set(byLabel.East).size).toBe(1);
    expect(new Set(byLabel.West).size).toBe(1);
    expect(byLabel.East[0]).toBeTruthy();
    expect(byLabel.East[0]).not.toEqual(byLabel.West[0]);
  });

  // Colors are stored once, as the light-mode value, and go to CSS as a
  // `light-dark()` pair so the window follows the system theme with nothing
  // to subscribe to. That only works if WebKit resolves the function inside
  // an inline style -- and if it does not, the whole declaration is dropped
  // and every painted cell silently loses its color. Nothing but the real
  // webview can answer that, so it is asked here.
  //
  // Both halves are checked by overriding `color-scheme` on the element,
  // which is what decides the branch: same stored value, two resolved
  // colors, neither of them nothing.
  it("resolves a stored color to a different one in each theme", async () => {
    const painted = await browser.execute(() => {
      const cell = Array.from(
        document.querySelectorAll<HTMLElement>("td.styled-frame-cell")
      ).find((candidate) => candidate.style.backgroundColor !== "");
      if (!cell) return null;
      const declared = cell.style.backgroundColor;
      const read = (scheme: string) => {
        cell.style.colorScheme = scheme;
        return getComputedStyle(cell).backgroundColor;
      };
      const light = read("light");
      const dark = read("dark");
      cell.style.removeProperty("color-scheme");
      return { declared, light, dark };
    });
    if (!painted) throw new Error("no cell was painted to read a color off");
    // The declaration survived the parser: a `light-dark()` WebKit could not
    // read would leave `style.backgroundColor` empty, and the cell would be
    // unpainted rather than wrongly painted.
    expect(painted.declared).toContain("light-dark(");
    // And it resolves, differently, on both sides -- an unsupported function
    // computes to the transparent default instead of a color.
    for (const resolved of [painted.light, painted.dark]) {
      expect(resolved).toMatch(/^rgba?\(/);
      expect(resolved).not.toBe("rgba(0, 0, 0, 0)");
    }
    expect(painted.light).not.toBe(painted.dark);
  });

  // The bug this exists to keep out: text and fill used to compete for one
  // ramp. Choosing the second property silently discarded the first, even
  // though a readable ink ramp over a heatmap is an ordinary thing to ask.
  //
  // Only the real panel can prove this -- the swatch, the merge, the
  // operation and the stored rule are four separate pieces and the bug lived
  // in the seam between them.
  it("keeps independent text and fill ramps on the same numeric rule", async () => {
    await pointAtCell("142,000");
    const format = $('button[aria-label="Format"]');
    await format.waitForClickable();
    await format.click();

    // A ramp of its own to aim at: the first test rewrote its heatmap into a
    // question, so the only rules on this frame paint by condition and by
    // value and neither has ends.
    await browser.execute(() => {
      document
        .querySelector<HTMLElement>('button[aria-label="Add rule"]')
        ?.dispatchEvent(
          new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button: 0 })
        );
    });
    const heatmap = $("button*=Heatmap");
    await heatmap.waitForClickable();
    await heatmap.click();

    const lowest = $("button*=lowest");
    await lowest.waitForClickable();
    await lowest.click();
    await $('button[aria-label^="Set text color"]').waitForExist();

    const scales = async () => {
      const stored = await browser.execute(() => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const w = window as any;
        w.__e2eRampProbe = undefined;
        w.__TAURI__.core
          .invoke("get_document")
          .then((view: unknown) => (w.__e2eRampProbe = view));
        return true;
      });
      expect(stored).toBe(true);
      await browser.waitUntil(
        async () =>
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          browser.execute(() => (window as any).__e2eRampProbe !== undefined),
        { timeoutMsg: "get_document never answered" }
      );
      return browser.execute(() => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const view = (window as any).__e2eRampProbe;
        const frames = Array.isArray(view?.objects)
          ? view.objects.filter((object: { kind?: string }) => object?.kind === "frame")
          : [];
        return frames
          .flatMap(
            (frame: {
              display?: {
                styleRules?: Array<{
                  output?: {
                    kind?: string;
                    scale?: {
                      text?: { low?: string; high?: string } | null;
                      fill?: { low?: string; high?: string } | null;
                    };
                  };
                }>;
              };
            }) => frame.display?.styleRules ?? []
          )
          .filter((rule: { output?: { kind?: string } }) => rule.output?.kind === "scale")
          .map((rule: {
            output?: {
              scale?: {
                text?: { low?: string; high?: string } | null;
                fill?: { low?: string; high?: string } | null;
              };
            };
          }) => rule.output?.scale);
      });
    };

    // Text, then fill, then text again: each edit keeps the other channel's
    // endpoints intact.
    const click = async (label: string) => {
      const swatch = $(`button[aria-label^="${label}"]`);
      await swatch.waitForClickable();
      await swatch.click();
    };
    await click("Set text color Coral ink");
    await browser.waitUntil(async () => (await scales()).some((scale: StoredScale) => scale?.text), {
      timeout: 8000,
      timeoutMsg: `a text ramp was not stored: ${JSON.stringify(await scales())}`,
    });
    await click("Set fill color Sage");
    await browser.waitUntil(async () => (await scales()).some((scale: StoredScale) => scale?.text && scale.fill), {
      timeout: 8000,
      timeoutMsg: `text and fill did not coexist: ${JSON.stringify(await scales())}`,
    });
    await click("Set text color Sage ink");
    await browser.waitUntil(async () => (await scales()).some(
      (scale: StoredScale) => scale?.text?.low === "#174829" && Boolean(scale.fill)
    ), {
      timeout: 8000,
      timeoutMsg: `recolouring text disturbed fill: ${JSON.stringify(await scales())}`,
    });
  });

  it("keeps the rule on the document", async () => {
    const stored = await browser.execute(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      w.__e2eRulesProbe = undefined;
      w.__TAURI__.core
        .invoke("get_document")
        .then((view: unknown) => (w.__e2eRulesProbe = view))
        .catch((reason: unknown) => (w.__e2eRulesProbe = { failure: String(reason) }));
      return true;
    });
    expect(stored).toBe(true);
    await browser.waitUntil(
      async () =>
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        browser.execute(() => (window as any).__e2eRulesProbe !== undefined),
      { timeoutMsg: "get_document never answered" }
    );
    const rules = await browser.execute(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const view = (window as any).__e2eRulesProbe;
      const frames = Array.isArray(view?.objects)
        ? view.objects.filter(
            (object: { kind?: string }) => object?.kind === "frame"
          )
        : [];
      return frames.flatMap(
        (frame: { display?: { styleRules?: Array<{ output?: { kind?: string } }> } }) =>
          (frame.display?.styleRules ?? []).map((rule) => rule.output?.kind ?? "?")
      );
    });
    expect(rules).toEqual(["condition", "category", "scale"]);
  });
});
