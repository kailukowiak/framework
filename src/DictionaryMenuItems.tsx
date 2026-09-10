import { BookOpen } from "lucide-react";
import { dictionaryColumns } from "./lib/dictionaries";
import { MappingFrameMenu } from "./MappingFrameMenu";
import type { Column, DocumentView, FrameObject, Operation } from "./lib/types";

/** The menu authors an ordinary Wrangle formula; the table itself is the
 * only editor for dictionary entries. Twenty mappings remain twenty rows. */
export function DictionaryMenuItems({ document, frame, column, position, onMap, run, close }: {
  document: DocumentView; frame: FrameObject; column: Column | null;
  position: { x: number; y: number };
  onMap: (formula: string) => void;
  run: (operation: Operation) => Promise<string | null>;
  close: () => void;
}) {
  const selectedKey = column ?? frame.columns[0];
  return <>
    {frame.columns.length === 2 && !dictionaryColumns(frame) && selectedKey &&
      <button onClick={() => {
        close();
        void run({ type: "setUniqueKey", frameId: frame.id, columnIds: [selectedKey.id], enabled: true });
      }}><BookOpen size={14} /><span>Use for mapping — key: {selectedKey.name}</span></button>}
    {column && <MappingFrameMenu {...{ document, frame, column, position, onMap, run, close }} />}
    <MappingFrameMenu {...{ document, frame, column, position, onMap, run, close }} rename />
  </>;
}
