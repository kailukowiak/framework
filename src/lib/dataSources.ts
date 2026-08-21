// The document read as a list of data sources rather than a canvas.
//
// The canvas answers "what is where"; this answers "where does any of this
// come from" — which file, which frame, which of them is out of date, and
// which can be typed into. Grouped by where the values originate, because
// that is the property every management action hangs off: a connected file
// can be refreshed or repointed, a derived frame can be cached, and a frame
// someone typed in has no source to manage at all.

import type { ComputedFrame, ConnectorRecipe, DocumentView, FrameObject } from "./types";

export function connectorSourceLabel(connector: ConnectorRecipe): string {
  if (connector.kind === "file") return connector.sourcePath;
  return connector.kind === "cli" ? connector.sourceLabel : connector.sourceName;
}

/**
 * Where a frame's values come from: its own, or another frame's.
 *
 * Base covers everything the document did not compute — a file it reads, a
 * file it copied in, a grid somebody typed. Those differ in history rather
 * than in behaviour, and what separates them in practice is the second axis,
 * not this one.
 */
export type DataOrigin = "base" | "derived";

/**
 * Whether the values can move without anyone editing the document.
 *
 * Refreshable means there is somewhere to re-read from, so asking will
 * replace what is there; static means the numbers are the numbers. A derived
 * frame inherits the answer from what it reads, because a frame computed from
 * a file moves when the file does even though nothing in its own definition
 * touches a file.
 *
 * Ordered by how much the ground can move, so a third value has a place to
 * go: live, when values arrive without being asked for, sits above
 * refreshable the way refreshable sits above static.
 */
export type DataRefresh = "refreshable" | "static";

/** The two of them, which is the whole answer to "what kind of frame is this". */
export interface DataNature {
  origin: DataOrigin;
  refresh: DataRefresh;
}

/** The four combinations, as one string for a colour or a group to key on. */
export type DataSourceKind =
  | "base-refreshable"
  | "base-static"
  | "derived-refreshable"
  | "derived-static";

export interface DataSourceEntry {
  frame: FrameObject;
  computed?: ComputedFrame;
  kind: DataSourceKind;
  /** The line under the name: the file, the parent frame, or a row count. */
  detail: string;
  /** The long form of `detail` — a full path — when there is one. */
  title?: string;
  /** Its own snapshot has fallen behind. */
  stale: boolean;
  /** Something it reads from has. */
  upstreamStale: boolean;
  cached: boolean;
  /** Whether values can be typed into it. */
  editable: boolean;
  /**
   * Whether its values can move without anyone editing the document.
   *
   * Inherited: a frame computed from a connected file is live even though
   * nothing in its own definition reads a file. Worth showing on a derived
   * frame for exactly that reason — the group it sits in does not say it.
   */
  live: boolean;
}

export interface DataSourceGroup {
  kind: DataSourceKind;
  title: string;
  /** The two axes the heading is written in, so it can carry their colours. */
  nature: DataNature;
  entries: DataSourceEntry[];
}

const GROUP_TITLES: Record<DataSourceKind, string> = {
  "base-refreshable": "Base · Refreshable",
  "base-static": "Base · Static",
  "derived-refreshable": "Derived · Refreshable",
  "derived-static": "Derived · Static",
};

/** The order groups are shown in: closest to the outside world first. */
const GROUP_ORDER: DataSourceKind[] = [
  "base-refreshable",
  "base-static",
  "derived-refreshable",
  "derived-static",
];

/**
 * What each kind is, in one line, for whatever is showing a colour.
 *
 * A colour is a fast way to tell four things apart and a poor way to learn
 * what they are, so everything that carries one carries this too — as a
 * tooltip on the canvas, as the reading of a legend in the sidebar. Stated
 * by consequence, like the import dialog: what a frame *does* is the part
 * worth knowing, not which field is set on it.
 */
export const DATA_SOURCE_LABELS: Record<DataSourceKind, string> = {
  "base-refreshable": "Read from a file — refreshing replaces its values",
  "base-static": "Held in this document — these values are the document's own",
  "derived-refreshable": "Computed from data that can change under it",
  "derived-static": "Computed from data that does not move",
};

/**
 * What a frame is, on both axes at once.
 *
 * Refreshability is read from the computed side rather than re-derived here:
 * the core already works out that a frame is live, inheritance included, and
 * a second opinion about it computed from `connector` alone would disagree
 * with the first one about every derived frame.
 */
export function dataNature(
  frame: FrameObject,
  computed?: ComputedFrame
): DataNature {
  return {
    origin: frame.derivation ? "derived" : "base",
    refresh:
      computed?.live || (!frame.derivation && frame.connector) ? "refreshable" : "static",
  };
}

export function dataSourceKind(
  frame: FrameObject,
  computed?: ComputedFrame
): DataSourceKind {
  const nature = dataNature(frame, computed);
  return `${nature.origin}-${nature.refresh}`;
}

/**
 * Every frame in the document, grouped by where its values come from.
 *
 * Empty groups are dropped rather than shown empty: a heading with nothing
 * under it reads as something missing rather than something absent.
 */
export function groupDataSources(document: DocumentView): DataSourceGroup[] {
  const frames = document.objects.filter(
    (object): object is FrameObject => object.kind === "frame"
  );
  const named = frameNames(document);
  const entries = frames.map((frame): DataSourceEntry => {
    const computed = document.computedFrames[frame.id];
    const kind = dataSourceKind(frame, computed);
    return {
      frame,
      computed,
      kind,
      detail: detailFor(frame, computed, dataNature(frame, computed).origin, named),
      title: longFormFor(frame),
      stale: Boolean(computed?.materialization?.stale),
      upstreamStale: Boolean(computed?.upstreamStale),
      cached: Boolean(computed?.materialization),
      editable: Boolean(computed?.editing?.cells),
      live: Boolean(computed?.live),
    };
  });

  return GROUP_ORDER.map((kind) => {
    const [origin, refresh] = kind.split("-") as [DataOrigin, DataRefresh];
    return {
      kind,
      title: GROUP_TITLES[kind],
      nature: { origin, refresh },
      entries: entries.filter((entry) => entry.kind === kind),
    };
  }).filter((group) => group.entries.length > 0);
}

/** What a card says about itself when the canvas is too far out to read it. */
export interface FrameOutline {
  nature: DataNature;
  /** "1,204 rows", or "computed on read" when nothing knows yet. */
  rows: string;
  /** "8 columns". */
  columns: string;
  /** Where the values come from: a file, or the frame above it. */
  source: string;
  /** How many filters in this frame's chain are narrowing its rows, if any. */
  filters: number;
}

/**
 * A frame reduced to the four things worth knowing about it from across the
 * room: what it is called, what kind of thing it is, how much of it there is,
 * and where it came from.
 *
 * Deliberately the same vocabulary the sources sidebar uses — "from Ledger",
 * "ledger.csv", "computed on read". A card that describes itself one way when
 * zoomed out and another way in the sidebar is two facts to learn instead of
 * one.
 */
export function outlineFrame(
  frame: FrameObject,
  computed: ComputedFrame | undefined,
  named: Map<string, string>,
  filters = 0
): FrameOutline {
  const columns = frame.columns.length;
  return {
    nature: dataNature(frame, computed),
    rows: rowsLabel(frame, computed),
    columns: `${columns.toLocaleString()} ${columns === 1 ? "column" : "columns"}`,
    source: detailFor(frame, computed, dataNature(frame, computed).origin, named),
    filters,
  };
}

/** The nature as two words, for anywhere with the room to say it out loud. */
export function natureWords(nature: DataNature): string {
  return `${nature.origin} · ${nature.refresh}`;
}

/** The frame names in a document, for the "from <parent>" line. */
export function frameNames(document: DocumentView): Map<string, string> {
  return new Map(
    document.objects
      .filter((object): object is FrameObject => object.kind === "frame")
      .map((frame) => [frame.id, frame.name])
  );
}

function detailFor(
  frame: FrameObject,
  computed: ComputedFrame | undefined,
  origin: DataOrigin,
  named: Map<string, string>
): string {
  if (origin === "derived") {
    const parent = frame.derivation && named.get(frame.derivation.sourceFrameId);
    return parent ? `from ${parent}` : "from a frame that is gone";
  }
  const file = computed?.sourceName ?? frame.artifact?.sourceName;
  if (file) return file;
  return rowsLabel(frame, computed);
}

function longFormFor(frame: FrameObject): string | undefined {
  if (frame.connector) return connectorSourceLabel(frame.connector);
  return frame.sourceFile ?? frame.artifact?.path ?? undefined;
}

/**
 * A row count, when one is known without running a query.
 *
 * A derived frame reports none until something reads it, and inventing one
 * here would mean running the transformation to fill in a subtitle.
 */
function rowsLabel(frame: FrameObject, computed: ComputedFrame | undefined): string {
  const rows =
    computed?.totalRows ??
    computed?.materialization?.rowCount ??
    (frame.derivation ? undefined : frame.rows.length);
  if (rows === undefined) return "computed on read";
  return `${rows.toLocaleString()} ${rows === 1 ? "row" : "rows"}`;
}
