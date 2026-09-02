/**
 * One case-insensitive substring pass over everything the document view
 * already holds.
 *
 * Find is answered from what is in memory wherever it can be, because that is
 * the difference between a palette that fills in as you type and one that
 * waits on a query. Everything here is therefore pure and synchronous: names,
 * columns, formulas, Scratchwork lines, and the cells of frames small enough
 * to have travelled with the view. The one thing it cannot see — a frame whose
 * rows exist only as pages — is asked of the engine instead, and merged in by
 * `useDocumentSearch`.
 */

import type {
  ComputedFrame,
  DataObject,
  DocumentView,
  FrameObject,
  RenderedFrameStep,
} from "./types";

export type FindHitKind = "object" | "column" | "cell" | "formula" | "line";

export type FindHit = {
  /** Stable within one result set: what the palette keys and selects by. */
  id: string;
  kind: FindHitKind;
  objectId: string;
  viewId?: string;
  columnId?: string;
  rowId?: string;
  /**
   * Display position of the row, for a hit in a frame read through pages.
   * Absent for a frame whose rows are in hand, where `rowId` is enough — the
   * grid finds the row without being told where to scroll.
   */
  rowIndex?: number;
  label: string;
  snippet: string;
};

/** Hits under the object that holds them, in the document's own object order. */
export type FindGroup = {
  objectId: string;
  name: string;
  hits: FindHit[];
};

/**
 * Enough to fill a palette several screens deep and stop. A document with a
 * ten-thousand-row frame matches "e" on nearly every cell, and the twenty
 * thousandth of those is not a result anybody is scrolling to — the answer to
 * too many matches is a longer query, not a longer list.
 */
export const MAX_FIND_HITS = 200;

const contains = (haystack: string, needle: string) =>
  haystack.toLowerCase().includes(needle);

/** What kind of thing a hit sits on, for the one-word snippet on a name hit. */
const OBJECT_KIND_LABELS: Record<string, string> = {
  frame: "Frame",
  value: "Value",
  result: "Result",
  block: "Scratchwork",
  text: "Text",
  plot: "Plot",
  series: "Vector",
  container: "Container",
  calculationMatrix: "Calculation matrix",
};

/** Every formula a rendered chain step spells out, in the order it spells them. */
function stepFormulas(step: RenderedFrameStep): { label: string; text: string }[] {
  switch (step.kind) {
    case "filter":
      return step.predicates.map((text) => ({ label: "Filter", text }));
    case "withColumns":
      return step.columns.map((column) => ({ label: "Calculated", text: column.formula }));
    case "summarize":
      return [...step.groupKeys, ...step.aggregates].map((entry) => ({
        label: "Summarize",
        text: entry.formula,
      }));
    case "broadcast":
      return [{ label: "Vector", text: step.vector }];
    case "zipVector":
      return [{ label: "Vector", text: step.vector }];
    case "comment":
      return [{ label: "Note", text: step.text }];
    default:
      return [];
  }
}

/**
 * Every match in `document`, in the order the objects sit in the document.
 *
 * `view` is the same document — it is passed separately because the caller
 * holds one object and this reads two different halves of it, and naming both
 * says which half each scan is answering from.
 */
export function searchDocument(
  document: DocumentView,
  view: DocumentView,
  query: string
): FindHit[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const hits: FindHit[] = [];
  const viewIdFor = (objectId: string) =>
    document.views.find((candidate) => candidate.objectId === objectId)?.id ??
    document.views.find((candidate) => candidate.tabObjectIds?.includes(objectId))?.id;

  const push = (hit: Omit<FindHit, "id">) => {
    if (hits.length >= MAX_FIND_HITS) return false;
    hits.push({ ...hit, id: `${hit.kind}:${hits.length}:${hit.objectId}` });
    return true;
  };

  for (const object of document.objects) {
    const viewId = viewIdFor(object.id);
    if (contains(object.name, needle)) {
      if (
        !push({
          kind: "object",
          objectId: object.id,
          viewId,
          label: object.name,
          snippet: OBJECT_KIND_LABELS[object.kind] ?? object.kind,
        })
      )
        return hits;
    }
    if (object.kind === "block") {
      if (!scanBlock(object, view, needle, viewId, push)) return hits;
    }
    if (object.kind === "frame") {
      if (!scanFrame(object, view, needle, viewId, push)) return hits;
    }
  }
  return hits;
}

type Push = (hit: Omit<FindHit, "id">) => boolean;

function scanBlock(
  block: Extract<DataObject, { kind: "block" }>,
  view: DocumentView,
  needle: string,
  viewId: string | undefined,
  push: Push
): boolean {
  // The computed lines rather than the stored ones: a line's name is only
  // resolved once the block has been parsed, and the text is the same text.
  const computed = view.computedBlocks[block.id];
  const lines = computed
    ? computed.lines.map((line) => ({ name: line.name, text: line.text }))
    : block.lines.map((line) => ({ name: line.name, text: line.source }));
  for (const line of lines) {
    const text = line.text;
    if (!contains(text, needle) && !contains(line.name, needle)) continue;
    if (
      !push({
        kind: "line",
        objectId: block.id,
        viewId,
        label: line.name || "Line",
        snippet: text,
      })
    )
      return false;
  }
  return true;
}

function scanFrame(
  frame: FrameObject,
  view: DocumentView,
  needle: string,
  viewId: string | undefined,
  push: Push
): boolean {
  const computed = view.computedFrames[frame.id];
  const at = { frame, needle, viewId, push };
  if (!scanColumnNames(at)) return false;
  if (!computed) return true;
  if (!scanFrameFormulas(at, computed)) return false;
  // A paged frame's rows are not here to be searched — `useDocumentSearch`
  // asks the engine for those, and a half-answer from the handful of rows
  // that happen to be cached would be worse than none.
  if (computed.paged) return true;
  return scanCells(at, computed);
}

/** The three cell-level scans share exactly these four facts. */
type FrameScan = {
  frame: FrameObject;
  needle: string;
  viewId: string | undefined;
  push: Push;
};

function scanColumnNames({ frame, needle, viewId, push }: FrameScan): boolean {
  for (const column of frame.columns) {
    if (!contains(column.name, needle)) continue;
    if (
      !push({
        kind: "column",
        objectId: frame.id,
        viewId,
        columnId: column.id,
        label: column.name,
        snippet: "Column",
      })
    )
      return false;
  }
  return true;
}

function scanFrameFormulas(
  { frame, needle, viewId, push }: FrameScan,
  computed: ComputedFrame
): boolean {
  const columnName = (columnId: string) =>
    frame.columns.find((column) => column.id === columnId)?.name ?? "Column";
  for (const [columnId, formula] of Object.entries(computed.formulas ?? {})) {
    if (!contains(formula, needle)) continue;
    if (
      !push({
        kind: "formula",
        objectId: frame.id,
        viewId,
        columnId,
        label: columnName(columnId),
        snippet: formula,
      })
    )
      return false;
  }
  for (const step of computed.steps ?? []) {
    for (const { label, text } of stepFormulas(step)) {
      if (!contains(text, needle)) continue;
      if (!push({ kind: "formula", objectId: frame.id, viewId, label, snippet: text }))
        return false;
    }
  }
  return true;
}

function scanCells(
  { frame, needle, viewId, push }: FrameScan,
  computed: ComputedFrame
): boolean {
  for (const row of frame.rows) {
    const cells = computed.rows[row.id];
    if (!cells) continue;
    for (const column of frame.columns) {
      const display = cells[column.id]?.display ?? "";
      if (!display || !contains(display, needle)) continue;
      if (
        !push({
          kind: "cell",
          objectId: frame.id,
          viewId,
          columnId: column.id,
          rowId: row.id,
          label: column.name,
          snippet: display,
        })
      )
        return false;
    }
  }
  return true;
}

/**
 * Hits under their object, in the order `searchDocument` produced them — which
 * is the order the objects sit in the document, so the palette's groups do not
 * reshuffle as somebody types.
 */
export function groupFindHits(document: DocumentView, hits: FindHit[]): FindGroup[] {
  const names = new Map(document.objects.map((object) => [object.id, object.name]));
  const groups: FindGroup[] = [];
  for (const hit of hits) {
    let group = groups.find((candidate) => candidate.objectId === hit.objectId);
    if (!group) {
      group = {
        objectId: hit.objectId,
        name: names.get(hit.objectId) ?? "Untitled",
        hits: [],
      };
      groups.push(group);
    }
    group.hits.push(hit);
  }
  return groups;
}
