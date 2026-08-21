import { useEffect, useRef, useState } from "react";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import { CrosstabTable } from "./CrosstabTable";
import { GeneratorRuleRow } from "./GeneratorRuleRow";
import { RecordsFrameBody } from "./RecordsFrameBody";
import { RecordsFrameHeader } from "./RecordsFrameHeader";
import {
  FrameSummaryDrawer,
  useFrameSummary,
} from "./FrameSummaryFooter";
import { FrameTitleRow } from "./FrameTitleRow";

export function RecordsAsRowsFrameCard(model: RecordsAsRowsFrameCardProps) {
  const {
    frame,
    totalRows,
    scrollRef,
    pendingScrollTop,
    scrollFrame,
    setScrollState,
    pagedStatus,
  } = model;
  const drawerOpen = frame.display?.summaryDrawerOpen ?? false;
  const persistedHeight = frame.display?.summaryDrawerHeight ?? 150;
  const [drawerHeight, setDrawerHeight] = useState(persistedHeight);
  const summaryScrollRef = useRef<HTMLDivElement>(null);
  const summaryDrawerRef = useRef<HTMLElement>(null);
  const summary = useFrameSummary(frame, model.computed.fingerprint, drawerOpen);
  useEffect(() => setDrawerHeight(persistedHeight), [persistedHeight]);
  useEffect(() => {
    if (drawerOpen && summaryScrollRef.current && scrollRef.current)
      summaryScrollRef.current.scrollLeft = scrollRef.current.scrollLeft;
  }, [drawerOpen, scrollRef, frame.columns.length]);

  const setSummaryRows = (operations: import("./lib/types").SummaryOperation[]) => {
    void model.onOperation({
      type: "setFrameSummaryRows",
      frameId: frame.id,
      summaryRows: operations,
    });
  };
  const setDrawer = (open: boolean, height = drawerHeight) => {
    void model.onOperation({
      type: "setFrameSummaryDrawer",
      frameId: frame.id,
      open,
      height,
    });
  };
  const beginDrawerResize = (event: React.PointerEvent<HTMLButtonElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const drawer = summaryDrawerRef.current;
    const startHeight = drawer?.offsetHeight ?? drawerHeight;
    const renderedHeight = drawer?.getBoundingClientRect().height ?? startHeight;
    const canvasScale = drawer?.offsetHeight ? renderedHeight / drawer.offsetHeight : 1;
    const startY = event.clientY;
    let nextHeight = startHeight;
    const move = (moveEvent: PointerEvent) => {
      nextHeight = Math.max(
        72,
        Math.min(600, Math.round(startHeight + (startY - moveEvent.clientY) / canvasScale))
      );
      setDrawerHeight(nextHeight);
    };
    const end = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      setDrawer(true, nextHeight);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };
  return (
    <div className="frame-card" data-frame-id={frame.id}>
      <FrameTitleRow
        model={model}
        summaryDrawerOpen={drawerOpen}
        onToggleSummaryDrawer={() => setDrawer(!drawerOpen)}
      />
      {model.computed.generatorRule !== undefined && (
        <GeneratorRuleRow
          frameId={frame.id}
          rule={model.computed.generatorRule}
          onOperation={model.onOperation}
        />
      )}
      <div
        className="frame-scroll"
        ref={scrollRef}
        onScroll={(event) => {
          if (summaryScrollRef.current)
            summaryScrollRef.current.scrollLeft = event.currentTarget.scrollLeft;
          pendingScrollTop.current = event.currentTarget.scrollTop;
          if (scrollFrame.current !== null) return;
          scrollFrame.current = requestAnimationFrame(() => {
            scrollFrame.current = null;
            const top = pendingScrollTop.current;
            setScrollState((current) =>
              current.top === top ? current : { ...current, top }
            );
          });
        }}
      >
        {/* A crosstab replaces the records table inside the same shell: it
            is a way of looking at the same rows, not a different card. The
            display setting is ignored for paged frames — spreading needs
            every row on this side of the wire. */}
        {frame.display?.crosstab && !model.isFileBacked ? (
          <CrosstabTable
            frame={frame}
            rows={model.displayedRows}
            crosstab={frame.display.crosstab}
            onOperation={model.onOperation}
          />
        ) : (
          <table
            aria-rowcount={
              totalRows + 3
            }
            style={{ minWidth: Math.max(360, frame.columns.length * 150 + 66) }}
          >
            <colgroup>
              <col className="row-number-column" />
              {frame.columns.map((column) => (
                <col key={column.id} />
              ))}
              <col className="frame-edge-column" />
            </colgroup>
            <RecordsFrameHeader model={model} />
            <RecordsFrameBody model={model} />
          </table>
        )}
      </div>
      {pagedStatus}
      {drawerOpen && (
        <FrameSummaryDrawer
          frame={frame}
          summary={summary}
          height={drawerHeight}
          drawerRef={summaryDrawerRef}
          scrollRef={summaryScrollRef}
          onScroll={(scrollLeft) => {
            if (scrollRef.current) scrollRef.current.scrollLeft = scrollLeft;
          }}
          onResize={beginDrawerResize}
          onSetRows={setSummaryRows}
        />
      )}
    </div>
  );
}
