import type {
  Dispatch,
  PointerEvent as ReactPointerEvent,
  ReactNode,
  RefObject,
  SetStateAction,
} from "react";
import type {
  CellPointerHandler,
  FrameStyleMatches,
  GridFocus,
  GridFocusMode,
  RenderedGrid,
} from "./FrameGrid";
import type { GridDirection, GridRange } from "./lib/gridNavigation";
import type { OperationHandler } from "./lib/handlers";
import type {
  CanvasView,
  Column,
  ComputedFrame,
  Row,
  Selection,
  SortKey,
  FrameObject,
} from "./lib/types";

export type FrameCardProps = {
  view: CanvasView;
  frame: FrameObject;
  computed: ComputedFrame;
  selection: Selection | null;
  gridFocus: GridFocus | null;
  onSelect: (selection: Selection) => void;
  onGridFocus: Dispatch<SetStateAction<GridFocus | null>>;
  onGridStep: (direction: GridDirection) => void;
  onRenderedRows: (frameId: string, grid: RenderedGrid | null) => void;
  onOperation: OperationHandler;
  onRearrangeColumns: (frameId: string, columnIds: string[]) => void;
  onFilterColumn: (frame: FrameObject, column: Column) => void;
  onTransformColumn: (frame: FrameObject, column: Column, formula: string) => void;
  onEditCalculatedColumn: (
    frame: FrameObject,
    column: Column,
    rowIndex: number
  ) => void;
  dataRefreshRevision: number;
};

type VirtualRange = {
  start: number;
  end: number;
  paddingTop: number;
  paddingBottom: number;
};

export type RecordsAsRowsFrameCardProps = {
  frame: FrameObject;
  computed: ComputedFrame;
  selection: Selection | null;
  gridFocus: GridFocus | null;
  displayedRows: Row[];
  /** What the frame's conditional-formatting rules made of each row. */
  styleMatches: FrameStyleMatches;
  visibleRows: Row[];
  virtualRange: VirtualRange;
  selectionRange: GridRange | null;
  filterMark: {
    weight: "unfiltered" | "structural";
    count: number;
    reading: string;
  };
  filterPredicateCount: number;
  filterPredicates: string[];
  transformationLabels: Array<string | null>;
  sortKeys: SortKey[];
  totalRows: number;
  isDerived: boolean;
  isFileBacked: boolean;
  isTransposed: boolean;
  isReadOnly: boolean;
  canAddRows: boolean;
  canAddColumns: boolean;
  pagedLoading: boolean;
  placeholderOffsets: Set<number>;
  pagedStatus: ReactNode;
  draftRow: Record<string, string>;
  setDraftRow: Dispatch<SetStateAction<Record<string, string>>>;
  scrollRef: RefObject<HTMLDivElement | null>;
  pendingScrollTop: RefObject<number>;
  scrollFrame: RefObject<number | null>;
  setScrollState: Dispatch<SetStateAction<{ top: number; height: number }>>;
  frameColumnDrop: { columnId: string; after: boolean } | null;
  onOperation: OperationHandler;
  onSelect: (selection: Selection) => void;
  selectWholeColumn: (event: ReactPointerEvent, column: Column) => void;
  beginFrameColumnDrag: (event: ReactPointerEvent, columnId: string) => void;
  selectWholeRow: (event: ReactPointerEvent, row: Row) => void;
  beginCellSelection: CellPointerHandler;
  extendCellSelection: CellPointerHandler;
  focusCell: (
    row: Row,
    column: Column,
    mode: GridFocusMode,
    options?: { extend?: boolean; span?: GridFocus["span"] }
  ) => void;
  commitCellEdit: (
    row: Row,
    column: Column,
    raw: string,
    move: GridDirection | null
  ) => void;
  settleCellEdit: (row: Row, column: Column) => void;
  commitDraftRow: (allowEmpty?: boolean) => void;
  addColumn: (afterColumnId: string | null) => void;
  editCalculatedColumn: (column: Column, rowIndex: number) => void;
  filterColumn: (column: Column) => void;
};
