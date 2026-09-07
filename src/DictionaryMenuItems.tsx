import { BookOpen } from "lucide-react";
import { dictionaryColumns } from "./lib/dictionaries";
import { formulaToken } from "./lib/formulaReferences";
import type { Column, DocumentView, FrameObject, Operation } from "./lib/types";

/** The menu authors an ordinary Wrangle formula; the table itself is the
 * only editor for dictionary entries. Twenty mappings remain twenty rows. */
export function DictionaryMenuItems({ document, frame, column, onMap, run, close }: {
  document: DocumentView; frame: FrameObject; column: Column | null;
  onMap: (formula: string) => void;
  run: (operation: Operation) => Promise<string | null>;
  close: () => void;
}) {
  const dictionaries = document.objects.filter((object): object is FrameObject =>
    object.kind === "frame" && object.id !== frame.id && dictionaryColumns(object) !== null);
  const selectedKey = column ?? frame.columns[0];
  return <>
    {frame.columns.length === 2 && !dictionaryColumns(frame) && selectedKey &&
      <button onClick={() => {
        close();
        void run({ type: "setUniqueKey", frameId: frame.id, columnIds: [selectedKey.id], enabled: true });
      }}><BookOpen size={14} /><span>Use as dictionary — key: {selectedKey.name}</span></button>}
    {column && <details>
      <summary>Map values…</summary>
      {dictionaries.length === 0 && <span>Create a dictionary, or mark a two-column table’s key as unique.</span>}
      {dictionaries.map((dictionary) => {
        const { key, value } = dictionaryColumns(dictionary)!;
        const prefix = formulaToken(dictionary.name);
        return <button key={dictionary.id} onClick={() => {
          close();
          onMap(`map_values(${formulaToken(column.name)}, ${prefix}.${formulaToken(key.name)}, ${prefix}.${formulaToken(value.name)})`);
        }}>Using {dictionary.name}</button>;
      })}
    </details>}
  </>;
}
