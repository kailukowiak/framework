import { Fragment, useContext, type ReactNode } from "react";
import { NumberDisplayContext } from "./FrameGrid";
import type { ComputedTextSegment } from "./lib/bindings/ComputedTextSegment";
import { formatComputedScalar } from "./lib/columnFormatting";

/**
 * A deliberately small markdown: headings, lists, paragraphs, bold, italic,
 * inline code, and http(s) links. Rendered straight to React nodes — no
 * HTML string ever exists, so there is nothing to sanitize and no way for
 * a document to smuggle markup into the app.
 *
 * Values interleave through sentinels: a `\u0000N\u0000` token in the
 * source renders as `values[N]`, which is how a text card's `{{…}}` holes
 * end up inside sentences, list items, and headings without the block
 * parser having to know about them.
 */
const SENTINEL = "\u0000";

/**
 * A failed hole must say that it failed. Showing the formula source in its
 * place makes a rendered card look as though interpolation never ran, while
 * hiding the complaint in a title makes the only useful information depend on
 * discovering a hover. The card itself is the error surface: ordinary inline
 * text at the exact point where the answer would otherwise appear. Clicking
 * the card still opens the untouched source for repair.
 */
function TextHoleError({ source, error }: { source: string; error: string }) {
  const message = error.replace(/^Formula error:\s*/i, "");
  return (
    <span className="text-hole broken" title={`{{${source}}}`}>
      <span className="text-hole-error-label">Formula error:</span> {message}
    </span>
  );
}

// The sentinel is a control character *because* no keyboard can put one in
// a document — any printable sentinel could collide with real prose.
const INLINE_TOKEN =
  // eslint-disable-next-line no-control-regex
  /(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|_[^_]+_|\[[^\]]+\]\((?:https?:)[^)\s]+\)|\u0000\d+\u0000)/g;

function renderInline(
  text: string,
  values: Map<number, ReactNode> | undefined,
  keyBase: string
): ReactNode[] {
  return text
    .split(INLINE_TOKEN)
    .filter((piece) => piece !== "")
    .map((piece, index) => {
      const key = `${keyBase}.${index}`;
      if (piece.startsWith(SENTINEL) && piece.endsWith(SENTINEL)) {
        const value = values?.get(Number(piece.slice(1, -1)));
        return <Fragment key={key}>{value ?? null}</Fragment>;
      }
      if (piece.startsWith("`") && piece.endsWith("`") && piece.length > 2)
        return <code key={key}>{piece.slice(1, -1)}</code>;
      if (piece.startsWith("**") && piece.endsWith("**") && piece.length > 4)
        return (
          <strong key={key}>
            {renderInline(piece.slice(2, -2), values, key)}
          </strong>
        );
      if (
        (piece.startsWith("*") && piece.endsWith("*") && piece.length > 2) ||
        (piece.startsWith("_") && piece.endsWith("_") && piece.length > 2)
      )
        return (
          <em key={key}>{renderInline(piece.slice(1, -1), values, key)}</em>
        );
      const link = piece.match(/^\[([^\]]+)\]\((https?:[^)\s]+)\)$/);
      if (link)
        return (
          <a key={key} href={link[2]} target="_blank" rel="noreferrer noopener">
            {link[1]}
          </a>
        );
      return <Fragment key={key}>{piece}</Fragment>;
    });
}

type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "list"; ordered: boolean; items: string[] }
  | { kind: "paragraph"; text: string };

function parseBlocks(source: string): Block[] {
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length)
      blocks.push({ kind: "paragraph", text: paragraph.join(" ") });
    paragraph = [];
  };
  for (const line of source.split("\n")) {
    const heading = line.match(/^(#{1,4})\s+(.*)$/);
    if (heading) {
      flush();
      blocks.push({
        kind: "heading",
        level: heading[1].length,
        text: heading[2],
      });
      continue;
    }
    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    const numbered = line.match(/^\s*\d+[.)]\s+(.*)$/);
    if (bullet || numbered) {
      flush();
      const ordered = Boolean(numbered);
      const item = (bullet ?? numbered)?.[1] ?? "";
      const last = blocks.at(-1);
      if (last?.kind === "list" && last.ordered === ordered) last.items.push(item);
      else blocks.push({ kind: "list", ordered, items: [item] });
      continue;
    }
    if (!line.trim()) {
      flush();
      continue;
    }
    paragraph.push(line);
  }
  flush();
  return blocks;
}

export function Markdown({
  source,
  values,
}: {
  source: string;
  values?: Map<number, ReactNode>;
}) {
  return (
    <div className="markdown-body">
      {parseBlocks(source).map((block, index) => {
        const key = `b${index}`;
        if (block.kind === "heading") {
          const Tag = (["h1", "h2", "h3", "h4"] as const)[block.level - 1];
          return <Tag key={key}>{renderInline(block.text, values, key)}</Tag>;
        }
        if (block.kind === "list") {
          const Tag = block.ordered ? "ol" : "ul";
          return (
            <Tag key={key}>
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>
                  {renderInline(item, values, `${key}.${itemIndex}`)}
                </li>
              ))}
            </Tag>
          );
        }
        return <p key={key}>{renderInline(block.text, values, key)}</p>;
      })}
    </div>
  );
}

/**
 * A text card's content: the computed segments woven back into one
 * markdown document, each hole rendering as its live answer.
 */
export function TextMarkdown({
  segments,
}: {
  segments: ComputedTextSegment[];
}) {
  const useGrouping = useContext(NumberDisplayContext);
  let source = "";
  const values = new Map<number, ReactNode>();
  segments.forEach((segment, index) => {
    if (segment.kind === "literal") {
      source += segment.text;
      return;
    }
    source += `${SENTINEL}${index}${SENTINEL}`;
    if (segment.kind === "value") {
      values.set(
        index,
        segment.error ? (
          <TextHoleError source={segment.formula} error={segment.error} />
        ) : (
          <span className="text-hole" title={segment.formula}>
            {formatComputedScalar(
              segment.typedValue,
              segment.dataType,
              segment.display,
              useGrouping
            )}
          </span>
        )
      );
    } else {
      values.set(
        index,
        <TextHoleError source={segment.source} error={segment.error} />
      );
    }
  });
  return <Markdown source={source} values={values} />;
}
