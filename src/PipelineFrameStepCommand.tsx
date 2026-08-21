import { PipelineCommand } from "./PipelineCommand";
import { formulaToken, type FormulaReference } from "./lib/formulaReferences";

/** The shared one-line frame picker used by two-input Wrangle steps. */
export function PipelineFrameStepCommand({
  editorId,
  label,
  frameId,
  frames,
  focusToken,
  resolveName,
  onInvalid,
  onSelect,
}: {
  editorId: string;
  label: string;
  frameId: string;
  frames: Array<{ id: string; name: string }>;
  focusToken?: number;
  resolveName: (draft: string) => string | null;
  onInvalid: () => void;
  onSelect: (frameId: string, saveNow: boolean) => void;
}) {
  const frameName = frames.find((frame) => frame.id === frameId)?.name ?? "";
  const references: FormulaReference[] = frames.map((frame) => ({
    id: frame.id,
    label: frame.name,
    token: formulaToken(frame.name),
    kind: "value",
    detail: "frame",
  }));
  const update = (draft: string, saveNow: boolean) => {
    const frame = frames.find((candidate) => candidate.name === resolveName(draft));
    if (!frame) {
      if (saveNow) onInvalid();
      return;
    }
    onSelect(frame.id, saveNow);
  };
  return (
    <PipelineCommand
      editorId={editorId}
      label={label}
      initialDraft={frameName ? formulaToken(frameName) : ""}
      references={references}
      focusToken={focusToken}
      onChange={(draft) => update(draft, false)}
      onCommit={(draft) => update(draft, true)}
    />
  );
}
