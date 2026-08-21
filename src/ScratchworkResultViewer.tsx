import { Check, Copy, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { getBlockLinePage } from "./lib/api";
import { writeClipboardText } from "./lib/clipboard";
import type { ComputedBlockLine } from "./lib/types";

/**
 * The gutter is deliberately one line wide. This is the place an answer can
 * spend space when it actually needs it, without making every scratchwork
 * line pay for the longest one in the block.
 */
export function ScratchworkResultViewer({
  line,
  blockId,
  onClose,
}: {
  line: ComputedBlockLine;
  blockId: string;
  onClose: () => void;
}) {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");
  const [values, setValues] = useState<string[] | null>(null);
  const [totalValues, setTotalValues] = useState<number | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const loading = useRef(false);
  const request = useRef(0);

  const load = useCallback(
    async (offset: number, reset = false) => {
      if (loading.current) return;
      loading.current = true;
      const current = request.current;
      try {
        const page = await getBlockLinePage(blockId, line.id, offset, 200);
        if (request.current !== current) return;
        setValues((before) => (reset || before === null ? page.values : [...before, ...page.values]));
        setTotalValues(page.totalValues);
        setFailure(null);
      } catch (reason) {
        if (request.current === current)
          setFailure(String(reason).replace(/^Error:\s*/, ""));
      } finally {
        if (request.current === current) loading.current = false;
      }
    },
    [blockId, line.id]
  );

  useEffect(() => {
    request.current += 1;
    loading.current = false;
    setCopyState("idle");
    setValues(null);
    setTotalValues(null);
    setFailure(null);
    void load(0, true);
    return () => {
      request.current += 1;
      loading.current = false;
    };
  }, [blockId, line.id, line.display, load]);

  const copy = async () => {
    let text = line.display;
    try {
      const first = await getBlockLinePage(blockId, line.id, 0, 1);
      if (first.totalValues > 1) {
        // Copy is an explicit request for the whole answer. Fetch it once,
        // rather than walking pages and recomputing the expression for each.
        const whole = await getBlockLinePage(blockId, line.id, 0, first.totalValues);
        text = whole.values.join("\n");
      }
    } catch {
      // The compact answer is still worth copying if a fresh evaluation
      // failed between opening the viewer and pressing Copy.
    }
    const written = await writeClipboardText(text);
    setCopyState(written ? "copied" : "failed");
    if (written) window.setTimeout(() => setCopyState("idle"), 1800);
  };

  return (
    <section className="scratchwork-result-viewer" aria-label="Scratchwork result">
      <header>
        <strong>{line.name || "Result"}</strong>
        <span>
          {line.dataType}
          {totalValues !== null && totalValues > 1 ? ` · ${totalValues} values` : ""}
        </span>
        <button type="button" onClick={() => void copy()}>
          {copyState === "copied" ? <Check size={13} /> : <Copy size={13} />}
          {copyState === "copied"
            ? "Copied"
            : copyState === "failed"
              ? "Copy failed"
              : "Copy"}
        </button>
        <button type="button" className="icon-button" aria-label="Close result" onClick={onClose}>
          <X size={14} />
        </button>
      </header>
      <pre
        tabIndex={0}
        onScroll={(event) => {
          const node = event.currentTarget;
          if (
            values !== null &&
            totalValues !== null &&
            values.length < totalValues &&
            node.scrollHeight - node.scrollTop - node.clientHeight < 40
          )
            void load(values.length);
        }}
      >
        {values?.join("\n") ?? line.display}
      </pre>
      {failure && <small className="result-error">{failure}</small>}
    </section>
  );
}

/** A cue only where the gutter is visibly withholding part of the answer. */
export function scratchworkResultIsLong(display: string): boolean {
  return display.length > 20 || display.includes("\n");
}
