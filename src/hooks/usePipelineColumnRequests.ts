import { useCallback, useRef, useState } from "react";
import { formulaToken } from "../lib/formulaReferences";
import { needsDeclaredOrder } from "../lib/typedColumnFormula";
import type { Column, FrameObject, Selection } from "../lib/types";

export type AddCalculatedColumnRequest = {
  frameId: string;
  token: number;
  afterColumnId?: string;
  anchorRowIndex?: number;
} | null;
export type TransformColumnRequest = {
  frameId: string;
  columnId: string;
  formula: string;
  focus?: boolean;
  editExisting?: boolean;
  anchorRowIndex?: number;
  orderByColumnId?: string;
  focusAtEnd?: boolean;
  token: number;
} | null;
export type FilterColumnRequest = {
  frameId: string;
  columnId: string;
  token: number;
} | null;
export type HidePipelineColumnRequest = {
  frameId: string;
  columnId: string;
  token: number;
} | null;
export type RearrangeColumnsRequest = {
  frameId: string;
  columnIds: string[];
  token: number;
} | null;
export type ApplyVectorRequest = {
  frameId: string;
  columnIds: string[];
  vector: string;
  expectedLength: number;
  token: number;
} | null;
export type PairVectorRequest = {
  frameId: string;
  name: string;
  vector: string;
  expectedLength: number;
  token: number;
} | null;

// The editors that receive a request already know which frame they are
// editing, so the request they are handed drops the frameId that scoped it.
export type AddCalculatedColumnEditorRequest = Omit<
  NonNullable<AddCalculatedColumnRequest>,
  "frameId"
>;
export type TransformColumnEditorRequest = Omit<
  NonNullable<TransformColumnRequest>,
  "frameId"
>;
export type FilterColumnEditorRequest = Omit<
  NonNullable<FilterColumnRequest>,
  "frameId"
>;
export type HidePipelineColumnEditorRequest = Omit<
  NonNullable<HidePipelineColumnRequest>,
  "frameId"
>;
export type RearrangeColumnsEditorRequest = Omit<
  NonNullable<RearrangeColumnsRequest>,
  "frameId"
>;
export type ApplyVectorEditorRequest = Omit<
  NonNullable<ApplyVectorRequest>,
  "frameId"
>;
export type PairVectorEditorRequest = Omit<
  NonNullable<PairVectorRequest>,
  "frameId"
>;

function useMultiColumnPipelineRequests({
  setContextMenu,
  setSelection,
  setInspectorSection,
}: {
  setContextMenu: (value: null) => void;
  setSelection: (value: Selection) => void;
  setInspectorSection: (value: "wrangle" | { panel: "prepare"; section: "wrangle" }) => void;
}) {
  const [rearrangeColumnsRequest, setRearrangeColumnsRequest] =
    useState<RearrangeColumnsRequest>(null);
  const rearrangeToken = useRef(0);
  const [applyVectorRequest, setApplyVectorRequest] =
    useState<ApplyVectorRequest>(null);
  const applyToken = useRef(0);
  const [pairVectorRequest, setPairVectorRequest] = useState<PairVectorRequest>(null);
  const pairToken = useRef(0);

  const requestRearrangeColumns = useCallback(
    (frameId: string, columnIds: string[], viewId?: string) => {
      setSelection({ objectId: frameId, viewId });
      setInspectorSection("wrangle");
      rearrangeToken.current += 1;
      setRearrangeColumnsRequest({ frameId, columnIds, token: rearrangeToken.current });
    },
    [setSelection, setInspectorSection]
  );
  const requestApplyVector = useCallback(
    (
      frameId: string,
      columnIds: string[],
      vector: string,
      expectedLength: number,
      viewId?: string
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId, columnId: columnIds[0] });
      setInspectorSection("wrangle");
      applyToken.current += 1;
      setApplyVectorRequest({
        frameId,
        columnIds,
        vector,
        expectedLength,
        token: applyToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );
  const requestPairVector = useCallback(
    (
      frameId: string,
      name: string,
      vector: string,
      expectedLength: number,
      viewId?: string
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId });
      setInspectorSection("wrangle");
      pairToken.current += 1;
      setPairVectorRequest({
        frameId,
        name,
        vector,
        expectedLength,
        token: pairToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  return {
    rearrangeColumnsRequest,
    applyVectorRequest,
    pairVectorRequest,
    requestRearrangeColumns,
    requestApplyVector,
    requestPairVector,
    clearRearrangeColumnsRequest: () => setRearrangeColumnsRequest(null),
    clearApplyVectorRequest: () => setApplyVectorRequest(null),
    clearPairVectorRequest: () => setPairVectorRequest(null),
  };
}

function useAddCalculatedColumnRequest({
  setContextMenu,
  setSelection,
  setInspectorSection,
}: {
  setContextMenu: (value: null) => void;
  setSelection: (value: Selection) => void;
  setInspectorSection: (value: "wrangle" | { panel: "prepare"; section: "wrangle" }) => void;
}) {
  const [addCalculatedColumnRequest, setRequest] =
    useState<AddCalculatedColumnRequest>(null);
  const token = useRef(0);
  const requestAddCalculatedColumn = useCallback(
    (
      frameId: string,
      afterColumnId: string | undefined,
      anchorRowIndex: number | undefined,
      viewId?: string
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId, columnId: afterColumnId });
      setInspectorSection("wrangle");
      token.current += 1;
      setRequest({ frameId, token: token.current, afterColumnId, anchorRowIndex });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );
  return {
    addCalculatedColumnRequest,
    requestAddCalculatedColumn,
    clearAddCalculatedColumnRequest: () => setRequest(null),
  };
}

/**
 * Every doorway into an existing column's formula: the context menu's
 * transform, a formula begun in one of its cells, and the edit that reopens
 * the calculation a column already has. They all mint the same request and
 * differ only in what they know about the gesture that opened it.
 */
function useColumnTransformationRequests({
  setContextMenu,
  setSelection,
  setInspectorSection,
}: {
  setContextMenu: (value: null) => void;
  setSelection: (value: Selection) => void;
  setInspectorSection: (value: "wrangle" | { panel: "prepare"; section: "wrangle" }) => void;
}) {
  const [transformColumnRequest, setTransformColumnRequest] =
    useState<TransformColumnRequest>(null);
  const transformColumnToken = useRef(0);
  const openColumnTransformation = useCallback(
    (
      frame: FrameObject,
      viewId: string | undefined,
      request: Omit<NonNullable<TransformColumnRequest>, "frameId" | "token">,
      fromGrid = false
    ) => {
      setContextMenu(null);
      setSelection({ objectId: frame.id, viewId, columnId: request.columnId });
      setInspectorSection(fromGrid ? { panel: "prepare", section: "wrangle" } : "wrangle");
      transformColumnToken.current += 1;
      setTransformColumnRequest({
        ...request,
        frameId: frame.id,
        token: transformColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const requestColumnTransformation = useCallback(
    (
      frame: FrameObject,
      column: Column,
      formula: string,
      focus = false,
      viewId?: string,
      orderByColumnId?: string
    ) =>
      openColumnTransformation(frame, viewId, {
        columnId: column.id,
        formula,
        focus,
        orderByColumnId,
      }),
    [openColumnTransformation]
  );

  const requestColumnFill = useCallback(
    (
      frame: FrameObject,
      column: Column,
      formula: string,
      rowIndex?: number,
      viewId?: string
    ) =>
      openColumnTransformation(frame, viewId, {
        columnId: column.id,
        // An empty formula is the doorway from a cell: the editor opens on
        // the column's own name, the way the header gesture does, with the
        // whole column visibly the subject before anything is typed.
        formula: formula || formulaToken(column.name),
        focus: true,
        orderByColumnId: needsDeclaredOrder(formula) ? column.id : undefined,
        // The cell this started from is the formula's anchor, exactly as it
        // is for "Formula here" on a column that does not exist yet. It was
        // dropped here, so a formula begun in an existing column's cell had
        // no anchor at all, and every reference pointed at afterwards lost
        // the `.shift(n)` the clicked row meant.
        anchorRowIndex: rowIndex,
      }, true),
    [openColumnTransformation]
  );

  // Deliberately does not close a context menu: this is also reached
  // straight from the formula bar, where there is none open to close.
  const requestCalculatedColumnEdit = useCallback(
    (frame: FrameObject, column: Column, rowIndex?: number, viewId?: string) => {
      setSelection({ objectId: frame.id, viewId, columnId: column.id });
      setInspectorSection({ panel: "prepare", section: "wrangle" });
      transformColumnToken.current += 1;
      setTransformColumnRequest({
        frameId: frame.id,
        columnId: column.id,
        formula: "",
        focus: true,
        editExisting: true,
        anchorRowIndex: rowIndex,
        token: transformColumnToken.current,
      });
    },
    [setSelection, setInspectorSection]
  );

  return {
    transformColumnRequest,
    requestColumnTransformation,
    requestColumnFill,
    requestCalculatedColumnEdit,
    clearTransformColumnRequest: () => setTransformColumnRequest(null),
    // Exposed raw, in addition to the triggers above, because the keyboard
    // formula gesture (handleGridFormulaKey in GridFormulaKeyboard.ts) mints
    // its own differently-shaped request and needs to manage the token
    // itself rather than go through requestColumnTransformation's signature.
    transformColumnToken,
    setTransformColumnRequest,
  };
}

/**
 * The pipeline-column-editing gestures — add a calculated column,
 * transform or fill one, filter one, hide one, rearrange them — all funnel
 * through the same shape: close whatever context menu was open, select the
 * column being edited, switch the inspector to Wrangle, and hand
 * PipelineEditor a freshly-tokened request. The token is what lets it tell
 * "the user asked again" apart from "this document re-rendered" — a
 * request object's identity alone isn't a safe signal once it has been
 * threaded through props and reshaped along the way.
 */
export function usePipelineColumnRequests({
  setContextMenu,
  setSelection,
  setInspectorSection,
}: {
  setContextMenu: (value: null) => void;
  setSelection: (value: Selection) => void;
  setInspectorSection: (value: "wrangle" | { panel: "prepare"; section: "wrangle" }) => void;
}) {
  const addRequest = useAddCalculatedColumnRequest({
    setContextMenu,
    setSelection,
    setInspectorSection,
  });
  const [filterColumnRequest, setFilterColumnRequest] =
    useState<FilterColumnRequest>(null);
  const filterColumnToken = useRef(0);
  const [hidePipelineColumnRequest, setHidePipelineColumnRequest] =
    useState<HidePipelineColumnRequest>(null);
  const hidePipelineColumnToken = useRef(0);
  const multiColumnRequests = useMultiColumnPipelineRequests({
    setContextMenu,
    setSelection,
    setInspectorSection,
  });
  const transformRequests = useColumnTransformationRequests({
    setContextMenu,
    setSelection,
    setInspectorSection,
  });

  const requestColumnFilter = useCallback(
    (frame: FrameObject, column: Column, viewId?: string) => {
      setContextMenu(null);
      setSelection({ objectId: frame.id, viewId, columnId: column.id });
      setInspectorSection("wrangle");
      filterColumnToken.current += 1;
      setFilterColumnRequest({
        frameId: frame.id,
        columnId: column.id,
        token: filterColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const requestHidePipelineColumn = useCallback(
    (frameId: string, columnId: string, viewId?: string) => {
      setContextMenu(null);
      setSelection({ objectId: frameId, viewId, columnId });
      setInspectorSection("wrangle");
      hidePipelineColumnToken.current += 1;
      setHidePipelineColumnRequest({
        frameId,
        columnId,
        token: hidePipelineColumnToken.current,
      });
    },
    [setContextMenu, setSelection, setInspectorSection]
  );

  const clearFilterColumnRequest = useCallback(() => setFilterColumnRequest(null), []);
  const clearHidePipelineColumnRequest = useCallback(
    () => setHidePipelineColumnRequest(null),
    []
  );

  return {
    ...addRequest,
    ...transformRequests,
    ...multiColumnRequests,
    filterColumnRequest,
    hidePipelineColumnRequest,
    clearFilterColumnRequest,
    clearHidePipelineColumnRequest,
    requestColumnFilter,
    requestHidePipelineColumn,
  };
}

/**
 * One request narrowed to the object it targets, in the shape its editor
 * prop expects: present only while it names this `frameId`, and with the
 * `frameId` itself dropped since the editor already knows which frame it
 * is editing.
 */
export function scopedPipelineRequest<T extends { frameId: string; token: number }>(
  request: T | null,
  frameId: string
): Omit<T, "frameId"> | undefined {
  if (!request || request.frameId !== frameId) return undefined;
  const { frameId: _frameId, ...rest } = request;
  return rest;
}
