import { useEffect, useState } from "react";
import { CombineDialog } from "./CombineDialog";
import { LookupJoinPrompt } from "./LookupJoinPrompt";
import type { OperationHandler } from "./lib/handlers";
import type { DocumentView } from "./lib/types";
import type { JoinState } from "./lib/joinState";

/**
 * The entry into Combine.
 *
 * A gesture that already carried columns across has answered most of the
 * question, so it still gets the compact lookup prompt first: two keys, the
 * diagnostics, and one button. Everything else — the frame menu, Quick
 * Commands — opens Combine directly, where the relationship is the first
 * control rather than an assumption. "More options" on the prompt is the
 * seam between the two, and lands on *Match on a key* because that is the
 * relationship the gesture was already describing.
 */
export function JoinDialog({
  state,
  document,
  onClose,
  onOperation,
  onCreated,
  onChained,
}: {
  state: NonNullable<JoinState>;
  document: DocumentView;
  onClose: () => void;
  onOperation: OperationHandler;
  onCreated: () => void;
  onChained?: (frameId: string) => void;
}) {
  const [moreOptions, setMoreOptions] = useState(false);
  // Escape closes, same as every other dialog; handled once here so both
  // the compact prompt and the full surface get it.
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);
  if (state.lookupOutputColumnIds?.length && !moreOptions)
    return (
      <LookupJoinPrompt
        state={state}
        document={document}
        onClose={onClose}
        onMoreOptions={() => setMoreOptions(true)}
        onOperation={onOperation}
        onCreated={onCreated}
      />
    );
  return (
    <CombineDialog
      state={state}
      document={document}
      relationship="match"
      onClose={onClose}
      onOperation={onOperation}
      onCreated={onCreated}
      onChained={onChained}
    />
  );
}
