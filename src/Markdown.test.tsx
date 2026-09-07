// @vitest-environment jsdom
import { render } from "@testing-library/react";
import { expect, it } from "vitest";
import { Markdown } from "./Markdown";

it("preserves copyable formula syntax and line breaks inside fenced examples", () => {
  const formula = 'label = lookup("Offce supplies", `Category fixes`.`Key`, `Category fixes`.`Value`)\nmissing = "<none>"';
  const { container } = render(<Markdown source={`# Example\n\n\`\`\`text\n${formula}\n\`\`\`\n\nKeep going.`} />);
  expect(container.querySelector("pre code")?.textContent).toBe(formula);
  expect(container.querySelector("pre em, pre none")).toBeNull();
  expect(container.querySelector("p")?.textContent).toBe("Keep going.");
});

it("renders a relative link as its text, with the target kept as a title", () => {
  const { container } = render(
    <Markdown source="Open [`grand-tour-start.fw`](grand-tour-start.fw) to begin." />
  );
  expect(container.querySelector("a")).toBeNull();
  const code = container.querySelector("p code");
  expect(code?.textContent).toBe("grand-tour-start.fw");
  expect(code?.closest("span")?.getAttribute("title")).toBe("grand-tour-start.fw");
});

it("renders a relative link with plain text as text only, no code styling", () => {
  const { container } = render(<Markdown source="See [the tutorial folder](tutorials/) for more." />);
  expect(container.querySelector("a")).toBeNull();
  expect(container.querySelector("p code")).toBeNull();
  const span = container.querySelector("p span[title]");
  expect(span?.textContent).toBe("the tutorial folder");
  expect(span?.getAttribute("title")).toBe("tutorials/");
});

it("still renders an http(s) link as a clickable anchor", () => {
  const { container } = render(<Markdown source="See [the docs](https://example.com/docs) for more." />);
  const anchor = container.querySelector("a");
  expect(anchor?.textContent).toBe("the docs");
  expect(anchor?.getAttribute("href")).toBe("https://example.com/docs");
});

it("renders a relative image as its alt text in italics, not a broken image", () => {
  const { container } = render(
    <Markdown source="![Starting workbook: two columns](screenshots/start.jpg)" />
  );
  expect(container.querySelector("img")).toBeNull();
  const em = container.querySelector("p em");
  expect(em?.textContent).toBe("Starting workbook: two columns");
});
