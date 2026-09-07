import "./Markdown.css";
import { Fragment, useContext, type ReactNode } from "react";
import { NumberDisplayContext } from "./FrameGrid";
import type { ComputedTextSegment } from "./lib/bindings/ComputedTextSegment";
import { formatComputedScalar } from "./lib/columnFormatting";

/**
 * A deliberately small markdown: headings, lists, paragraphs, bold, italic,
 * inline and fenced code, and http(s) links. Rendered straight to React nodes — no
 * HTML string ever exists, so there is nothing to sanitize and no way for
 * a document to smuggle markup into the app.
 *
 * A relative-path link or image (no `http(s):` scheme) has nothing inside the
 * app to point at, so it renders as text only — its link text, or its alt
 * text for an image — instead of the raw `[text](path)` / `![alt](path)`
 * syntax or a broken image element. http(s) links and images are unaffected.
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
//
// Link and image targets are matched for any scheme, http(s) or not, so the
// handler below can tell the two apart: an http(s) target keeps its existing
// rendering, and a relative one — nothing the app can navigate to inside
// FrameWork — collapses to its link text (a link) or its alt text (an image)
// instead of printing the raw markdown syntax. See renderInline.
const INLINE_TOKEN =
  // eslint-disable-next-line no-control-regex
  /(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*|_[^_]+_|!\[[^\]]*\]\([^)\s]+\)|\[[^\]]+\]\([^)\s]+\)|\u0000\d+\u0000)/g;

// An image: an http(s) source keeps the prior (unimplemented) rendering —
// printed verbatim, since nothing in the app fetches remote images — while a
// relative path names a file with nothing to load it from, so it renders as
// its alt text and nothing else, rather than a broken image icon or the raw
// `![alt](path)` syntax.
//
// A link with an http(s) target renders as before, a clickable anchor. A
// relative target has nothing inside the app to navigate to, so it shows
// only the link text (as code if it was written as code) with the target
// kept as a title, rather than an inert bracket-and-paren pair the reader
// would otherwise try to click.
function renderLinkOrImage(piece: string, key: string): ReactNode | undefined {
  const image = piece.match(/^!\[([^\]]*)\]\(([^)\s]+)\)$/);
  if (image) {
    if (/^https?:/i.test(image[2])) return <Fragment key={key}>{piece}</Fragment>;
    return <em key={key}>{image[1]}</em>;
  }
  const link = piece.match(/^\[([^\]]+)\]\(([^)\s]+)\)$/);
  if (!link) return undefined;
  if (/^https?:/i.test(link[2]))
    return (
      <a key={key} href={link[2]} target="_blank" rel="noreferrer noopener">
        {link[1]}
      </a>
    );
  const codeText = link[1].match(/^`([^`]+)`$/);
  return (
    <span key={key} title={link[2]}>
      {codeText ? <code>{codeText[1]}</code> : link[1]}
    </span>
  );
}

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
      const linkOrImage = renderLinkOrImage(piece, key);
      if (linkOrImage !== undefined) return linkOrImage;
      return <Fragment key={key}>{piece}</Fragment>;
    });
}

type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "list"; ordered: boolean; items: string[] }
  | { kind: "table"; header: string[]; rows: string[][] }
  | { kind: "code"; text: string }
  | { kind: "paragraph"; text: string };

// A pipe row splits on unescaped pipes, with the optional leading and
// trailing pipe dropped so `| a | b |` and `a | b` give the same cells.
function tableCells(line: string): string[] {
  const trimmed = line.trim().replace(/^\|/, "").replace(/\|$/, "");
  return trimmed.split("|").map((cell) => cell.trim());
}

const TABLE_DIVIDER = /^\s*\|?\s*:?-{1,}:?\s*(\|\s*:?-{1,}:?\s*)*\|?\s*$/;

function fencedCode(lines: string[], start: number): { text: string; end: number } {
  const code: string[] = [];
  let end = start + 1;
  while (end < lines.length && !/^```\s*$/.test(lines[end])) {
    code.push(lines[end]);
    end += 1;
  }
  return { text: code.join("\n"), end };
}

function parseBlocks(source: string): Block[] {
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length)
      blocks.push({ kind: "paragraph", text: paragraph.join(" ") });
    paragraph = [];
  };
  const lines = source.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    // Formula examples contain literal backticks and newlines. Keep them out
    // of inline markdown so the displayed example can be copied unchanged.
    if (/^```[^`]*$/.test(line)) {
      flush();
      const code = fencedCode(lines, index);
      index = code.end;
      blocks.push({ kind: "code", text: code.text });
      continue;
    }
    // A header row is only a table if the next line is the dash divider —
    // otherwise a sentence containing a pipe would become a one-cell table.
    if (
      line.includes("|") &&
      index + 1 < lines.length &&
      TABLE_DIVIDER.test(lines[index + 1]) &&
      lines[index + 1].includes("-")
    ) {
      flush();
      const header = tableCells(line);
      const rows: string[][] = [];
      index += 2;
      while (index < lines.length && lines[index].includes("|")) {
        rows.push(tableCells(lines[index]));
        index += 1;
      }
      index -= 1;
      blocks.push({ kind: "table", header, rows });
      continue;
    }
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
        if (block.kind === "code") return <pre key={key}><code>{block.text}</code></pre>;
        if (block.kind === "heading") {
          const Tag = (["h1", "h2", "h3", "h4"] as const)[block.level - 1];
          return <Tag key={key}>{renderInline(block.text, values, key)}</Tag>;
        }
        if (block.kind === "table") {
          return (
            <table key={key}>
              <thead>
                <tr>
                  {block.header.map((cell, cellIndex) => (
                    <th key={cellIndex}>
                      {renderInline(cell, values, `${key}.h${cellIndex}`)}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {block.rows.map((row, rowIndex) => (
                  <tr key={rowIndex}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex}>
                        {renderInline(cell, values, `${key}.${rowIndex}.${cellIndex}`)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          );
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
