import { useRef, useState } from "react";
import { Markdown } from "./Markdown";

/** A markdown remark that edits in place and commits on blur. */
export function CommentStepRow({
  text,
  startEditing,
  onCommit,
}: {
  text: string;
  startEditing: boolean;
  onCommit: (text: string) => void;
}) {
  const [editing, setEditing] = useState(startEditing);
  const [draft, setDraft] = useState(text);
  const cancelled = useRef(false);
  if (!editing)
    return (
      <button
        type="button"
        className="pipeline-comment"
        title="Edit comment"
        onClick={() => {
          setDraft(text);
          setEditing(true);
        }}
      >
        <Markdown source={text} />
      </button>
    );
  return (
    <textarea
      className="pipeline-comment-editor"
      value={draft}
      autoFocus
      rows={Math.max(2, draft.split("\n").length)}
      placeholder="Why the chain does what it does — markdown allowed"
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => {
        setEditing(false);
        onCommit(cancelled.current ? text : draft);
        cancelled.current = false;
      }}
      onKeyDown={(event) => {
        if ((event.metaKey || event.ctrlKey) && event.key === "Enter")
          event.currentTarget.blur();
        if (event.key === "Escape") {
          cancelled.current = true;
          event.currentTarget.blur();
        }
      }}
    />
  );
}
