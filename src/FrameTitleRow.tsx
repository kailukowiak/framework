import { CircleAlert, Database, Filter, GitBranch, MessageSquare } from "lucide-react";
import { useState } from "react";
import { Markdown } from "./Markdown";
import { displayedSummaryRows } from "./FrameSummaryFooter";
import type { OperationHandler } from "./lib/handlers";
import type { RecordsAsRowsFrameCardProps } from "./FrameCardProps";
import { dataNature } from "./lib/dataSources";

/**
 * The frame's pinned remark, once its icon has been clicked: rendered
 * markdown, or a textarea while being written. Blur commits; committing
 * nothing clears the comment, so "no comment" is one state, not two.
 */
function FrameCommentPanel({
  frameId,
  comment,
  editing,
  onEdit,
  onCommitted,
  onOperation,
}: {
  frameId: string;
  comment: string | null;
  editing: boolean;
  onEdit: () => void;
  onCommitted: (cleared: boolean) => void;
  onOperation: OperationHandler;
}) {
  if (!editing)
    return (
      <button
        type="button"
        className="frame-comment"
        title="Edit comment"
        onClick={onEdit}
      >
        <Markdown source={comment ?? ""} />
      </button>
    );
  return (
    <textarea
      className="frame-comment-editor"
      autoFocus
      defaultValue={comment ?? ""}
      rows={Math.max(2, (comment ?? "").split("\n").length)}
      placeholder="What this frame is, for the next reader — markdown allowed"
      onBlur={(event) => {
        const text = event.target.value.trim();
        onCommitted(!text);
        if ((text || null) !== comment)
          void onOperation({
            type: "setFrameComment",
            frameId,
            comment: text || null,
          });
      }}
      onKeyDown={(event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === "Enter")
          event.currentTarget.blur();
      }}
    />
  );
}

function FrameNameField({
  frameId,
  name,
  onOperation,
}: {
  frameId: string;
  name: string;
  onOperation: OperationHandler;
}) {
  return (
    <input
      className="frame-name"
      size={Math.max(6, Math.min(30, name.length))}
      defaultValue={name}
      key={name}
      onBlur={(event) => {
        if (event.target.value !== name)
          void onOperation({
            type: "renameObject",
            objectId: frameId,
            name: event.target.value,
          });
      }}
    />
  );
}

function FrameSummaryToggle({
  open,
  configured,
  onToggle,
}: {
  open: boolean;
  configured: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      className={`frame-summary-toggle ${open ? "active" : ""} ${
        configured ? "configured" : ""
      }`}
      title={open ? "Hide profile drawer" : "Show profile drawer"}
      aria-expanded={open}
      onClick={onToggle}
    >
      Σ
    </button>
  );
}

function hasFrameSummary(frame: RecordsAsRowsFrameCardProps["frame"]): boolean {
  return displayedSummaryRows(frame).length > 0;
}

export function FrameTitleRow({
  model,
  summaryDrawerOpen,
  onToggleSummaryDrawer,
}: {
  model: RecordsAsRowsFrameCardProps;
  summaryDrawerOpen: boolean;
  onToggleSummaryDrawer: () => void;
}) {
  const {
    frame,
    computed,
    onOperation,
    filterMark,
    filterPredicateCount,
    transformationLabels,
    isDerived,
    isFileBacked,
    totalRows,
    displayedRows,
    pagedLoading,
  } = model;
  const [commentOpen, setCommentOpen] = useState(false);
  const [editingComment, setEditingComment] = useState(false);
  return (
    <>
      <div className="frame-title-row">
        {/* An input is 20 characters wide whatever is in it, so the explicit
            size makes this row space itself around the name it actually has. */}
        <FrameNameField frameId={frame.id} name={frame.name} onOperation={onOperation} />
        <span className="frame-title-meta">
          <button
            className="frame-orientation-toggle"
            title="Display fields as rows (glimpse)"
            onClick={() =>
              onOperation({
                type: "setFrameDisplayOrientation",
                frameId: frame.id,
                orientation: "fieldsAsRows",
              })
            }
          >
            Fields ↓
          </button>
          <FrameSummaryToggle
            open={summaryDrawerOpen}
            configured={hasFrameSummary(frame)}
            onToggle={onToggleSummaryDrawer}
          />
          {/* The comment is behind an icon because it is occasional reading,
              not a value: solid when the frame has one, faint when it is the
              gesture for adding one. */}
          <button
            className={`frame-comment-toggle ${frame.comment ? "has-comment" : ""}`}
            title={frame.comment ? "Comment" : "Add a comment"}
            onClick={() => {
              // Opening with no comment yet goes straight to writing one.
              setEditingComment(!commentOpen && !frame.comment);
              setCommentOpen(!commentOpen);
            }}
          >
            <MessageSquare size={12} />
          </button>
          {/* The filter is a mark rather than a clause in a sentence, so it
              can be found without reading the row — and it is always in the
              row, so "not filtered" is something the card says rather than
              something you infer from a gap. */}
          <span
            className={`frame-title-filtered ${filterMark.weight}`}
            title={filterMark.reading}
          >
            <Filter size={12} />
            {filterMark.count > 1 && filterMark.count}
          </span>
          {isDerived && (
            <span className="frame-title-lineage">
              <GitBranch size={12} /> {transformationLabels.join(" · ")}
            </span>
          )}
          <span>
            {isFileBacked
              ? `${totalRows.toLocaleString()} rows`
              : filterPredicateCount
              ? `${displayedRows.length.toLocaleString()} of ${frame.rows.length.toLocaleString()} rows`
              : `${frame.rows.length.toLocaleString()} rows`}
          </span>
          {/* Both axes, each with its own word and its own colour. One hue
              covering both meant "refreshable" showed up in two colours
              depending on where the data came from, which is a palette that
              teaches the wrong thing. */}
          <span className="nature-words">
            <span className={`origin-${dataNature(frame, computed).origin}`}>
              {dataNature(frame, computed).origin}
            </span>
            <i>·</i>
            <span className={`refresh-${dataNature(frame, computed).refresh}`}>
              {dataNature(frame, computed).refresh}
            </span>
          </span>
          {pagedLoading && <span>loading…</span>}
          {/* Whether a frame is cached, and whether that cache is behind its
              source, has to be readable from the canvas -- a stale number
              that looks live is the failure mode worth designing against. */}
          {computed.materialization && (
            <span
              className={`frame-cache-badge ${
                computed.materialization.stale ? "stale" : ""
              }`}
              title={
                computed.materialization.stale
                  ? "Cached, and the source has changed since. Refresh it from the Frame tab."
                  : "Cached to a snapshot; reads do not re-run the transformation."
              }
            >
              {computed.materialization.stale ? (
                <>
                  <CircleAlert size={10} /> cached · out of date
                </>
              ) : (
                <>
                  <Database size={10} /> cached
                </>
              )}
            </span>
          )}
          {/* A live frame has no snapshot to be out of date, which is
              exactly why this needs saying: it is reading one that is. */}
          {computed.upstreamStale && !computed.materialization?.stale && (
            <span
              className="frame-cache-badge stale"
              title="A frame this one reads from is serving an out-of-date snapshot, so these numbers are out of date too."
            >
              <CircleAlert size={10} /> reading old numbers
            </span>
          )}
        </span>
      </div>
      {commentOpen && (
        <FrameCommentPanel
          frameId={frame.id}
          comment={frame.comment ?? null}
          editing={editingComment}
          onEdit={() => setEditingComment(true)}
          onCommitted={(cleared) => {
            setEditingComment(false);
            if (cleared) setCommentOpen(false);
          }}
          onOperation={onOperation}
        />
      )}
    </>
  );
}
